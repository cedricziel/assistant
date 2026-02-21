//! Slack interface runner.
//!
//! Without `--features slack` this provides a no-op [`SlackInterface`] that
//! returns an informative error when started.
//!
//! With `--features slack` the runner opens a Socket Mode WebSocket connection
//! using `slack-morphism`.  Each incoming `AppMention` or direct `Message`
//! event is dispatched to the [`Orchestrator`] and the reply is posted back
//! to the channel via the Slack Web API.
//!
//! # Safety
//!
//! [`SafetyGate`][assistant_runtime::safety::SafetyGate] already blocks
//! `shell-exec` when the interface is [`Interface::Slack`].  Additionally,
//! `SlackConfig::allowed_channels` and `SlackConfig::allowed_users` are
//! checked before dispatching.

use std::sync::Arc;

use anyhow::Result;
use assistant_runtime::Orchestrator;

use crate::config::SlackConfig;
#[cfg(feature = "slack")]
use crate::config::SlackConfigExt;

/// The Slack interface handle.
pub struct SlackInterface {
    #[allow(dead_code)]
    config: SlackConfig,
    #[allow(dead_code)]
    orchestrator: Arc<Orchestrator>,
}

impl SlackInterface {
    /// Create a new [`SlackInterface`].
    ///
    /// Call [`run`][Self::run] to start the Socket Mode listener loop
    /// (requires `--features slack`).
    pub fn new(config: SlackConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
    }

    /// Start the Slack Socket Mode listener loop.
    ///
    /// Without `--features slack` this always returns an error explaining how
    /// to enable the feature.
    pub async fn run(&self) -> Result<()> {
        #[cfg(not(feature = "slack"))]
        {
            anyhow::bail!(
                "The Slack interface requires recompiling with `--features slack`.\n\
                 Rebuild with:\n\
                 \n\
                 cargo build -p assistant-interface-slack --features slack\n\
                 \n\
                 You also need a Slack app with Socket Mode enabled, a bot token\n\
                 (xoxb-...) and an app-level token (xapp-...) configured in\n\
                 ~/.assistant/config.toml under [slack]."
            );
        }

        #[cfg(feature = "slack")]
        self.run_socket_mode_loop().await
    }

