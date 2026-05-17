//! Nextcloud Talk `ChannelAdapter` implementation.
//!
//! Wraps the existing webhook-based axum HTTP server inside the `ChannelAdapter`
//! trait.  The axum handler feeds incoming messages into a `tokio::sync::mpsc`
//! channel that is exposed as a `futures::Stream` from `start()`.
//!
//! HMAC-SHA256 signing logic is unchanged — it lives in `signing.rs`.

use assistant_core::clock::{Clock, SystemClock};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::types::channels::NextcloudConfig;
use assistant_core::{
    Attachment, ChannelAdapter, ChannelContent, ChannelMessage, ChannelUser, ToolHandler,
    types::conversation::ChannelType,
};
use assistant_transcription::{TranscriptionProvider, TranscriptionRequest, is_audio_mime};
use async_trait::async_trait;
use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use futures::stream::Stream;
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::config::NextcloudConfigExt;
use super::signing::{sign_request, verify_signature};
use super::types::WebhookEvent;

const SEND_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum audio download size (25 MB).
const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;

/// Nextcloud Talk `ChannelAdapter`.  Runs an axum HTTP server to receive
/// webhook events from Nextcloud and exposes them as a message stream.
pub struct NextcloudAdapter {
    config: NextcloudConfig,
    server_url: String,
    secret: String,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
    /// Stores pending ⏳ message IDs keyed by conversation_token.
    /// Used to remove the hourglass reaction in on_turn_start.
    pending_message_ids: Arc<Mutex<HashMap<String, String>>>,
    /// Optional audio transcription provider for voice messages.
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    /// BCP-47 language hint passed to the transcription provider.
    transcription_language: Option<String>,
}

impl NextcloudAdapter {
    pub fn new(config: NextcloudConfig) -> Result<Self> {
        let server_url = config
            .resolved_server_url()
            .context("Nextcloud server_url is required")?;
        let secret = config
            .resolved_secret()
            .context("Nextcloud secret is required")?;
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        Ok(Self {
            config,
            server_url,
            secret,
            stop_tx,
            stop_rx,
            pending_message_ids: Arc::new(Mutex::new(HashMap::new())),
            transcription: None,
            transcription_language: None,
        })
    }

    /// Attach a transcription provider for inbound voice messages.
    pub fn with_transcription(
        mut self,
        provider: Arc<dyn TranscriptionProvider>,
        language: Option<String>,
    ) -> Self {
        self.transcription = Some(provider);
        self.transcription_language = language;
        self
    }
}

// ── Shared axum state ────────────────────────────────────────────────────────

struct WebhookState {
    secret: String,
    server_url: String,
    allowed_channels: Vec<String>,
    allowed_users: Vec<String>,
    tx: mpsc::Sender<ChannelMessage>,
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    transcription_language: Option<String>,
}

// ── ChannelAdapter impl ───────────────────────────────────────────────────────

#[async_trait]
impl ChannelAdapter for NextcloudAdapter {
    fn name(&self) -> &str {
        "nextcloud"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Nextcloud
    }

