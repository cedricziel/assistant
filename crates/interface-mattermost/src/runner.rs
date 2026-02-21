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
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::{MattermostConfig, MattermostConfigExt};

// ── WebSocket handler ─────────────────────────────────────────────────────────

/// Implements [`WebsocketHandler`] and holds all state needed to dispatch
/// incoming Mattermost events to the orchestrator and post replies back.
struct MattermostHandler {
    config: MattermostConfig,
    orchestrator: Arc<Orchestrator>,
    /// Cloned Mattermost client used for posting replies.
    api: Mattermost,
    /// One conversation UUID per (channel_id, user_id) pair.
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
            Err(_) => return,
        };

        let channel_id = message.broadcast.channel_id.clone();
        let user_id = post.user_id.clone();
        let text = post.message.clone();

        if text.is_empty() {
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
            text_len = text.len(),
            "Dispatching to orchestrator"
        );

        let conversation_id = {
            let mut map = self.conversations.lock().await;
            *map.entry((channel_id.clone(), user_id.clone()))
                .or_insert_with(Uuid::new_v4)
        };

        let (tok_tx, mut tok_rx) = tokio::sync::mpsc::channel::<String>(64);
        let collector = tokio::spawn(async move {
            let mut buf = String::new();
            while let Some(tok) = tok_rx.recv().await {
                buf.push_str(&tok);
            }
            buf
        });

        let turn_result = self
            .orchestrator
            .run_turn_streaming(&text, conversation_id, Interface::Mattermost, tok_tx)
            .await;

        let reply = collector.await.unwrap_or_default();

        if let Err(e) = turn_result {
            tracing::error!(error = %e, "Orchestrator error");
            return;
        }

        if reply.is_empty() {
            return;
        }

        // Post reply to the same channel.
        let body = mattermost_api::models::PostBody {
            channel_id: channel_id.clone(),
            message: reply,
            root_id: None,
        };
        if let Err(e) = self.api.create_post(&body).await {
            tracing::error!(error = %e, "Failed to post Mattermost reply");
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

    /// Start the Mattermost WebSocket listener loop.
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

        info!(server = %server_url, "Connecting to Mattermost");

        let auth = AuthenticationData::from_access_token(token);
        let mut api = Mattermost::new(&server_url, auth)
            .map_err(|e| anyhow::anyhow!("Failed to create Mattermost client: {e}"))?;

        // For token auth this is a no-op; for password auth it fetches a session token.
        api.store_session_token()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to authenticate with Mattermost: {e}"))?;

        let handler = MattermostHandler {
            config: self.config.clone(),
            orchestrator: self.orchestrator.clone(),
            api: api.clone(),
            conversations: Arc::new(Mutex::new(HashMap::new())),
        };

        api.connect_to_websocket(handler)
            .await
            .map_err(|e| anyhow::anyhow!("Mattermost WebSocket error: {e}"))?;

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
