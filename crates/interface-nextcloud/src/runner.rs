//! Nextcloud Talk webhook-based bot interface.
//!
//! Unlike Slack and Mattermost (which use WebSocket connections), the
//! Nextcloud Talk Bot API is webhook-based:
//!
//! 1. The bot registers a webhook URL on the Nextcloud server via
//!    `occ talk:bot:install`.
//! 2. The Nextcloud server POSTs Activity Streams 2.0 events to that URL
//!    whenever a message is sent in a conversation the bot is enabled for.
//! 3. The bot verifies the HMAC-SHA256 signature and processes the event.
//! 4. The bot replies via REST API calls, also signed with the shared secret.
//!
//! This module runs an [`axum`] HTTP server to receive those webhooks.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use assistant_core::{AllowlistFilter, Interface, NextcloudConfig};
use assistant_runtime::{InterfaceRunner, Orchestrator};

use crate::config::NextcloudConfigExt;
use crate::signing::{sign_request, verify_signature};
use crate::tools::build_nextcloud_tools;
use crate::types::WebhookEvent;

/// HTTP timeout for reaction helper requests.
const REACTION_TIMEOUT: Duration = Duration::from_secs(10);

// ── Shared state ─────────────────────────────────────────────────────────────

/// Shared state passed to all axum handlers via `State`.
struct AppState {
    config: NextcloudConfig,
    orchestrator: Arc<Orchestrator>,
    /// Shared secret for HMAC verification and signing.
    secret: String,
    /// Nextcloud server URL (for outgoing API calls).
    server_url: String,
    /// Maps `conversation_token` -> `(conversation_uuid, semaphore)`.
    ///
    /// The semaphore (permits=1) serializes turns within the same
    /// conversation, preventing interleaved `register_extensions()` calls
    /// from clobbering each other's tool sets.
    conversations: Mutex<HashMap<String, (Uuid, Arc<Semaphore>)>>,
    /// HTTP client for reaction helpers (shared, with timeout).
    reaction_client: reqwest::Client,
}

impl AppState {
    /// Look up (or create) the conversation UUID and per-conversation
    /// semaphore for the given conversation token.
    async fn get_or_create_conversation(&self, token: &str) -> (Uuid, Arc<Semaphore>) {
        let mut map = self.conversations.lock().await;
        let entry = map
            .entry(token.to_string())
            .or_insert_with(|| (Uuid::new_v4(), Arc::new(Semaphore::new(1))));
        (entry.0, entry.1.clone())
    }
}

// ── NextcloudInterface ───────────────────────────────────────────────────────

/// The Nextcloud Talk bot interface.
///
/// Runs an HTTP server that receives webhook events from the Nextcloud Talk
/// server and processes them through the orchestrator.
pub struct NextcloudInterface {
    config: NextcloudConfig,
    orchestrator: Arc<Orchestrator>,
    /// Optional external shutdown signal.  When set, the HTTP server shuts
    /// down when this token is cancelled instead of installing its own
    /// process-wide signal handlers.  Used in background mode to avoid
    /// conflicting with the CLI REPL's Ctrl-C handling.
    shutdown: Option<tokio_util::sync::CancellationToken>,
}

impl NextcloudInterface {
    pub fn new(config: NextcloudConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
            shutdown: None,
        }
    }

    /// Attach an external shutdown token.  When cancelled, the HTTP server
    /// shuts down gracefully without installing process-wide signal handlers.
    pub fn with_shutdown(mut self, token: tokio_util::sync::CancellationToken) -> Self {
        self.shutdown = Some(token);
        self
    }

    /// Start the webhook HTTP server and block until shutdown.
    pub async fn run(&self) -> Result<()> {
        let server_url = self
            .config
            .resolved_server_url()
            .context("Nextcloud server_url is required. Set it in [nextcloud] config or NEXTCLOUD_SERVER_URL env var")?;

        let secret = self
            .config
            .resolved_secret()
            .context("Nextcloud secret is required. Set it in [nextcloud] config or NEXTCLOUD_TALK_SECRET env var")?;

        let listen_addr = self.config.listen_addr.clone();

        info!(
            server_url = %server_url,
            listen_addr = %listen_addr,
            "Starting Nextcloud Talk bot"
        );

        // Run BOOT.md startup hook.
        let boot_conversation = Uuid::new_v4();
        match self
            .orchestrator
            .run_boot(boot_conversation, Interface::Nextcloud)
            .await
        {
            Ok(true) => info!("BOOT.md startup hook executed"),
            Ok(false) => {}
            Err(e) => warn!("BOOT.md startup hook failed: {e}"),
        }

        let reaction_client = reqwest::Client::builder()
            .timeout(REACTION_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let state = Arc::new(AppState {
            config: self.config.clone(),
            orchestrator: self.orchestrator.clone(),
            secret,
            server_url,
            conversations: Mutex::new(HashMap::new()),
            reaction_client,
        });

        let app = Router::new()
            .route("/", post(webhook_handler))
            .route("/webhook", post(webhook_handler))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(&listen_addr)
            .await
            .with_context(|| format!("Failed to bind to {listen_addr}"))?;

        info!(listen_addr = %listen_addr, "Nextcloud Talk bot listening for webhooks");

        // Choose shutdown strategy: external token (background mode) or
        // process-wide signals (standalone mode).
        match self.shutdown.clone() {
            Some(token) => {
                axum::serve(listener, app)
                    .with_graceful_shutdown(async move { token.cancelled().await })
                    .await
                    .context("HTTP server error")?;
            }
            None => {
                axum::serve(listener, app)
                    .with_graceful_shutdown(shutdown_signal())
                    .await
                    .context("HTTP server error")?;
            }
        }

        info!("Nextcloud Talk bot shut down");
        Ok(())
    }
}

