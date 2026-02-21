//! Mattermost interface runner.
//!
//! Authenticates with the Mattermost REST API and opens a WebSocket event
//! stream.  Each incoming `posted` event in an allowed channel is dispatched
//! to the [`Orchestrator`] and the reply is posted back via the REST API.
//!
//! # API notes
//!
//! `mattermost_api` uses a **trait-based** WebSocket handler
//! ([`WebsocketHandler`]).  The handler struct holds `Arc`-wrapped shared
//! state so it is `Send + Sync` and can be passed to `connect_to_websocket`.
//!
//! # Safety
//!
//! [`SafetyGate`][assistant_runtime::safety::SafetyGate] blocks `shell-exec`
//! for [`Interface::Mattermost`].  Additionally, `allowed_channels` and
//! `allowed_users` allowlists are checked before dispatching.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::Interface;
use assistant_runtime::Orchestrator;
use async_trait::async_trait;
use mattermost_api::prelude::*;
use mattermost_api::socket::WebsocketEventType;
use serde::Deserialize;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Short preview of a string for log output.
fn preview(s: &str, max: usize) -> &str {
    let end = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
    &s[..end]
}

use crate::config::{MattermostConfig, MattermostConfigExt};
use crate::tools::build_mattermost_tools;

/// Minimal response for `GET /users/me` — we only need the user ID.
#[derive(Debug, Deserialize)]
struct MeUser {
    id: String,
}

// ── WebSocket handler ─────────────────────────────────────────────────────────

/// Implements [`WebsocketHandler`] and holds all state needed to dispatch
/// incoming Mattermost events to the orchestrator and post replies back.
struct MattermostHandler {
    config: MattermostConfig,
    orchestrator: Arc<Orchestrator>,
    /// Shared Mattermost client used for posting replies and reactions.
    api: Arc<Mattermost>,
    /// The bot's own Mattermost user ID — required for posting reactions.
    bot_user_id: String,
    /// One conversation UUID per (channel_id, root_post_id) pair.
    conversations: Arc<Mutex<HashMap<(String, String), Uuid>>>,
}

#[async_trait]
impl WebsocketHandler for MattermostHandler {
    async fn callback(&self, message: WebsocketEvent) {
        // Only handle `posted` events.
        if message.event != WebsocketEventType::Posted {
            return;
        }

        // The post payload is a JSON string nested inside the event data.
        let post_json = match message.data.get("post").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return,
        };
        let post: mattermost_api::models::Post = match serde_json::from_str(post_json) {
            Ok(p) => p,
            Err(e) => {
                warn!(error = %e, "Failed to deserialize Mattermost post payload");
                return;
            }
        };

        let channel_id = message.broadcast.channel_id.clone();
        let user_id = post.user_id.clone();
        let post_id = post.id.clone();
        let text = post.message.clone();

        // Determine the thread root for replies.
        // If the triggering post is itself a reply, keep using its root_id.
        // Otherwise create a new thread rooted at this post.
        let reply_root_id = if post.root_id.is_empty() {
            Some(post_id.clone())
        } else {
            Some(post.root_id.clone())
        };

        if text.is_empty() {
            return;
        }

        // Ignore the bot's own messages to prevent infinite reply loops.
        if !self.bot_user_id.is_empty() && user_id == self.bot_user_id {
            debug!(user_id = %user_id, "Ignoring message from self");
            return;
        }

        // Channel allowlist check.
        if !self.config.allowed_channels.is_empty()
            && !self.config.allowed_channels.contains(&channel_id)
        {
            warn!(channel = %channel_id, "Ignoring message from non-allowlisted channel");
            return;
        }

        // User allowlist check.
        if !self.config.allowed_users.is_empty() && !self.config.allowed_users.contains(&user_id) {
            warn!(user = %user_id, "Ignoring message from non-allowlisted user");
            return;
        }

        info!(
            channel = %channel_id,
            user = %user_id,
            post_id = %post_id,
            text_len = text.len(),
            text_preview = preview(&text, 120),
            "Incoming message"
        );

        // Key conversations by (channel_id, root_post_id) so every message in
        // the same thread shares a single LLM conversation context.
        let thread_key = reply_root_id.clone().unwrap_or_else(|| post_id.clone());
        let conversation_id = {
            let mut map = self.conversations.lock().await;
            *map.entry((channel_id.clone(), thread_key))
                .or_insert_with(Uuid::new_v4)
        };

        // Build per-turn Mattermost extension tools.
        let reply_root_id_for_err = reply_root_id.clone();
        let extensions = build_mattermost_tools(
            channel_id.clone(),
            post_id,
            reply_root_id,
            self.bot_user_id.clone(),
            self.api.clone(),
        );

        let orchestrator_start = std::time::Instant::now();
        let turn_result = self
            .orchestrator
            .run_turn_with_tools(&text, conversation_id, Interface::Mattermost, extensions)
            .await;
        let elapsed_ms = orchestrator_start.elapsed().as_millis();

        if let Err(e) = turn_result {
            tracing::error!(error = %e, elapsed_ms, "Orchestrator error");
            // Notify the user so they aren't left waiting silently.
            let err_body = mattermost_api::models::PostBody {
                channel_id: channel_id.clone(),
                message: "Sorry, something went wrong processing your message.".to_string(),
                root_id: reply_root_id_for_err,
            };
            if let Err(post_err) = self.api.create_post(&err_body).await {
                warn!(error = %post_err, "Failed to post error feedback to user");
            }
        } else {
            info!(
                channel = %channel_id,
                elapsed_ms,
                "orchestrator.run_turn_with_tools ← ok"
            );
        }
    }
}