    /// The slack-morphism Socket Mode event loop (only compiled with
    /// `--features slack`).
    #[cfg(feature = "slack")]
    async fn run_socket_mode_loop(&self) -> Result<()> {
        use std::collections::HashMap;

        use assistant_core::Interface;
        use slack_morphism::prelude::*;
        use tracing::{debug, info, warn};
        use uuid::Uuid;

        let bot_token = self
            .config
            .resolved_bot_token()
            .ok_or_else(|| anyhow::anyhow!("No Slack bot token configured (set bot_token in [slack] config or SLACK_BOT_TOKEN env var)"))?;

        let app_token = self
            .config
            .resolved_app_token()
            .ok_or_else(|| anyhow::anyhow!("No Slack app token configured (set app_token in [slack] config or SLACK_APP_TOKEN env var)"))?;

        let client = Arc::new(SlackClient::new(SlackClientHyperConnector::new()?));

        let token = SlackApiToken::new(bot_token.into());
        let app_token_val = SlackApiToken::new(app_token.into());

        info!("Connecting to Slack via Socket Mode");

        // Track one conversation_id per (channel, user) pair so the
        // orchestrator retains memory across messages from the same user in
        // the same channel.
        let conversations: Arc<tokio::sync::Mutex<HashMap<(String, String), Uuid>>> =
            Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        let config = self.config.clone();
        let orchestrator = self.orchestrator.clone();

        let socket_mode_callbacks = SlackSocketModeListenerCallbacks::new()
            .with_push_events(move |event: SlackPushEventCallback, _client, _states| {
                let config = config.clone();
                let orchestrator = orchestrator.clone();
                let conversations = conversations.clone();
                let token = token.clone();

                async move {
                    if let SlackEventCallbackBody::Message(msg) = &event.event {
                        let channel_id = msg
                            .origin
                            .channel
                            .as_ref()
                            .map(|c| c.to_string())
                            .unwrap_or_default();
                        let user_id = msg
                            .sender
                            .user
                            .as_ref()
                            .map(|u| u.to_string())
                            .unwrap_or_default();
                        let text = msg
                            .content
                            .as_ref()
                            .and_then(|c| c.text.as_ref())
                            .map(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();

                        // Skip bot messages and empty text.
                        if text.is_empty() || msg.sender.bot_id.is_some() {
                            debug!("Ignoring bot message or empty text");
                            return Ok(());
                        }

                        // Channel allowlist check.
                        if !config.allowed_channels.is_empty()
                            && !config.allowed_channels.contains(&channel_id)
                        {
                            warn!(channel = %channel_id, "Ignoring message from non-allowlisted channel");
                            return Ok(());
                        }

                        // User allowlist check.
                        if !config.allowed_users.is_empty()
                            && !config.allowed_users.contains(&user_id)
                        {
                            warn!(user = %user_id, "Ignoring message from non-allowlisted user");
                            return Ok(());
                        }

                        info!(channel = %channel_id, user = %user_id, text_len = text.len(), "Dispatching to orchestrator");

                        let conversation_id = {
                            let mut map = conversations.lock().await;
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

                        let turn_result = orchestrator
                            .run_turn_streaming(&text, conversation_id, Interface::Slack, tok_tx)
                            .await;

                        let reply = collector.await.unwrap_or_default();

                        if let Err(e) = turn_result {
                            tracing::error!(error = %e, "Orchestrator error");
                            return Ok(());
                        }

                        if reply.is_empty() {
                            return Ok(());
                        }

                        // Post the reply to the same channel.
                        let session = _client.open_session(&token);
                        let post_req = SlackApiChatPostMessageRequest::new(
                            channel_id.into(),
                            SlackMessageContent::new().with_text(reply),
                        );
                        if let Err(e) = session.chat_post_message(&post_req).await {
                            tracing::error!(error = %e, "Failed to post Slack reply");
                        }
                    }
                    Ok(())
                }
            });

        let listener_environment = Arc::new(
            SlackClientEventsListenerEnvironment::new(client.clone()).with_error_handler(
                |err, _client, _states| {
                    tracing::error!(error = %err, "Slack Socket Mode error");
                    std::future::ready(())
                },
            ),
        );

        let socket_mode_listener = SlackClientSocketModeListener::new(
            &SlackClientSocketModeConfig::new(),
            listener_environment,
            socket_mode_callbacks,
        );

        socket_mode_listener
            .listen_for(&app_token_val)
            .await
            .map_err(|e| anyhow::anyhow!("Slack Socket Mode listener error: {e}"))?;

        socket_mode_listener.serve().await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::SlackConfig;

    #[test]
    fn allowlist_channel_logic_empty_accepts_all() {
        let cfg = SlackConfig {
            allowed_channels: vec![],
            ..Default::default()
        };
        let channel = "C001".to_string();
        let blocked = !cfg.allowed_channels.is_empty() && !cfg.allowed_channels.contains(&channel);
        assert!(!blocked);
    }

    #[test]
    fn allowlist_channel_logic_non_empty_blocks_unknown() {
        let cfg = SlackConfig {
            allowed_channels: vec!["C001".to_string()],
            ..Default::default()
        };
        let unknown = "C999".to_string();
        let blocked = !cfg.allowed_channels.is_empty() && !cfg.allowed_channels.contains(&unknown);
        assert!(blocked);
    }

    #[test]
    fn allowlist_user_logic_non_empty_passes_known() {
        let cfg = SlackConfig {
            allowed_users: vec!["U001".to_string()],
            ..Default::default()
        };
        let known = "U001".to_string();
        let blocked = !cfg.allowed_users.is_empty() && !cfg.allowed_users.contains(&known);
        assert!(!blocked);
    }
}