// ── Webhook handler ──────────────────────────────────────────────────────────

/// Main webhook endpoint.  Receives all events from Nextcloud Talk.
async fn webhook_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    // Extract required headers.
    let signature = match headers
        .get("x-nextcloud-talk-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_string(),
        None => {
            debug!("Missing X-Nextcloud-Talk-Signature header");
            return StatusCode::UNAUTHORIZED;
        }
    };

    let random = match headers
        .get("x-nextcloud-talk-random")
        .and_then(|v| v.to_str().ok())
    {
        Some(r) => r.to_string(),
        None => {
            debug!("Missing X-Nextcloud-Talk-Random header");
            return StatusCode::UNAUTHORIZED;
        }
    };

    // Verify HMAC signature.
    if !verify_signature(&state.secret, &random, &body, &signature) {
        warn!("Invalid webhook signature");
        return StatusCode::UNAUTHORIZED;
    }

    // Parse the event.
    let event: WebhookEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            warn!(error = %e, "Failed to parse webhook event");
            return StatusCode::BAD_REQUEST;
        }
    };

    debug!(
        event_type = %event.event_type,
        actor_id = %event.actor.id,
        "Received Nextcloud Talk webhook"
    );

    // Dispatch based on event type.
    match event.event_type.as_str() {
        "Create" => {
            // Spawn message processing in the background so we can return
            // 200 OK immediately (Nextcloud expects a quick response).
            let state = state.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_message(state, event).await {
                    error!(error = %e, "Failed to handle Nextcloud message");
                }
            });
            StatusCode::OK
        }
        "Join" => {
            info!(
                conversation = %event.conversation_token(),
                "Bot added to conversation"
            );
            StatusCode::OK
        }
        "Leave" => {
            info!(
                conversation = %event.conversation_token(),
                "Bot removed from conversation"
            );
            // Clean up conversation mapping.
            let token = event.conversation_token().to_string();
            state.conversations.lock().await.remove(&token);
            StatusCode::OK
        }
        "Like" => {
            debug!(
                reaction = event.content.as_deref().unwrap_or("?"),
                "Reaction added (ignored)"
            );
            StatusCode::OK
        }
        "Undo" => {
            debug!("Reaction removed (ignored)");
            StatusCode::OK
        }
        other => {
            debug!(event_type = %other, "Unknown event type (ignored)");
            StatusCode::OK
        }
    }
}