// ── MattermostInterface ───────────────────────────────────────────────────────

/// The Mattermost interface handle.
pub struct MattermostInterface {
    config: MattermostConfig,
    orchestrator: Arc<Orchestrator>,
}

impl MattermostInterface {
    pub fn new(config: MattermostConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
    }

    /// Start the Mattermost WebSocket listener loop, reconnecting on disconnect.
    ///
    /// Exits cleanly on SIGINT (Ctrl+C) or SIGTERM.
    pub async fn run(&self) -> Result<()> {
        let server_url = self.config.resolved_server_url().ok_or_else(|| {
            anyhow::anyhow!(
                "No Mattermost server URL configured. Set server_url in [mattermost] config \
                 or the MATTERMOST_SERVER_URL environment variable."
            )
        })?;

        let token = self.config.resolved_token().ok_or_else(|| {
            anyhow::anyhow!(
                "No Mattermost token configured. Set token in [mattermost] config \
                 or the MATTERMOST_TOKEN environment variable."
            )
        })?;

        // Build the client once; connect_to_websocket takes &mut self so the
        // same api instance is reused across reconnects.
        let auth = AuthenticationData::from_access_token(token);
        let mut api = Mattermost::new(&server_url, auth)
            .map_err(|e| anyhow::anyhow!("Failed to create Mattermost client: {e}"))?;

        // For token auth this is a no-op; for password auth it fetches a session token.
        api.store_session_token()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to authenticate with Mattermost: {e}"))?;

        // Fetch the bot's own user ID — required for posting reactions.
        let bot_user_id: String = match api.query::<MeUser>("GET", "users/me", None, None).await {
            Ok(me) => {
                info!(user_id = %me.id, "Fetched bot user ID");
                me.id
            }
            Err(e) => {
                anyhow::bail!(
                    "Failed to fetch bot user ID (required for self-message filtering): {e}"
                );
            }
        };

        // Wrap in Arc so handlers can share the same client without cloning it.
        let api = Arc::new(api);

        // Conversation map persists across reconnects.
        let conversations: Arc<Mutex<HashMap<(String, String), Uuid>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // ── Graceful shutdown ─────────────────────────────────────────────────
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = sigterm.recv() => {}
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
            info!("Shutdown signal received, stopping…");
            let _ = shutdown_tx.send(true);
        });

        let mut backoff = std::time::Duration::from_secs(1);

        loop {
            if *shutdown_rx.borrow() {
                break;
            }

            info!(server = %server_url, "Connecting to Mattermost WebSocket");

            let handler = MattermostHandler {
                config: self.config.clone(),
                orchestrator: self.orchestrator.clone(),
                api: api.clone(),
                bot_user_id: bot_user_id.clone(),
                conversations: conversations.clone(),
            };

            // connect_to_websocket requires &mut self.  Clone the inner client
            // so the WS session gets its own mutable copy while REST calls
            // continue through the shared Arc<Mattermost> in the handler.
            let mut ws_api = (*api).clone();
            let clean_disconnect = tokio::select! {
                result = ws_api.connect_to_websocket(handler) => {
                    match result {
                        Ok(()) => {
                            info!("Mattermost WebSocket closed, reconnecting…");
                            true
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                delay_secs = backoff.as_secs(),
                                "Mattermost connection error, retrying"
                            );
                            false
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    info!("Shutdown during WebSocket, exiting");
                    return Ok(());
                }
            };

            // Reset backoff after a clean connection so transient failures don't
            // permanently slow down reconnects after recovery.
            if clean_disconnect {
                backoff = std::time::Duration::from_secs(1);
            } else {
                backoff = (backoff * 2).min(std::time::Duration::from_secs(60));
            }

            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                _ = shutdown_rx.changed() => {
                    info!("Shutdown during backoff, exiting");
                    return Ok(());
                }
            }
        }

        info!("Mattermost interface stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::MattermostConfig;

    #[test]
    fn allowlist_channel_logic_empty_accepts_all() {
        let cfg = MattermostConfig {
            allowed_channels: vec![],
            ..Default::default()
        };
        let channel = "town-square".to_string();
        let blocked = !cfg.allowed_channels.is_empty() && !cfg.allowed_channels.contains(&channel);
        assert!(!blocked);
    }

    #[test]
    fn allowlist_channel_logic_non_empty_blocks_unknown() {
        let cfg = MattermostConfig {
            allowed_channels: vec!["bot-test".to_string()],
            ..Default::default()
        };
        let unknown = "town-square".to_string();
        let blocked = !cfg.allowed_channels.is_empty() && !cfg.allowed_channels.contains(&unknown);
        assert!(blocked);
    }

    #[test]
    fn allowlist_user_logic_non_empty_passes_known() {
        let cfg = MattermostConfig {
            allowed_users: vec!["alice".to_string()],
            ..Default::default()
        };
        let known = "alice".to_string();
        let blocked = !cfg.allowed_users.is_empty() && !cfg.allowed_users.contains(&known);
        assert!(!blocked);
    }
}