    async fn start(&self) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>> {
        let listen_addr = self.config.listen_addr.clone();
        let (tx, rx) = mpsc::channel::<ChannelMessage>(64);

        let state = Arc::new(WebhookState {
            secret: self.secret.clone(),
            server_url: self.server_url.clone(),
            allowed_channels: self.config.allowed_channels.clone(),
            allowed_users: self.config.allowed_users.clone(),
            tx,
            transcription: self.transcription.clone(),
            transcription_language: self.transcription_language.clone(),
        });

        // Bind before spawning so failures propagate as errors from start().
        let listener = tokio::net::TcpListener::bind(&listen_addr)
            .await
            .with_context(|| format!("Nextcloud: failed to bind to {listen_addr}"))?;
        info!(addr = %listen_addr, "Nextcloud: webhook server listening");

        let mut stop_rx = self.stop_rx.clone();

        tokio::spawn(async move {
            let app = Router::new()
                .route("/", post(webhook_handler))
                .route("/webhook", post(webhook_handler))
                .with_state(state);

            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = stop_rx.changed().await;
                })
                .await;
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn send(&self, user: &ChannelUser, content: ChannelContent) -> Result<()> {
        let conversation_token = &user.platform_id;
        match content {
            ChannelContent::Text(text) => {
                http_post_message(
                    &self.server_url,
                    &self.secret,
                    conversation_token,
                    &text,
                    None,
                )
                .await?;
            }
            ChannelContent::FileData {
                filename,
                mime_type,
                ..
            } => {
                // Nextcloud Talk Bot API does not support direct file uploads.
                // Log a warning so operators know the attachment was not delivered.
                warn!(
                    filename = %filename,
                    mime_type = %mime_type,
                    "nextcloud: file/audio sending is not yet supported via the Bot API; skipping"
                );
            }
            _ => {}
        }
        Ok(())
    }

    async fn send_in_thread(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
        thread_id: &str,
    ) -> Result<()> {
        let conversation_token = &user.platform_id;
        match content {
            ChannelContent::Text(text) => {
                let reply_to: Option<i64> = thread_id.parse().ok();
                http_post_message(
                    &self.server_url,
                    &self.secret,
                    conversation_token,
                    &text,
                    reply_to,
                )
                .await?;
            }
            ChannelContent::FileData {
                filename,
                mime_type,
                ..
            } => {
                warn!(
                    filename = %filename,
                    mime_type = %mime_type,
                    "nextcloud: file/audio sending is not yet supported via the Bot API; skipping"
                );
            }
            _ => {}
        }
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let _ = self.stop_tx.send(true);
        Ok(())
    }

    fn platform_tools(&self, msg: &ChannelMessage, _conv_id: Uuid) -> Vec<Arc<dyn ToolHandler>> {
        // Prefer the platform-native conversation_token from metadata (inbound turns).
        // Fall back to sender.platform_id for synthetic messages (scheduler turns).
        let conversation_token = msg
            .metadata
            .get("conversation_token")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| msg.sender.platform_id.clone());
        let message_id = msg.platform_message_id.as_deref().unwrap_or("").to_string();
        super::tools::build_nextcloud_tools(
            &self.server_url,
            &self.secret,
            &conversation_token,
            &message_id,
        )
    }

    /// Add ⏳ hourglass reaction immediately on message receipt (before the conv lock).
    async fn on_message_received(&self, msg: &ChannelMessage) -> Result<()> {
        // Use sender.platform_id as the key — it equals conversation_token for all
        // inbound webhook messages and matches what on_turn_start uses via user.platform_id.
        let conversation_token = msg.sender.platform_id.clone();
        let message_id = match &msg.platform_message_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return Ok(()),
        };
        // Store for removal in on_turn_start.
        self.pending_message_ids
            .lock()
            .await
            .insert(conversation_token.clone(), message_id.clone());
        // Add ⏳ reaction (best-effort; Talk < 17 returns 404, which we ignore).
        if let Err(e) = http_add_reaction(
            &self.server_url,
            &self.secret,
            &conversation_token,
            &message_id,
            "⏳",
        )
        .await
        {
            debug!(error = %e, "nextcloud: failed to add hourglass reaction (best-effort)");
        }
        Ok(())
    }

    /// Remove ⏳, send typing notification when a turn starts.
    async fn on_turn_start(&self, user: &ChannelUser, _conv_id: Uuid) -> Result<()> {
        // Use the same key derivation as on_message_received: prefer platform_id
        // (which equals conversation_token for all inbound webhook messages).
        let conversation_token = &user.platform_id;
        let message_id = self
            .pending_message_ids
            .lock()
            .await
            .remove(conversation_token);
        if let Some(mid) = message_id {
            let _ = http_remove_reaction(
                &self.server_url,
                &self.secret,
                conversation_token,
                &mid,
                "⏳",
            )
            .await;
        }
        let _ = http_send_typing(&self.server_url, &self.secret, conversation_token, true).await;
        Ok(())
    }

    /// On success, clear typing and deliver any file attachments as text notices.
    async fn on_turn_success(
        &self,
        user: &ChannelUser,
        _answer: &str,
        attachments: &[Attachment],
    ) -> Result<()> {
        let conversation_token = &user.platform_id;
        let _ = http_send_typing(&self.server_url, &self.secret, conversation_token, false).await;
        if attachments.is_empty() {
            return Ok(());
        }
        let mut lines = Vec::new();
        for a in attachments {
            let kind = if a.is_image() { "image" } else { "file" };
            lines.push(format!(
                "- **{}** ({}, {} bytes) [{kind}]",
                a.filename,
                a.mime_type,
                a.data.len()
            ));
        }
        let notice = format!(
            "This turn produced {} attachment(s):\n{}",
            attachments.len(),
            lines.join("\n")
        );
        let _ = http_post_message(
            &self.server_url,
            &self.secret,
            conversation_token,
            &notice,
            None,
        )
        .await;
        Ok(())
    }

    async fn on_turn_error(&self, user: &ChannelUser, err: &anyhow::Error) -> Result<()> {
        let conversation_token = &user.platform_id;
        let _ = http_send_typing(&self.server_url, &self.secret, conversation_token, false).await;
        let _ = http_post_message(
            &self.server_url,
            &self.secret,
            conversation_token,
            &format!("Sorry, something went wrong: {err}"),
            None,
        )
        .await;
        Ok(())
    }
}

