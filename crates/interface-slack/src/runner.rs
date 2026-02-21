//! Slack interface runner.
//!
//! Opens a Socket Mode WebSocket connection using `slack-morphism` and
//! dispatches each incoming [`SlackPushEventCallback`] message to the
//! [`Orchestrator`], then posts the reply back via the Slack Web API.
//!
//! # API notes
//!
//! `slack-morphism` uses **function-pointer** callbacks (`fn`, not `Fn`), so
//! shared state must be stored with
//! [`SlackClientEventsListenerEnvironment::with_user_state`] and retrieved
//! inside callbacks via `states.read().await.get_user_state::<T>()`.
//!
//! # Safety
//!
//! [`SafetyGate`][assistant_runtime::safety::SafetyGate] blocks `shell-exec`
//! for [`Interface::Slack`].  Additionally, `allowed_channels` and
//! `allowed_users` allowlists are checked before dispatching.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::Interface;
use assistant_runtime::Orchestrator;
use slack_morphism::prelude::*;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::{SlackConfig, SlackConfigExt};

// ── Shared callback state ─────────────────────────────────────────────────────

/// State stored in [`SlackClientEventsListenerEnvironment`] and retrieved
/// inside each callback via `states.read().await.get_user_state::<Self>()`.
struct SlackCallbackState {
    config: SlackConfig,
    orchestrator: Arc<Orchestrator>,
    /// Bot token used to post replies via the Web API.
    bot_token: SlackApiToken,
    /// One conversation UUID per (channel_id, user_id) pair.
    conversations: Arc<Mutex<HashMap<(String, String), Uuid>>>,
}

// ── Push-event callback (free async fn — function pointer, not closure) ───────

async fn on_push_event(
    event: SlackPushEventCallback,
    client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    states: SlackClientEventsUserState,
) -> UserCallbackResult<()> {
    // Retrieve shared state; clone the Arcs before dropping the guard.
    let (config, orchestrator, bot_token, conversations) = {
        let guard = states.read().await;
        let s = guard
            .get_user_state::<SlackCallbackState>()
            .expect("SlackCallbackState must be registered via with_user_state");
        (
            s.config.clone(),
            s.orchestrator.clone(),
            s.bot_token.clone(),
            s.conversations.clone(),
        )
    };

    let SlackEventCallbackBody::Message(msg) = &event.event else {
        return Ok(());
    };

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
    if !config.allowed_channels.is_empty() && !config.allowed_channels.contains(&channel_id) {
        warn!(channel = %channel_id, "Ignoring message from non-allowlisted channel");
        return Ok(());
    }

    // User allowlist check.
    if !config.allowed_users.is_empty() && !config.allowed_users.contains(&user_id) {
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

    // Post the reply to the originating channel.
    let session = client.open_session(&bot_token);
    let post_req = SlackApiChatPostMessageRequest::new(
        channel_id.into(),
        SlackMessageContent::new().with_text(reply),
    );
    if let Err(e) = session.chat_post_message(&post_req).await {
        tracing::error!(error = %e, "Failed to post Slack reply");
    }

    Ok(())
}

// ── Error handler (free fn — must return http::StatusCode) ────────────────────

fn on_error(
    err: Box<dyn std::error::Error + Send + Sync>,
    _client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    _states: SlackClientEventsUserState,
) -> HttpStatusCode {
    tracing::error!(error = %err, "Slack Socket Mode error");
    HttpStatusCode::OK
}

// ── SlackInterface ────────────────────────────────────────────────────────────

/// The Slack interface handle.
pub struct SlackInterface {
    config: SlackConfig,
    orchestrator: Arc<Orchestrator>,
}

impl SlackInterface {
    pub fn new(config: SlackConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
    }

    /// Start the Slack Socket Mode listener loop.
    pub async fn run(&self) -> Result<()> {
        let bot_token_str = self.config.resolved_bot_token().ok_or_else(|| {
            anyhow::anyhow!(
                "No Slack bot token configured. Set bot_token in [slack] config \
                 or the SLACK_BOT_TOKEN environment variable."
            )
        })?;

        let app_token_str = self.config.resolved_app_token().ok_or_else(|| {
            anyhow::anyhow!(
                "No Slack app-level token configured. Set app_token in [slack] config \
                 or the SLACK_APP_TOKEN environment variable."
            )
        })?;

        let client = Arc::new(SlackClient::new(SlackClientHyperHttpsConnector::new()?));
        let bot_token = SlackApiToken::new(bot_token_str.into());
        let app_token = SlackApiToken::new(app_token_str.into());

        info!("Connecting to Slack via Socket Mode");

        let state = SlackCallbackState {
            config: self.config.clone(),
            orchestrator: self.orchestrator.clone(),
            bot_token: bot_token.clone(),
            conversations: Arc::new(Mutex::new(HashMap::new())),
        };

        let listener_environment = Arc::new(
            SlackClientEventsListenerEnvironment::new(client.clone())
                .with_error_handler(on_error)
                .with_user_state(state),
        );

        let callbacks = SlackSocketModeListenerCallbacks::new().with_push_events(on_push_event);

        let socket_mode_listener = SlackClientSocketModeListener::new(
            &SlackClientSocketModeConfig::new(),
            listener_environment,
            callbacks,
        );

        socket_mode_listener
            .listen_for(&app_token)
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