/// Process an incoming chat message through the orchestrator.
async fn handle_message(state: Arc<AppState>, event: WebhookEvent) -> Result<()> {
    // Ignore messages from bots (prevents loops).
    if event.actor.is_bot() {
        debug!("Ignoring message from bot: {}", event.actor.id);
        return Ok(());
    }

    // Ignore system messages.
    if event.object.name != "message" {
        debug!(
            name = %event.object.name,
            "Ignoring non-message object"
        );
        return Ok(());
    }

    // Extract the message text.
    let text = match event.extract_message_text() {
        Some(t) if !t.is_empty() => t,
        _ => {
            debug!("Empty or unparseable message, ignoring");
            return Ok(());
        }
    };

    let conversation_token = event.conversation_token().to_string();
    let user_id = event.actor.user_id().to_string();
    let message_id = event.message_id().to_string();

    // Allowlist checks.
    if !AllowlistFilter::new(state.config.allowed_channels.clone()).is_allowed(&conversation_token)
    {
        debug!(
            conversation = %conversation_token,
            "Conversation not in allowed_channels, ignoring"
        );
        return Ok(());
    }

    if !AllowlistFilter::new(state.config.allowed_users.clone()).is_allowed(&user_id) {
        debug!(user = %user_id, "User not in allowed_users, ignoring");
        return Ok(());
    }

    // Get (or create) the conversation UUID and per-conversation semaphore.
    let (conversation_id, semaphore) = state.get_or_create_conversation(&conversation_token).await;

    debug!(
        conversation_id = %conversation_id,
        conversation_token = %conversation_token,
        user = %user_id,
        message_id = %message_id,
        "Processing Nextcloud message"
    );

    // Add a "thinking" reaction to acknowledge receipt (fire-and-forget).
    let think_handle = {
        let client = state.reaction_client.clone();
        let server_url = state.server_url.clone();
        let secret = state.secret.clone();
        let token = conversation_token.clone();
        let mid = message_id.clone();
        tokio::spawn(async move {
            let _ = add_reaction(&client, &server_url, &secret, &token, &mid, "\u{23F3}").await;
        })
    };

    // Acquire the per-conversation lock so that concurrent webhooks for the
    // same room are serialised.  This prevents `register_extensions()` from
    // being overwritten by a later message before the worker claims the turn.
    let _permit = semaphore.acquire().await;

    // Build per-turn extension tools.
    let tools = build_nextcloud_tools(
        &state.server_url,
        &state.secret,
        &conversation_token,
        &message_id,
    );

    let result = state
        .orchestrator
        .run_turn_with_tools(
            &text,
            conversation_id,
            Interface::Nextcloud,
            tools,
            None,
            vec![],
        )
        .await;

    // Remove the thinking reaction (fire-and-forget).
    // Wait for the add to finish first so there's something to remove.
    let _ = think_handle.await;
    {
        let client = state.reaction_client.clone();
        let server_url = state.server_url.clone();
        let secret = state.secret.clone();
        let token = conversation_token.clone();
        let mid = message_id.clone();
        tokio::spawn(async move {
            let _ = remove_reaction(&client, &server_url, &secret, &token, &mid, "\u{23F3}").await;
        });
    }

    match result {
        Ok(turn_result) => {
            debug!(
                conversation_id = %conversation_id,
                "Turn completed successfully"
            );

            // Deliver any file attachments produced by tools.
            if !turn_result.attachments.is_empty() {
                deliver_attachments(
                    &state.server_url,
                    &state.secret,
                    &conversation_token,
                    &message_id,
                    &turn_result.attachments,
                )
                .await;
            }
        }
        Err(e) => {
            error!(error = %e, "Orchestrator turn failed");
            // Post an error message so the user knows something went wrong.
            let error_tools = build_nextcloud_tools(
                &state.server_url,
                &state.secret,
                &conversation_token,
                &message_id,
            );
            if let Some(reply_tool) = error_tools.into_iter().next() {
                let mut params = HashMap::new();
                params.insert(
                    "message".to_string(),
                    serde_json::json!("Sorry, something went wrong while processing your message."),
                );
                let ctx = assistant_core::ExecutionContext {
                    conversation_id,
                    agent_id: "default".to_string(),
                    turn: 0,
                    interface: Interface::Nextcloud,
                    interactive: false,
                    allowed_tools: None,
                    depth: 0,
                };
                let _ = reply_tool.run(params, &ctx).await;
            }
        }
    }

    Ok(())
}

// ── Attachment delivery ──────────────────────────────────────────────────────