// ── Webhook handler ───────────────────────────────────────────────────────────

async fn webhook_handler(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let signature = match headers
        .get("x-nextcloud-talk-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_string(),
        None => {
            debug!("Missing X-Nextcloud-Talk-Signature");
            return StatusCode::UNAUTHORIZED;
        }
    };
    let random = match headers
        .get("x-nextcloud-talk-random")
        .and_then(|v| v.to_str().ok())
    {
        Some(r) => r.to_string(),
        None => {
            debug!("Missing X-Nextcloud-Talk-Random");
            return StatusCode::UNAUTHORIZED;
        }
    };

    if !verify_signature(&state.secret, &random, &body, &signature) {
        warn!("Nextcloud: invalid webhook signature");
        return StatusCode::UNAUTHORIZED;
    }

    let event: WebhookEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            warn!(error = %e, "Nextcloud: failed to parse webhook event");
            return StatusCode::BAD_REQUEST;
        }
    };

    if event.event_type != "Create" {
        return StatusCode::OK;
    }

    if event.actor.is_bot() || event.object.name != "message" {
        return StatusCode::OK;
    }

    // Check for voice-message / audio file shares that need transcription.
    let audio_transcript = transcribe_webhook_audio(
        &event,
        &state.server_url,
        &state.secret,
        &state.transcription,
        &state.transcription_language,
    )
    .await;

    let text = match (event.extract_message_text(), audio_transcript) {
        (Some(t), Some(transcript)) if !t.is_empty() => format!("{transcript}\n{t}"),
        (_, Some(transcript)) => transcript,
        (Some(t), None) if !t.is_empty() => t,
        _ => return StatusCode::OK,
    };

    let conversation_token = event.conversation_token().to_string();
    let user_id = event.actor.user_id().to_string();
    let message_id = event.message_id().to_string();

    if !state.allowed_channels.is_empty() && !state.allowed_channels.contains(&conversation_token) {
        debug!(conversation = %conversation_token, "Nextcloud: conversation not in allowlist");
        return StatusCode::OK;
    }
    if !state.allowed_users.is_empty() && !state.allowed_users.contains(&user_id) {
        debug!(user = %user_id, "Nextcloud: user not in allowlist");
        return StatusCode::OK;
    }

    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "conversation_token".to_string(),
        serde_json::Value::String(conversation_token.clone()),
    );
    metadata.insert(
        "message_id".to_string(),
        serde_json::Value::String(message_id.clone()),
    );
    metadata.insert(
        "user_id".to_string(),
        serde_json::Value::String(user_id.clone()),
    );

    let channel_msg = ChannelMessage {
        channel_type: ChannelType::Nextcloud,
        platform_message_id: Some(message_id.clone()),
        sender: ChannelUser {
            platform_id: conversation_token.clone(),
            display_name: Some(user_id),
        },
        content: ChannelContent::Text(text),
        thread_id: Some(message_id),
        timestamp: SystemClock.now(),
        metadata,
    };

    if state.tx.send(channel_msg).await.is_err() {
        warn!("Nextcloud: stream receiver dropped");
    }

    StatusCode::OK
}

// ── HTTP helper ───────────────────────────────────────────────────────────────

