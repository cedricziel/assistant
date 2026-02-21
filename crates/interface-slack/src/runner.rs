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
//!
//! # Thread conversation context
//!
//! Each `(channel_id, thread_ts)` pair maps to a stable conversation UUID so
//! every message in the same Slack thread shares the same LLM conversation
//! history.  On the **first** time a thread is touched (after a restart or for
//! an existing thread the bot hasn't seen yet), the full thread history is
//! fetched via `conversations.replies` and seeded into the local
//! [`ConversationStore`] before the orchestrator runs.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{Interface, Message, MessageRole};
use assistant_runtime::Orchestrator;
use assistant_storage::StorageLayer;
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
    /// One conversation UUID per `(channel_id, thread_ts)` pair.
    /// Using `thread_ts` (instead of `user_id`) ensures every message in the
    /// same thread shares a single LLM conversation.
    conversations: Arc<Mutex<HashMap<(String, String), Uuid>>>,
    /// Unix timestamp (seconds) recorded at startup. Messages with a Slack `ts`
    /// older than this are stale catch-up events and are silently dropped.
    started_at: f64,
    /// Storage layer used to seed thread history into the conversation store.
    storage: Arc<StorageLayer>,
}

// ── History-message helpers ───────────────────────────────────────────────────

/// Classify a history message as a role for conversation seeding.
///
/// Returns:
/// - `Some(MessageRole::User)` — plain human message
/// - `Some(MessageRole::Assistant)` — bot / assistant message
/// - `None` — system event (message_changed, message_deleted, …); skip
fn classify_history_msg(msg: &SlackHistoryMessage) -> Option<MessageRole> {
    let is_bot = msg.sender.bot_id.is_some() || msg.sender.display_as_bot.unwrap_or(false);

    if is_bot {
        return Some(MessageRole::Assistant);
    }

    // Any non-bot subtype is a system event — skip.
    if msg.subtype.is_some() {
        return None;
    }

    // Plain human message.
    if msg.sender.user.is_some() {
        return Some(MessageRole::User);
    }

    None
}

// ── Push-event callback (free async fn — function pointer, not closure) ───────

