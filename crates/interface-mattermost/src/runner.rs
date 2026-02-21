//! Mattermost interface runner.
//!
//! Without `--features mattermost` this provides a no-op
//! [`MattermostInterface`] that returns an informative error when started.
//!
//! With `--features mattermost` the runner authenticates with the Mattermost
//! REST API and opens a WebSocket event stream.  Each incoming `posted` event
//! in an allowed channel is dispatched to the [`Orchestrator`] and the reply
//! is posted back via the REST API.
//!
//! # Safety
//!
//! [`SafetyGate`][assistant_runtime::safety::SafetyGate] already blocks
//! `shell-exec` when the interface is [`Interface::Mattermost`].
//! Additionally, `MattermostConfig::allowed_channels` and
//! `MattermostConfig::allowed_users` are checked before dispatching.

use std::sync::Arc;

use anyhow::Result;
use assistant_runtime::Orchestrator;

use crate::config::MattermostConfig;
#[cfg(feature = "mattermost")]
use crate::config::MattermostConfigExt;

/// The Mattermost interface handle.
pub struct MattermostInterface {
    #[allow(dead_code)]
    config: MattermostConfig,
    #[allow(dead_code)]
    orchestrator: Arc<Orchestrator>,
}

impl MattermostInterface {
    /// Create a new [`MattermostInterface`].
    ///
    /// Call [`run`][Self::run] to start the WebSocket listener loop (requires
    /// `--features mattermost`).
    pub fn new(config: MattermostConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
    }

    /// Start the Mattermost WebSocket listener loop.
    ///
    /// Without `--features mattermost` this always returns an error explaining
    /// how to enable the feature.
    pub async fn run(&self) -> Result<()> {
        #[cfg(not(feature = "mattermost"))]
        {
            anyhow::bail!(
                "The Mattermost interface requires recompiling with `--features mattermost`.\n\
                 Rebuild with:\n\
                 \n\
                 cargo build -p assistant-interface-mattermost --features mattermost\n\
                 \n\
                 You also need a Mattermost personal access token and server URL\n\
                 configured in ~/.assistant/config.toml under [mattermost]."
            );
        }

        #[cfg(feature = "mattermost")]
        self.run_websocket_loop().await
    }

    /// The mattermost_api WebSocket event loop (only compiled with
    /// `--features mattermost`).
    #[cfg(feature = "mattermost")]
    async fn run_websocket_loop(&self) -> Result<()> {
        use std::collections::HashMap;

        use assistant_core::Interface;
        use mattermost_api::prelude::*;
        use tracing::{info, warn};
        use uuid::Uuid;

        let server_url = self.config.resolved_server_url().ok_or_else(|| {
            anyhow::anyhow!(
                "No Mattermost server URL configured (set server_url in [mattermost] config \
                 or MATTERMOST_SERVER_URL env var)"
            )
        })?;

        let token = self.config.resolved_token().ok_or_else(|| {
            anyhow::anyhow!(
                "No Mattermost token configured (set token in [mattermost] config \
                 or MATTERMOST_TOKEN env var)"
            )
        })?;

        info!(server = %server_url, "Connecting to Mattermost");

        let api = ApiClient::new(&server_url, &token)
            .map_err(|e| anyhow::anyhow!("Failed to create Mattermost API client: {e}"))?;

        // Track one conversation_id per (channel_id, user_id) pair.
        let mut conversations: HashMap<(String, String), Uuid> = HashMap::new();

        let config = &self.config;

        api.listen_for_events(|event: ApiEvent| {
            async move {
                // We only handle `posted` events.
                if event.event != "posted" {
                    return;
                }

                let post: Post = match serde_json::from_str(
                    event.data.get("post").and_then(|v| v.as_str()).unwrap_or(""),
                ) {
                    Ok(p) => p,
                    Err(_) => return,
                };

                let channel_id = post.channel_id.clone().unwrap_or_default();
                let user_id = post.user_id.clone().unwrap_or_default();
                let text = post.message.clone().unwrap_or_default();

                if text.is_empty() {
                    return;
                }

                // Channel allowlist check.
                if !config.allowed_channels.is_empty()
                    && !config.allowed_channels.contains(&channel_id)
                {
                    warn!(channel = %channel_id, "Ignoring message from non-allowlisted channel");
                    return;
                }

                // User allowlist check.
                if !config.allowed_users.is_empty()
                    && !config.allowed_users.contains(&user_id)
                {
                    warn!(user = %user_id, "Ignoring message from non-allowlisted user");
                    return;
                }

                info!(channel = %channel_id, user = %user_id, text_len = text.len(), "Dispatching to orchestrator");

                let conversation_id = *conversations
                    .entry((channel_id.clone(), user_id.clone()))
                    .or_insert_with(Uuid::new_v4);

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
                let create_post = CreatePost {
                    channel_id: channel_id.clone(),
                    message: reply,
                    ..Default::default()
                };
                if let Err(e) = api.create_post(&create_post).await {
                    tracing::error!(error = %e, "Failed to post Mattermost reply");
                }
            }
        })
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