/// POST a message to Nextcloud Talk Bot API using HMAC-signed request.
pub(crate) async fn http_post_message(
    server_url: &str,
    secret: &str,
    conversation_token: &str,
    message: &str,
    reply_to: Option<i64>,
) -> Result<()> {
    let mut body = serde_json::json!({ "message": message });
    if let Some(r) = reply_to
        && r > 0
    {
        body["replyTo"] = serde_json::json!(r);
    }
    let body_str = body.to_string();
    let (random, signature) = sign_request(secret, &body_str)?;

    let url = format!(
        "{}/ocs/v2.php/apps/spreed/api/v1/bot/{}/message",
        server_url.trim_end_matches('/'),
        conversation_token
    );

    let client = reqwest::Client::builder()
        .timeout(SEND_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("OCS-APIRequest", "true")
        .header("X-Nextcloud-Talk-Bot-Random", &random)
        .header("X-Nextcloud-Talk-Bot-Signature", &signature)
        .body(body_str)
        .send()
        .await
        .context("Nextcloud post_message HTTP error")?;

    if !resp.status().is_success() {
        anyhow::bail!("Nextcloud post_message failed: HTTP {}", resp.status());
    }
    Ok(())
}

/// Send a typing indicator to Nextcloud Talk (best-effort; Talk < 17 may 404).
pub(crate) async fn http_send_typing(
    server_url: &str,
    secret: &str,
    conversation_token: &str,
    typing: bool,
) -> Result<()> {
    let body_str = serde_json::json!({ "typing": typing }).to_string();
    let (random, signature) = sign_request(secret, &body_str)?;

    let url = format!(
        "{}/ocs/v2.php/apps/spreed/api/v1/bot/{}/typing",
        server_url.trim_end_matches('/'),
        conversation_token
    );

    let client = reqwest::Client::builder()
        .timeout(SEND_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("OCS-APIRequest", "true")
        .header("X-Nextcloud-Talk-Bot-Random", &random)
        .header("X-Nextcloud-Talk-Bot-Signature", &signature)
        .body(body_str)
        .send()
        .await
        .context("Nextcloud send_typing HTTP error")?;

    if !resp.status().is_success() {
        anyhow::bail!("Nextcloud send_typing failed: HTTP {}", resp.status());
    }
    Ok(())
}

/// Add an emoji reaction to a Nextcloud Talk message.
pub(crate) async fn http_add_reaction(
    server_url: &str,
    secret: &str,
    conversation_token: &str,
    message_id: &str,
    reaction: &str,
) -> Result<()> {
    let body_str = serde_json::json!({ "reaction": reaction }).to_string();
    let (random, signature) = sign_request(secret, &body_str)?;

    let url = format!(
        "{}/ocs/v2.php/apps/spreed/api/v1/reaction/{}/{}",
        server_url.trim_end_matches('/'),
        conversation_token,
        message_id
    );

    let client = reqwest::Client::builder()
        .timeout(SEND_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("OCS-APIRequest", "true")
        .header("X-Nextcloud-Talk-Bot-Random", &random)
        .header("X-Nextcloud-Talk-Bot-Signature", &signature)
        .body(body_str)
        .send()
        .await
        .context("Nextcloud add_reaction HTTP error")?;

    if !resp.status().is_success() {
        anyhow::bail!("Nextcloud add_reaction failed: HTTP {}", resp.status());
    }
    Ok(())
}

/// Remove an emoji reaction from a Nextcloud Talk message.
pub(crate) async fn http_remove_reaction(
    server_url: &str,
    secret: &str,
    conversation_token: &str,
    message_id: &str,
    reaction: &str,
) -> Result<()> {
    let body_str = serde_json::json!({ "reaction": reaction }).to_string();
    let (random, signature) = sign_request(secret, &body_str)?;

    let url = format!(
        "{}/ocs/v2.php/apps/spreed/api/v1/reaction/{}/{}",
        server_url.trim_end_matches('/'),
        conversation_token,
        message_id
    );

    let client = reqwest::Client::builder()
        .timeout(SEND_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = client
        .delete(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("OCS-APIRequest", "true")
        .header("X-Nextcloud-Talk-Bot-Random", &random)
        .header("X-Nextcloud-Talk-Bot-Signature", &signature)
        .body(body_str)
        .send()
        .await
        .context("Nextcloud remove_reaction HTTP error")?;

    if !resp.status().is_success() {
        anyhow::bail!("Nextcloud remove_reaction failed: HTTP {}", resp.status());
    }
    Ok(())
}

// ── Audio transcription ──────────────────────────────────────────────────────

/// Inspect a Nextcloud Talk webhook event for voice-message or audio file share
/// parameters.  If an audio attachment is detected and a transcription provider
/// is configured, download the file and return the transcript.
async fn transcribe_webhook_audio(
    event: &WebhookEvent,
    server_url: &str,
    secret: &str,
    transcription: &Option<Arc<dyn TranscriptionProvider>>,
    language: &Option<String>,
) -> Option<String> {
    // Nextcloud Talk encodes rich-object parameters inside the JSON `content`
    // field.  Voice messages appear as a parameter with type "file" and
    // mimetype starting with "audio/".
    let raw_content = event.object.content.as_deref()?;
    let parsed: serde_json::Value = serde_json::from_str(raw_content).ok()?;
    let params = parsed.get("parameters")?.as_object()?;

    // Collect audio file parameters.
    let audio_files: Vec<_> = params
        .values()
        .filter(|p| {
            p.get("type").and_then(|t| t.as_str()) == Some("file")
                && p.get("mimetype")
                    .and_then(|m| m.as_str())
                    .is_some_and(is_audio_mime)
        })
        .collect();

    if audio_files.is_empty() {
        return None;
    }

    let provider = match transcription {
        Some(p) => p,
        None => {
            warn!(
                "nextcloud: received audio file share but no transcription provider configured; dropping"
            );
            return None;
        }
    };

    let mut transcripts = Vec::new();
    for file_param in audio_files {
        let file_id = match file_param.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let mimetype = file_param
            .get("mimetype")
            .and_then(|m| m.as_str())
            .unwrap_or("audio/ogg")
            .to_string();
        let filename = file_param
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        // Download via the Nextcloud Bot API download endpoint.
        let download_url = format!(
            "{}/ocs/v2.php/apps/spreed/api/v1/bot/media/{}",
            server_url.trim_end_matches('/'),
            file_id
        );

        let body_str = String::new();
        let (random, signature) = match super::signing::sign_request(secret, &body_str) {
            Ok(pair) => pair,
            Err(e) => {
                warn!(error = %e, "nextcloud: failed to sign audio download request");
                continue;
            }
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let resp = match client
            .get(&download_url)
            .header("OCS-APIRequest", "true")
            .header("X-Nextcloud-Talk-Bot-Random", &random)
            .header("X-Nextcloud-Talk-Bot-Signature", &signature)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "nextcloud: failed to download audio file");
                continue;
            }
        };

        if !resp.status().is_success() {
            warn!(
                status = %resp.status(),
                file_id = %file_id,
                "nextcloud: audio download returned non-success status"
            );
            continue;
        }

        let audio_data = match resp.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                warn!(error = %e, "nextcloud: failed to read audio response body");
                continue;
            }
        };

        if audio_data.len() > MAX_AUDIO_BYTES {
            warn!(
                size = audio_data.len(),
                limit = MAX_AUDIO_BYTES,
                "nextcloud: audio file too large; skipping"
            );
            continue;
        }

        let request = TranscriptionRequest {
            audio_data,
            mime_type: mimetype,
            filename,
            language: language.clone(),
        };
        match provider.transcribe(request).await {
            Ok(result) => transcripts.push(result.text),
            Err(e) => {
                warn!(error = %e, "nextcloud: audio transcription failed");
            }
        }
    }

    if transcripts.is_empty() {
        return None;
    }

    Some(format!("[Voice message]: {}", transcripts.join(" ")))
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::nextcloud::types::{Actor, EventObject};

    const SECRET: &str = "test-secret";
    const TOKEN: &str = "conversation-token-abc";
    const MSG_ID: &str = "12345";

    fn ocs_ok() -> serde_json::Value {
        serde_json::json!({ "ocs": { "meta": { "status": "ok" }, "data": {} } })
    }

    #[tokio::test]
    async fn send_typing_posts_to_correct_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/ocs/v2.php/apps/spreed/api/v1/bot/{TOKEN}/typing"
            )))
            .and(header("OCS-APIRequest", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ocs_ok()))
            .mount(&server)
            .await;

        http_send_typing(&server.uri(), SECRET, TOKEN, true)
            .await
            .expect("send_typing should succeed");
    }

    #[tokio::test]
    async fn add_reaction_posts_to_correct_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/ocs/v2.php/apps/spreed/api/v1/reaction/{TOKEN}/{MSG_ID}"
            )))
            .and(header("OCS-APIRequest", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ocs_ok()))
            .mount(&server)
            .await;

        http_add_reaction(&server.uri(), SECRET, TOKEN, MSG_ID, "⏳")
            .await
            .expect("add_reaction should succeed");
    }

    #[tokio::test]
    async fn remove_reaction_deletes_correct_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path(format!(
                "/ocs/v2.php/apps/spreed/api/v1/reaction/{TOKEN}/{MSG_ID}"
            )))
            .and(header("OCS-APIRequest", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ocs_ok()))
            .mount(&server)
            .await;

        http_remove_reaction(&server.uri(), SECRET, TOKEN, MSG_ID, "⏳")
            .await
            .expect("remove_reaction should succeed");
    }

    // -- Transcription wiring tests -------------------------------------------

    #[test]
    fn with_transcription_sets_provider() {
        use assistant_transcription::{
            TranscriptionProvider, TranscriptionRequest, TranscriptionResult,
        };

        #[derive(Debug)]
        struct FakeProvider;

        #[async_trait]
        impl TranscriptionProvider for FakeProvider {
            fn name(&self) -> &str {
                "fake"
            }

            async fn transcribe(
                &self,
                _request: TranscriptionRequest,
            ) -> anyhow::Result<TranscriptionResult> {
                Ok(TranscriptionResult {
                    text: "hello".into(),
                    language: None,
                    duration_secs: None,
                })
            }
        }

        let config = NextcloudConfig {
            server_url: Some("http://localhost".to_string()),
            secret: Some("s3cret".to_string()),
            ..Default::default()
        };
        let adapter = NextcloudAdapter::new(config)
            .unwrap()
            .with_transcription(Arc::new(FakeProvider), Some("en".to_string()));

        assert!(adapter.transcription.is_some());
        assert_eq!(adapter.transcription_language.as_deref(), Some("en"));
    }

    #[tokio::test]
    async fn transcribe_webhook_audio_no_provider_returns_none() {
        let event = WebhookEvent {
            event_type: "Create".to_string(),
            actor: Actor {
                actor_type: "Person".to_string(),
                id: "users/alice".to_string(),
                name: "Alice".to_string(),
            },
            object: EventObject {
                object_type: "Note".to_string(),
                id: "42".to_string(),
                name: "message".to_string(),
                content: Some(
                    serde_json::json!({
                        "message": "{file}",
                        "parameters": {
                            "file": {
                                "type": "file",
                                "id": "999",
                                "name": "voice.ogg",
                                "mimetype": "audio/ogg"
                            }
                        }
                    })
                    .to_string(),
                ),
                media_type: None,
            },
            target: None,
            content: None,
        };

        let result =
            transcribe_webhook_audio(&event, "http://localhost", "secret", &None, &None).await;
        assert!(result.is_none(), "should return None without a provider");
    }

    #[tokio::test]
    async fn transcribe_webhook_audio_no_audio_params_returns_none() {
        let event = WebhookEvent {
            event_type: "Create".to_string(),
            actor: Actor {
                actor_type: "Person".to_string(),
                id: "users/bob".to_string(),
                name: "Bob".to_string(),
            },
            object: EventObject {
                object_type: "Note".to_string(),
                id: "43".to_string(),
                name: "message".to_string(),
                content: Some(
                    serde_json::json!({
                        "message": "Hello world",
                        "parameters": {}
                    })
                    .to_string(),
                ),
                media_type: None,
            },
            target: None,
            content: None,
        };

        let result =
            transcribe_webhook_audio(&event, "http://localhost", "secret", &None, &None).await;
        assert!(
            result.is_none(),
            "should return None when no audio parameters exist"
        );
    }
}