async fn on_push_event(
    event: SlackPushEventCallback,
    client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    states: SlackClientEventsUserState,
) -> UserCallbackResult<()> {
    // Retrieve shared state; clone the Arcs before dropping the guard.
    let (config, orchestrator, bot_token, conversations, started_at, storage) = {
        let guard = states.read().await;
        let s = guard
            .get_user_state::<SlackCallbackState>()
            .expect("SlackCallbackState must be registered via with_user_state");
        (
            s.config.clone(),
            s.orchestrator.clone(),
            s.bot_token.clone(),
            s.conversations.clone(),
            s.started_at,
            s.storage.clone(),
        )
    };

    let SlackEventCallbackBody::Message(msg) = &event.event else {
        debug!(event_type = ?event.event, "Ignoring non-message event");
        return Ok(());
    };

    // Only process plain human messages (subtype == None).
    // Any subtype (bot_message, message_changed, message_deleted, …) means
    // a system/bot/meta event — including the message_changed event Slack fires
    // when we add a reaction, which would otherwise cause a duplicate reply.
    if msg.subtype.is_some()
        || msg.sender.bot_id.is_some()
        || msg.sender.display_as_bot.unwrap_or(false)
        || msg.sender.user.is_none()
    {
        debug!(subtype = ?msg.subtype, "Ignoring non-human message");
        return Ok(());
    }

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

    // Thread ts: inherit an existing thread or start a new one from this message.
    let thread_ts = msg
        .origin
        .thread_ts
        .clone()
        .unwrap_or_else(|| msg.origin.ts.clone());

    if text.is_empty() {
        debug!(channel = %channel_id, user = %user_id, "Ignoring empty message");
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

    // The ts of the original message — used to anchor reactions to it.
    let msg_ts = msg.origin.ts.clone();

    // Drop messages that predate our startup — these are catch-up events
    // replayed by Slack after a reconnect and should not be reprocessed.
    let msg_unix: f64 = msg_ts.0.parse().unwrap_or(0.0);
    if msg_unix < started_at {
        debug!(ts = %msg_ts.0, started_at, "Dropping pre-startup message");
        return Ok(());
    }

    info!(
        channel = %channel_id,
        user = %user_id,
        ts = %msg_ts.0,
        thread_ts = %thread_ts.0,
        text_len = text.len(),
        "Dispatching to orchestrator"
    );

    // Key conversations by (channel_id, thread_ts) so every message in the
    // same Slack thread shares a single LLM conversation context.
    let (conversation_id, is_new) = {
        let mut map = conversations.lock().await;
        let entry = map.entry((channel_id.clone(), thread_ts.0.clone()));
        let is_new = matches!(entry, Entry::Vacant(_));
        let id = *entry.or_insert_with(Uuid::new_v4);
        (id, is_new)
    };

    debug!(conversation_id = %conversation_id, is_new, "Using conversation");

    // Open a Slack API session (cheaply wraps the client + token).
    let session = client.open_session(&bot_token);

    // ── Thread history hydration ──────────────────────────────────────────────
    //
    // On first touch of a thread that already has messages in Slack (i.e. the
    // bot was restarted or the thread existed before the bot joined), fetch the
    // full thread history via `conversations.replies` and seed it into the
    // local ConversationStore so the LLM has full context.
    //
    // `thread_ts == msg_ts` means this IS the parent (first) message, so there
    // is no prior history to hydrate.
    if is_new && thread_ts.0 != msg_ts.0 {
        let conv_store = storage.conversation_store();
        // Idempotent upsert — safe to call even if orchestrator creates it later.
        let title = format!("slack:{}:{}", channel_id, thread_ts.0);
        if let Err(e) = conv_store
            .create_conversation_with_id(conversation_id, Some(&title))
            .await
        {
            warn!(error = %e, "Failed to create conversation for history seeding");
        } else {
            let replies_req = SlackApiConversationsRepliesRequest::new(
                channel_id.clone().into(),
                thread_ts.clone(),
            );
            match session.conversations_replies(&replies_req).await {
                Ok(resp) => {
                    let mut seeded = 0usize;
                    for hist_msg in &resp.messages {
                        // Skip the triggering message — the orchestrator adds it.
                        if hist_msg.origin.ts == msg_ts {
                            continue;
                        }
                        let role = match classify_history_msg(hist_msg) {
                            Some(r) => r,
                            None => continue,
                        };
                        let content = hist_msg.content.text.as_deref().unwrap_or("").to_string();
                        if content.is_empty() {
                            continue;
                        }
                        let msg_to_seed = Message::new(conversation_id, role, content);
                        if let Err(e) = conv_store.save_message(&msg_to_seed).await {
                            warn!(error = %e, "Failed to seed thread history message");
                        } else {
                            seeded += 1;
                        }
                    }
                    info!(
                        conversation_id = %conversation_id,
                        seeded,
                        "Seeded thread history"
                    );
                }
                Err(e) => {
                    warn!(error = %e, "Failed to fetch thread history; proceeding without context");
                }
            }
        }
    }

    // Acknowledge receipt with 👀 — visible while the orchestrator is running.
    let ack_req = SlackApiReactionsAddRequest::new(
        channel_id.clone().into(),
        SlackReactionName("eyes".to_string()),
        msg_ts.clone(),
    );
    if let Err(e) = session.reactions_add(&ack_req).await {
        let msg = e.to_string();
        if msg.contains("already_reacted") {
            debug!("Eyes reaction already present, skipping");
        } else {
            warn!(error = %e, "Failed to add eyes reaction");
        }
    }

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

    // Remove 👀 regardless of outcome.
    let remove_req = SlackApiReactionsRemoveRequest::new(SlackReactionName("eyes".to_string()))
        .with_channel(channel_id.clone().into())
        .with_timestamp(msg_ts.clone());
    if let Err(e) = session.reactions_remove(&remove_req).await {
        warn!(error = %e, "Failed to remove eyes reaction");
    }

    if let Err(e) = turn_result {
        tracing::error!(error = %e, "Orchestrator error");
        return Ok(());
    }

    if reply.is_empty() {
        debug!(channel = %channel_id, "Orchestrator returned empty reply, skipping post");
        return Ok(());
    }

    info!(channel = %channel_id, reply_len = reply.len(), "Posting reply to Slack");

    // Post the reply in the same thread as the triggering message.
    let post_req = SlackApiChatPostMessageRequest::new(
        channel_id.into(),
        SlackMessageContent::new().with_text(reply),
    )
    .with_thread_ts(thread_ts);
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
    storage: Arc<StorageLayer>,
}

impl SlackInterface {
    pub fn new(
        config: SlackConfig,
        orchestrator: Arc<Orchestrator>,
        storage: Arc<StorageLayer>,
    ) -> Self {
        Self {
            config,
            orchestrator,
            storage,
        }
    }

    /// Start the Slack Socket Mode listener loop, reconnecting on disconnect.
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

        // Conversation map persists across reconnects so in-flight context is not lost.
        let conversations = Arc::new(Mutex::new(HashMap::new()));

        let mut backoff = std::time::Duration::from_secs(1);

        loop {
            let started_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();

            info!(started_at, "Connecting to Slack via Socket Mode");

            let state = SlackCallbackState {
                config: self.config.clone(),
                orchestrator: self.orchestrator.clone(),
                bot_token: bot_token.clone(),
                conversations: conversations.clone(),
                started_at,
                storage: self.storage.clone(),
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

            match socket_mode_listener.listen_for(&app_token).await {
                Ok(()) => {
                    socket_mode_listener.serve().await;
                    info!("Slack Socket Mode connection closed, reconnecting…");
                }
                Err(e) => {
                    warn!(error = %e, delay_secs = backoff.as_secs(), "Slack connection failed, retrying");
                }
            }

            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(std::time::Duration::from_secs(60));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::Entry;
    use std::collections::HashMap;

    use assistant_core::SlackConfig;
    use uuid::Uuid;

    use super::classify_history_msg;
    use slack_morphism::prelude::*;

    // ── Allowlist tests ───────────────────────────────────────────────────────

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

    // ── Conversation key tests ────────────────────────────────────────────────

    /// Two messages in the **same thread** (same channel + thread_ts, different
    /// user) must share a single conversation UUID.
    #[test]
    fn same_thread_same_conversation() {
        let mut conversations: HashMap<(String, String), Uuid> = HashMap::new();
        let channel = "C001".to_string();
        let thread_ts = "1700000000.000000".to_string();

        let id_a = *conversations
            .entry((channel.clone(), thread_ts.clone()))
            .or_insert_with(Uuid::new_v4);
        // Second call (e.g. different user replying in the thread) — same key.
        let id_b = *conversations
            .entry((channel.clone(), thread_ts.clone()))
            .or_insert_with(Uuid::new_v4);

        assert_eq!(
            id_a, id_b,
            "messages in the same thread must share a conversation"
        );
    }

    /// Messages in **different threads** within the same channel get separate
    /// conversation UUIDs.
    #[test]
    fn different_threads_different_conversations() {
        let mut conversations: HashMap<(String, String), Uuid> = HashMap::new();
        let channel = "C001".to_string();

        let id1 = *conversations
            .entry((channel.clone(), "1700000000.000000".to_string()))
            .or_insert_with(Uuid::new_v4);
        let id2 = *conversations
            .entry((channel.clone(), "1700000001.000000".to_string()))
            .or_insert_with(Uuid::new_v4);

        assert_ne!(
            id1, id2,
            "different threads must get different conversation IDs"
        );
    }

    /// The first time a key is inserted it must be detected as new; subsequent
    /// lookups must NOT be detected as new.
    #[test]
    fn is_new_detection() {
        let mut conversations: HashMap<(String, String), Uuid> = HashMap::new();
        let key = ("C001".to_string(), "1700000000.000000".to_string());

        let is_new_first = {
            let entry = conversations.entry(key.clone());
            let new = matches!(entry, Entry::Vacant(_));
            entry.or_insert_with(Uuid::new_v4);
            new
        };
        assert!(is_new_first, "first insert must be detected as new");

        let is_new_second = {
            let entry = conversations.entry(key.clone());
            let new = matches!(entry, Entry::Vacant(_));
            entry.or_insert_with(Uuid::new_v4);
            new
        };
        assert!(
            !is_new_second,
            "repeated lookup must NOT be detected as new"
        );
    }

    // ── History message classification tests ──────────────────────────────────

    fn make_human_history_msg(ts: &str) -> SlackHistoryMessage {
        SlackHistoryMessage {
            origin: SlackMessageOrigin {
                ts: SlackTs(ts.to_string()),
                channel: None,
                channel_type: None,
                thread_ts: None,
                client_msg_id: None,
            },
            content: SlackMessageContent::new().with_text("Hello!".to_string()),
            sender: SlackMessageSender {
                user: Some(SlackUserId("U001".to_string())),
                bot_id: None,
                username: None,
                display_as_bot: None,
                user_profile: None,
                bot_profile: None,
            },
            parent: SlackParentMessageParams {
                reply_count: None,
                reply_users_count: None,
                latest_reply: None,
                reply_users: None,
                subscribed: None,
                last_read: None,
            },
            subtype: None,
            edited: None,
        }
    }

    fn make_bot_history_msg(ts: &str) -> SlackHistoryMessage {
        SlackHistoryMessage {
            origin: SlackMessageOrigin {
                ts: SlackTs(ts.to_string()),
                channel: None,
                channel_type: None,
                thread_ts: None,
                client_msg_id: None,
            },
            content: SlackMessageContent::new().with_text("I am the bot.".to_string()),
            sender: SlackMessageSender {
                user: None,
                bot_id: Some(SlackBotId("B001".to_string())),
                username: None,
                display_as_bot: None,
                user_profile: None,
                bot_profile: None,
            },
            parent: SlackParentMessageParams {
                reply_count: None,
                reply_users_count: None,
                latest_reply: None,
                reply_users: None,
                subscribed: None,
                last_read: None,
            },
            subtype: Some(SlackMessageEventType::BotMessage),
            edited: None,
        }
    }

    fn make_system_history_msg(ts: &str) -> SlackHistoryMessage {
        SlackHistoryMessage {
            origin: SlackMessageOrigin {
                ts: SlackTs(ts.to_string()),
                channel: None,
                channel_type: None,
                thread_ts: None,
                client_msg_id: None,
            },
            content: SlackMessageContent::new(),
            sender: SlackMessageSender {
                user: None,
                bot_id: None,
                username: None,
                display_as_bot: None,
                user_profile: None,
                bot_profile: None,
            },
            parent: SlackParentMessageParams {
                reply_count: None,
                reply_users_count: None,
                latest_reply: None,
                reply_users: None,
                subscribed: None,
                last_read: None,
            },
            subtype: Some(SlackMessageEventType::MessageChanged),
            edited: None,
        }
    }

    #[test]
    fn classify_human_message_as_user() {
        use assistant_core::MessageRole;
        let msg = make_human_history_msg("1700000000.000000");
        assert_eq!(classify_history_msg(&msg), Some(MessageRole::User));
    }

    #[test]
    fn classify_bot_message_as_assistant() {
        use assistant_core::MessageRole;
        let msg = make_bot_history_msg("1700000001.000000");
        assert_eq!(classify_history_msg(&msg), Some(MessageRole::Assistant));
    }

    #[test]
    fn classify_system_event_as_none() {
        let msg = make_system_history_msg("1700000002.000000");
        assert_eq!(classify_history_msg(&msg), None);
    }
}
