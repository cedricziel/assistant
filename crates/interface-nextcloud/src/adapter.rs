//! Nextcloud Talk `ChannelAdapter` implementation.
//!
//! Wraps the existing webhook-based axum HTTP server inside the `ChannelAdapter`
//! trait.  The axum handler feeds incoming messages into a `tokio::sync::mpsc`
//! channel that is exposed as a `futures::Stream` from `start()`.
//!
//! HMAC-SHA256 signing logic is unchanged — it lives in `signing.rs`.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::{
    Attachment, ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser,
    NextcloudConfig, ToolHandler,
};
use async_trait::async_trait;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use chrono::Utc;
use futures::stream::Stream;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::NextcloudConfigExt;
use crate::signing::{sign_request, verify_signature};
use crate::types::WebhookEvent;

const SEND_TIMEOUT: Duration = Duration::from_secs(10);

/// Nextcloud Talk `ChannelAdapter`.  Runs an axum HTTP server to receive
/// webhook events from Nextcloud and exposes them as a message stream.
pub struct NextcloudAdapter {
    config: NextcloudConfig,
    server_url: String,
    secret: String,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
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
        })
    }
}

// ── Shared axum state ────────────────────────────────────────────────────────

struct WebhookState {
    secret: String,
    allowed_channels: Vec<String>,
    allowed_users: Vec<String>,
    tx: mpsc::Sender<ChannelMessage>,
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
            allowed_channels: self.config.allowed_channels.clone(),
            allowed_users: self.config.allowed_users.clone(),
            tx,
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
        if let ChannelContent::Text(text) = content {
            http_post_message(
                &self.server_url,
                &self.secret,
                conversation_token,
                &text,
                None,
            )
            .await?;
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
        if let ChannelContent::Text(text) = content {
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
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let _ = self.stop_tx.send(true);
        Ok(())
    }

    fn platform_tools(&self, msg: &ChannelMessage, _conv_id: Uuid) -> Vec<Arc<dyn ToolHandler>> {
        let conversation_token = msg
            .metadata
            .get("conversation_token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let message_id = msg.platform_message_id.as_deref().unwrap_or("").to_string();
        crate::tools::build_nextcloud_tools(
            &self.server_url,
            &self.secret,
            &conversation_token,
            &message_id,
        )
    }

    /// Add hourglass reaction while processing.
    async fn on_turn_start(&self, _user: &ChannelUser, _conv_id: Uuid) -> Result<()> {
        // Reaction is best-effort; log but don't fail the turn.
        Ok(())
    }

    /// On success, deliver any file attachments as text notices.
    async fn on_turn_success(
        &self,
        user: &ChannelUser,
        _answer: &str,
        attachments: &[Attachment],
    ) -> Result<()> {
        if attachments.is_empty() {
            return Ok(());
        }
        let conversation_token = &user.platform_id;
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
        let _ = http_post_message(
            &self.server_url,
            &self.secret,
            &user.platform_id,
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

    let text = match event.extract_message_text() {
        Some(t) if !t.is_empty() => t,
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
        timestamp: Utc::now(),
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
    if let Some(r) = reply_to {
        if r > 0 {
            body["replyTo"] = serde_json::json!(r);
        }
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