/// Post a text notice about each attachment produced during the turn.
///
/// Nextcloud Talk's bot API does not support file uploads, so we send a
/// text message listing the attachments and their sizes.  This ensures the
/// user knows that results were produced even though we cannot deliver the
/// raw files.
async fn deliver_attachments(
    server_url: &str,
    secret: &str,
    conversation_token: &str,
    reply_to_id: &str,
    attachments: &[assistant_core::Attachment],
) {
    let mut lines = Vec::new();
    for a in attachments {
        let kind = if a.is_image() { "image" } else { "file" };
        let size = a.data.len();
        lines.push(format!(
            "- **{}** ({}, {} bytes) [{kind}]",
            a.filename, a.mime_type, size
        ));
    }

    let message = format!(
        "This turn produced {} attachment(s):\n{}",
        attachments.len(),
        lines.join("\n")
    );

    let reply_to: i64 = reply_to_id.parse().unwrap_or(0);
    let mut body = serde_json::json!({ "message": message });
    if reply_to > 0 {
        body["replyTo"] = serde_json::json!(reply_to);
    }

    let body_str = body.to_string();
    let Ok((random, signature)) = sign_request(secret, &body_str) else {
        warn!("Failed to sign attachment notice");
        return;
    };

    let url = format!(
        "{}/ocs/v2.php/apps/spreed/api/v1/bot/{}/message",
        server_url.trim_end_matches('/'),
        conversation_token
    );

    let client = reqwest::Client::builder()
        .timeout(REACTION_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    match client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("OCS-APIRequest", "true")
        .header("X-Nextcloud-Talk-Bot-Random", &random)
        .header("X-Nextcloud-Talk-Bot-Signature", &signature)
        .body(body_str)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            debug!("Attachment notice posted");
        }
        Ok(r) => {
            warn!(
                status = %r.status(),
                "Failed to post attachment notice"
            );
        }
        Err(e) => {
            warn!(error = %e, "Failed to post attachment notice");
        }
    }
}

// ── Reaction helpers ─────────────────────────────────────────────────────────

/// Add a reaction to a message via the Nextcloud Talk Bot API.
async fn add_reaction(
    client: &reqwest::Client,
    server_url: &str,
    secret: &str,
    conversation_token: &str,
    message_id: &str,
    reaction: &str,
) -> Result<()> {
    let body = serde_json::json!({ "reaction": reaction });
    let body_str = body.to_string();
    let (random, signature) = sign_request(secret, &body_str)?;

    let url = format!(
        "{}/ocs/v2.php/apps/spreed/api/v1/bot/{}/reaction/{}",
        server_url.trim_end_matches('/'),
        conversation_token,
        message_id
    );

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("OCS-APIRequest", "true")
        .header("X-Nextcloud-Talk-Bot-Random", &random)
        .header("X-Nextcloud-Talk-Bot-Signature", &signature)
        .body(body_str)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Failed to add reaction: HTTP {}", resp.status());
    }

    Ok(())
}

/// Remove a reaction from a message via the Nextcloud Talk Bot API.
async fn remove_reaction(
    client: &reqwest::Client,
    server_url: &str,
    secret: &str,
    conversation_token: &str,
    message_id: &str,
    reaction: &str,
) -> Result<()> {
    let body = serde_json::json!({ "reaction": reaction });
    let body_str = body.to_string();
    let (random, signature) = sign_request(secret, &body_str)?;

    let url = format!(
        "{}/ocs/v2.php/apps/spreed/api/v1/bot/{}/reaction/{}",
        server_url.trim_end_matches('/'),
        conversation_token,
        message_id
    );

    let resp = client
        .delete(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("OCS-APIRequest", "true")
        .header("X-Nextcloud-Talk-Bot-Random", &random)
        .header("X-Nextcloud-Talk-Bot-Signature", &signature)
        .body(body_str)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Failed to remove reaction: HTTP {}", resp.status());
    }

    Ok(())
}

// ── Graceful shutdown ────────────────────────────────────────────────────────

/// Process-wide signal handler for standalone mode only.
///
/// In background mode, the caller passes a [`CancellationToken`] instead
/// so that the REPL's Ctrl-C handling is not affected.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => info!("Received Ctrl+C, shutting down"),
        () = terminate => info!("Received SIGTERM, shutting down"),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_conversation_mapping() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let conversations = Mutex::new(HashMap::new());

            // First access creates a new entry.
            let id1 = {
                let mut map = conversations.lock().await;
                let entry = map
                    .entry("room1".to_string())
                    .or_insert_with(|| (Uuid::new_v4(), Arc::new(Semaphore::new(1))));
                entry.0
            };

            // Second access returns the same UUID.
            let id2 = {
                let mut map = conversations.lock().await;
                let entry = map
                    .entry("room1".to_string())
                    .or_insert_with(|| (Uuid::new_v4(), Arc::new(Semaphore::new(1))));
                entry.0
            };

            assert_eq!(id1, id2);

            // Different room gets a different UUID.
            let id3 = {
                let mut map = conversations.lock().await;
                let entry = map
                    .entry("room2".to_string())
                    .or_insert_with(|| (Uuid::new_v4(), Arc::new(Semaphore::new(1))));
                entry.0
            };

            assert_ne!(id1, id3);
        });
    }
}

// ── InterfaceRunner impl ──────────────────────────────────────────────────────

#[async_trait::async_trait]
impl InterfaceRunner for NextcloudInterface {
    async fn run(&self) -> Result<()> {
        self.run().await
    }
}
