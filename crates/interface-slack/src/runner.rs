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
//! Skills listed in `disabled_skills` are blocked before dispatch.
//! Additionally, `allowed_channels` and `allowed_users` allowlists are
//! checked before dispatching.
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
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{Interface, Message, MessageRole};
use assistant_runtime::Orchestrator;
use assistant_storage::StorageLayer;
use slack_morphism::prelude::*;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Short preview of a string for log output (avoids flooding logs with long messages).
fn preview(s: &str, max: usize) -> &str {
    let end = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
    &s[..end]
}

#[derive(Debug)]
enum SlackIncomingKind {
    Message,
    Reaction { emoji: String },
}

#[derive(Debug)]
struct SlackIncomingEvent {
    channel_id: String,
    user_id: String,
    thread_ts: SlackTs,
    /// Unique timestamp for this specific event (used for dedup + stale checks).
    event_ts: SlackTs,
    /// Timestamp of the Slack message we anchor reactions/typing indicators to.
    ack_ts: SlackTs,
    text: String,
    kind: SlackIncomingKind,
}

/// A message waiting to be processed in a conversation turn.
///
/// When multiple messages arrive for the same thread while a turn is in-flight,
/// they accumulate in `pending_messages` and are drained into a single combined
/// orchestrator turn once the per-conversation lock is acquired.
struct PendingMessage {
    text: String,
    user_id: String,
    channel_id: String,
    thread_ts: SlackTs,
    msg_ts: SlackTs,
}

/// Build a single contextualized prompt from a batch of pending messages.
///
/// Each message gets a `[Slack: user=… channel=…]` header so the LLM can
/// attribute authorship.  Messages are separated by a blank line.
fn build_batch_prompt(batch: &[PendingMessage]) -> String {
    batch
        .iter()
        .map(|m| {
            format!(
                "[Slack: user={} channel={}]\n{}",
                m.user_id, m.channel_id, m.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

use crate::config::{SlackConfig, SlackConfigExt};
use crate::skills::{
    SlackDeleteMessageSkill, SlackGetHistorySkill, SlackListChannelsSkill, SlackLookupUserSkill,
    SlackPostSkill, SlackReactSkill, SlackSendDmSkill, SlackUpdateMessageSkill,
};
use crate::tools::build_slack_tools;

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
    /// Deduplication set of already-processed message `ts` values.
    ///
    /// `slack-morphism` opens two WebSocket connections for redundancy; both
    /// deliver the same push event, so without this guard each message would
    /// be processed twice.  Entries are retained until the set exceeds 500
    /// items, at which point it is cleared (ts values are monotonically
    /// increasing, so an old entry will never match a new message).
    processed_ts: Arc<Mutex<HashSet<String>>>,
    /// Unix timestamp (seconds) recorded at startup. Messages with a Slack `ts`
    /// older than this are stale catch-up events and are silently dropped.
    started_at: f64,
    /// Storage layer used to seed thread history into the conversation store.
    storage: Arc<StorageLayer>,
    /// Per-conversation serialisation mutex.
    ///
    /// Each conversation has a single `Mutex<()>` token; holding it means
    /// a turn is currently running for that conversation.  New messages wait
    /// for the current turn to finish before starting their own, ensuring
    /// the LLM always sees a consistent, complete history.
    conv_locks: Arc<Mutex<HashMap<Uuid, Arc<tokio::sync::Mutex<()>>>>>,
    /// Per-conversation queue of messages waiting for their turn.
    ///
    /// Messages push themselves here before acquiring the per-conversation
    /// lock.  When the lock is acquired, the handler drains all pending
    /// messages and combines them into a single orchestrator turn.  This
    /// avoids N sequential turns when N messages arrive during one turn.
    pending_messages: Arc<Mutex<HashMap<Uuid, Vec<PendingMessage>>>>,
}

// ── History-message helpers ───────────────────────────────────────────────────

/// Classify a history message as a role for conversation seeding.
///
/// Returns:
/// - `Some(MessageRole::User)` — plain human message
/// - `Some(MessageRole::Assistant)` — bot / assistant message
/// - `None` — system event (message_changed, message_deleted, …); skip
fn classify_history_msg(msg: &SlackHistoryMessage) -> Option<MessageRole> {
    // Incoming-webhook bot messages have subtype=BotMessage but no bot_id.
    let is_bot = msg.sender.bot_id.is_some()
        || msg.sender.display_as_bot.unwrap_or(false)
        || matches!(msg.subtype, Some(SlackMessageEventType::BotMessage));

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

fn incoming_from_message_event(msg: &SlackMessageEvent) -> Option<SlackIncomingEvent> {
    if msg.subtype.is_some()
        || msg.sender.bot_id.is_some()
        || msg.sender.display_as_bot.unwrap_or(false)
        || msg.sender.user.is_none()
    {
        return None;
    }

    let user_id = msg.sender.user.as_ref()?.to_string();
    let channel_id = msg.origin.channel.as_ref()?.to_string();
    let text = msg
        .content
        .as_ref()
        .and_then(|c| c.text.as_ref())
        .map(|t| t.to_string())
        .unwrap_or_default();

    if text.trim().is_empty() {
        return None;
    }

    let msg_ts = msg.origin.ts.clone();
    let thread_ts = msg
        .origin
        .thread_ts
        .clone()
        .unwrap_or_else(|| msg.origin.ts.clone());

    Some(SlackIncomingEvent {
        channel_id,
        user_id,
        thread_ts,
        event_ts: msg_ts.clone(),
        ack_ts: msg_ts,
        text,
        kind: SlackIncomingKind::Message,
    })
}

fn incoming_from_reaction_event(event: &SlackReactionAddedEvent) -> Option<SlackIncomingEvent> {
    let SlackReactionsItem::Message(item_msg) = &event.item else {
        return None;
    };

    let channel_id = item_msg.origin.channel.as_ref()?.to_string();
    let thread_ts = item_msg
        .origin
        .thread_ts
        .clone()
        .unwrap_or_else(|| item_msg.origin.ts.clone());
    let ack_ts = item_msg.origin.ts.clone();

    let original = item_msg.content.text.as_deref().unwrap_or("");
    let snippet = preview(original, 120).trim().to_string();
    let emoji = event.reaction.0.clone();
    let mut text = format!("Reaction :{}: from {}", emoji, event.user);
    if !snippet.is_empty() {
        text.push_str(&format!(" on message: {}", snippet));
    }

    Some(SlackIncomingEvent {
        channel_id,
        user_id: event.user.to_string(),
        thread_ts,
        event_ts: event.event_ts.clone(),
        ack_ts,
        text,
        kind: SlackIncomingKind::Reaction { emoji },
    })
}

// ── Push-event callback (free async fn — function pointer, not closure) ───────

async fn on_push_event(
    event: SlackPushEventCallback,
    client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    states: SlackClientEventsUserState,
) -> UserCallbackResult<()> {
    // Retrieve shared state; clone the Arcs before dropping the guard.
    let (
        config,
        orchestrator,
        bot_token,
        conversations,
        processed_ts,
        started_at,
        storage,
        conv_locks,
        pending_messages,
    ) = {
        let guard = states.read().await;
        let s = guard
            .get_user_state::<SlackCallbackState>()
            .ok_or_else(|| {
                Box::<dyn std::error::Error + Send + Sync>::from(
                    "SlackCallbackState not registered in states",
                )
            })?;
        (
            s.config.clone(),
            s.orchestrator.clone(),
            s.bot_token.clone(),
            s.conversations.clone(),
            s.processed_ts.clone(),
            s.started_at,
            s.storage.clone(),
            s.conv_locks.clone(),
            s.pending_messages.clone(),
        )
    };

    let incoming = match &event.event {
        SlackEventCallbackBody::Message(msg) => match incoming_from_message_event(msg) {
            Some(evt) => evt,
            None => {
                debug!("Ignoring non-human message event");
                return Ok(());
            }
        },
        SlackEventCallbackBody::ReactionAdded(reaction) => {
            match incoming_from_reaction_event(reaction) {
                Some(evt) => evt,
                None => {
                    debug!("Ignoring reaction event that is not tied to a message");
                    return Ok(());
                }
            }
        }
        other => {
            debug!(event_type = ?other, "Ignoring unsupported event");
            return Ok(());
        }
    };

    let channel_id = incoming.channel_id.clone();
    let user_id = incoming.user_id.clone();
    let text = incoming.text.clone();
    if text.trim().is_empty() {
        debug!(channel = %channel_id, user = %user_id, "Ignoring empty payload");
        return Ok(());
    }
    let thread_ts = incoming.thread_ts.clone();
    let msg_ts = incoming.ack_ts.clone();

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

    // Deduplicate: slack-morphism opens two WebSocket connections; both deliver
    // the same push event, which would cause double processing without this guard.
    {
        let mut seen = processed_ts.lock().await;
        if !seen.insert(incoming.event_ts.0.clone()) {
            debug!(ts = %incoming.event_ts.0, "Skipping duplicate event (already processed on other connection)");
            return Ok(());
        }
        // Prune to prevent unbounded growth. Clear older entries but retain the
        // ts we just inserted so a delayed duplicate from the other connection
        // is still rejected after the prune.
        if seen.len() > 500 {
            let current = incoming.event_ts.0.clone();
            seen.clear();
            seen.insert(current);
        }
    }

    // Drop messages that predate our startup — these are catch-up events
    // replayed by Slack after a reconnect and should not be reprocessed.
    let msg_unix: f64 = incoming.event_ts.0.parse().unwrap_or(0.0);
    if msg_unix < started_at {
        debug!(ts = %msg_ts.0, started_at, "Dropping pre-startup message");
        return Ok(());
    }

    let (event_kind, reaction_emoji) = match &incoming.kind {
        SlackIncomingKind::Message => ("message", None),
        SlackIncomingKind::Reaction { emoji } => ("reaction", Some(emoji.as_str())),
    };

    info!(
        channel = %channel_id,
        user = %user_id,
        ts = %incoming.event_ts.0,
        thread_ts = %thread_ts.0,
        event_kind,
        text_len = text.len(),
        text_preview = preview(&text, 120),
        reaction = ?reaction_emoji,
        "Incoming Slack event"
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
            // Paginate through all replies — Slack may return `has_more: true`
            // with a cursor for long-lived threads (>100 messages per page).
            let mut cursor: Option<SlackCursorId> = None;
            let mut seeded = 0usize;
            let mut fetch_error = false;
            loop {
                let mut replies_req = SlackApiConversationsRepliesRequest::new(
                    channel_id.clone().into(),
                    thread_ts.clone(),
                );
                if let Some(ref c) = cursor {
                    replies_req = replies_req.with_cursor(c.clone());
                }
                match session.conversations_replies(&replies_req).await {
                    Ok(resp) => {
                        for hist_msg in &resp.messages {
                            // Skip the triggering message when we're currently
                            // processing a new user message (the orchestrator
                            // inserts it). Reaction events are synthetic, so
                            // we keep the original message in-context.
                            if matches!(incoming.kind, SlackIncomingKind::Message)
                                && hist_msg.origin.ts == msg_ts
                            {
                                continue;
                            }
                            let role = match classify_history_msg(hist_msg) {
                                Some(r) => r,
                                None => continue,
                            };
                            let content =
                                hist_msg.content.text.as_deref().unwrap_or("").to_string();
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
                        // Advance cursor; exit loop when all pages consumed.
                        if resp
                            .response_metadata
                            .as_ref()
                            .and_then(|m| m.next_cursor.as_ref())
                            .map(|c| !c.0.is_empty())
                            .unwrap_or(false)
                        {
                            cursor = resp.response_metadata.and_then(|m| m.next_cursor);
                        } else {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to fetch thread history; proceeding without context");
                        fetch_error = true;
                        break;
                    }
                }
            }
            if !fetch_error {
                info!(
                    conversation_id = %conversation_id,
                    seeded,
                    "Seeded thread history"
                );
            }
        }
    }

    // Acknowledge receipt with ⏳ — visible while the message is queued,
    // before the per-conversation lock is acquired.
    debug!(channel = %channel_id, ts = %msg_ts.0, "reactions.add hourglass_flowing_sand (queued)");
    let queue_req = SlackApiReactionsAddRequest::new(
        channel_id.clone().into(),
        SlackReactionName("hourglass_flowing_sand".to_string()),
        msg_ts.clone(),
    );
    if let Err(e) = session.reactions_add(&queue_req).await {
        let msg = e.to_string();
        if msg.contains("already_reacted") {
            debug!("Hourglass reaction already present, skipping");
        } else {
            warn!(error = %e, "reactions.add hourglass_flowing_sand failed");
        }
    }

    // Push this message into the per-conversation pending queue so that if
    // multiple messages pile up while a turn is in-flight, they can be
    // drained and combined into a single orchestrator turn.
    {
        let mut queue = pending_messages.lock().await;
        queue
            .entry(conversation_id)
            .or_default()
            .push(PendingMessage {
                text: text.clone(),
                user_id: user_id.clone(),
                channel_id: channel_id.clone(),
                thread_ts: thread_ts.clone(),
                msg_ts: msg_ts.clone(),
            });
    }

    // Acquire (or create) the per-conversation mutex so that if two messages
    // arrive for the same thread while a turn is in-flight, the second one
    // waits until the first turn finishes.  This prevents interleaved history
    // and duplicate replies caused by concurrent orchestrator runs.
    let conv_lock = {
        let mut locks = conv_locks.lock().await;
        locks
            .entry(conversation_id)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };
    let _conv_guard = conv_lock.lock().await;

    // ── Drain the pending queue ──────────────────────────────────────────────
    //
    // All messages that arrived while the previous turn was running (including
    // our own) are now waiting in the queue.  Drain them all so they become a
    // single combined orchestrator turn instead of N sequential turns.
    let batch: Vec<PendingMessage> = {
        let mut queue = pending_messages.lock().await;
        queue.remove(&conversation_id).unwrap_or_default()
    };

    if batch.is_empty() {
        // Another handler already drained our message — nothing to do.
        debug!(conversation_id = %conversation_id, "Pending queue empty, another handler processed our message");
        return Ok(());
    }

    let batch_size = batch.len();
    info!(
        conversation_id = %conversation_id,
        batch_size,
        "Drained pending queue — processing as single turn"
    );

    // Swap ⏳ → 👀 for every message in the batch now that we're actively
    // processing this turn.
    for pending in &batch {
        debug!(channel = %pending.channel_id, ts = %pending.msg_ts.0, "⏳ → 👀");
        let remove_hg = SlackApiReactionsRemoveRequest::new(SlackReactionName(
            "hourglass_flowing_sand".to_string(),
        ))
        .with_channel(pending.channel_id.clone().into())
        .with_timestamp(pending.msg_ts.clone());
        if let Err(e) = session.reactions_remove(&remove_hg).await {
            warn!(error = %e, ts = %pending.msg_ts.0, "reactions.remove hourglass_flowing_sand failed");
        }
        let add_eyes = SlackApiReactionsAddRequest::new(
            pending.channel_id.clone().into(),
            SlackReactionName("eyes".to_string()),
            pending.msg_ts.clone(),
        );
        if let Err(e) = session.reactions_add(&add_eyes).await {
            let msg = e.to_string();
            if msg.contains("already_reacted") {
                debug!("Eyes reaction already present, skipping");
            } else {
                warn!(error = %e, ts = %pending.msg_ts.0, "reactions.add eyes failed");
            }
        }
    }

    // Show a typing/processing status in the thread via the Assistant API.
    debug!(channel = %channel_id, thread_ts = %thread_ts.0, "assistant.threads.setStatus → is thinking…");
    let set_status_req = SlackApiAssistantThreadsSetStatusRequest::new(
        channel_id.clone().into(),
        "is thinking\u{2026}".to_string(),
        thread_ts.clone(),
    );
    if let Err(e) = session.assistant_threads_set_status(&set_status_req).await {
        debug!(error = %e, "assistant.threads.setStatus failed");
    }

    // Build per-turn Slack extension tools bound to the *last* message in the
    // batch (most recent), so `react` targets the newest user message.
    let last = batch.last().expect("batch is non-empty");
    let extensions = build_slack_tools(
        last.channel_id.clone(),
        Some(last.thread_ts.clone()),
        last.msg_ts.clone(),
        client.clone(),
        bot_token.clone(),
    );

    // Combine all pending messages into a single contextualized prompt.
    // Each message retains its own Slack context header so the LLM knows
    // who sent what.
    let contextualized_text = build_batch_prompt(&batch);

    let orchestrator_start = std::time::Instant::now();
    debug!(
        conversation_id = %conversation_id,
        batch_size,
        text_len = contextualized_text.len(),
        "orchestrator.run_turn_with_tools →"
    );
    let turn_result = orchestrator
        .run_turn_with_tools(
            &contextualized_text,
            conversation_id,
            Interface::Slack,
            extensions,
            None,
        )
        .await;
    let elapsed_ms = orchestrator_start.elapsed().as_millis();

    // Remove 👀 from every message in the batch and clear the typing status
    // — regardless of outcome.
    for pending in &batch {
        debug!(channel = %pending.channel_id, ts = %pending.msg_ts.0, "reactions.remove eyes");
        let remove_eyes =
            SlackApiReactionsRemoveRequest::new(SlackReactionName("eyes".to_string()))
                .with_channel(pending.channel_id.clone().into())
                .with_timestamp(pending.msg_ts.clone());
        if let Err(e) = session.reactions_remove(&remove_eyes).await {
            warn!(error = %e, ts = %pending.msg_ts.0, "reactions.remove eyes failed");
        }
    }

    debug!(channel = %channel_id, thread_ts = %thread_ts.0, "assistant.threads.setStatus → clear");
    let clear_status_req = SlackApiAssistantThreadsSetStatusRequest::new(
        channel_id.clone().into(),
        String::new(),
        thread_ts.clone(),
    );
    if let Err(e) = session
        .assistant_threads_set_status(&clear_status_req)
        .await
    {
        debug!(error = %e, "assistant.threads.setStatus clear failed");
    }

    if let Err(e) = turn_result {
        error!(error = %e, elapsed_ms, batch_size, "orchestrator error");
        // Notify the user so they aren't left waiting silently.
        let err_req = SlackApiChatPostMessageRequest::new(
            channel_id.clone().into(),
            SlackMessageContent::new()
                .with_text("Sorry, something went wrong processing your message.".to_string()),
        )
        .with_thread_ts(thread_ts.clone());
        if let Err(post_err) = session.chat_post_message(&err_req).await {
            warn!(error = %post_err, "Failed to post error feedback to user");
        }
        return Ok(());
    }

    debug!(
        conversation_id = %conversation_id,
        elapsed_ms,
        batch_size,
        "orchestrator.run_turn_with_tools ← ok"
    );

    Ok(())
}

// ── Error handler (free fn — must return http::StatusCode) ────────────────────

fn on_error(
    err: Box<dyn std::error::Error + Send + Sync>,
    _client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    _states: SlackClientEventsUserState,
) -> HttpStatusCode {
    error!(error = %err, "Slack Socket Mode error");
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

    /// Return ambient tools contributed by this interface.
    ///
    /// Currently returns a single `slack-post` tool that allows the agent to
    /// proactively post messages to any Slack channel.  Returns an empty vec
    /// if the bot token is not configured or the HTTP client cannot be created.
    pub fn ambient_tools(&self) -> Vec<std::sync::Arc<dyn assistant_core::ToolHandler>> {
        let Some(bot_token_str) = self.config.resolved_bot_token() else {
            return vec![];
        };
        let client = match SlackClientHyperHttpsConnector::new() {
            Ok(connector) => std::sync::Arc::new(SlackClient::new(connector)),
            Err(e) => {
                warn!(error = %e, "slack-post: failed to create HTTP client");
                return vec![];
            }
        };
        let token = SlackApiToken::new(bot_token_str.into());
        vec![
            std::sync::Arc::new(SlackPostSkill {
                client: client.clone(),
                token: token.clone(),
            }) as std::sync::Arc<dyn assistant_core::ToolHandler>,
            std::sync::Arc::new(SlackSendDmSkill {
                client: client.clone(),
                token: token.clone(),
            }),
            std::sync::Arc::new(SlackListChannelsSkill {
                client: client.clone(),
                token: token.clone(),
            }),
            std::sync::Arc::new(SlackGetHistorySkill {
                client: client.clone(),
                token: token.clone(),
            }),
            std::sync::Arc::new(SlackReactSkill {
                client: client.clone(),
                token: token.clone(),
            }),
            std::sync::Arc::new(SlackUpdateMessageSkill {
                client: client.clone(),
                token: token.clone(),
            }),
            std::sync::Arc::new(SlackDeleteMessageSkill {
                client: client.clone(),
                token: token.clone(),
            }),
            std::sync::Arc::new(SlackLookupUserSkill { client, token }),
        ]
    }

    /// Start the Slack Socket Mode listener loop, reconnecting on disconnect.
    ///
    /// The loop exits cleanly on SIGINT (Ctrl+C) or SIGTERM.
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

        // Mark the bot as active. This may silently fail for some bot token
        // configurations; that is acceptable.
        {
            let session = client.open_session(&bot_token);
            if let Err(e) = session
                .users_set_presence(&SlackApiUsersSetPresenceRequest::new("auto".to_string()))
                .await
            {
                debug!(error = %e, "users.setPresence(auto) failed (missing_scope is expected for most bot tokens)");
            } else {
                info!("Presence set to auto");
            }
        }

        // Conversation map persists across reconnects so in-flight context is not lost.
        let conversations = Arc::new(Mutex::new(HashMap::new()));
        // Dedup set persists across reconnects so a message delivered on one
        // connection is not reprocessed when the other connection reconnects.
        let processed_ts: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        // Per-conversation serialisation mutexes — persists across reconnects
        // so a turn in progress is not interrupted by a reconnect event.
        let conv_locks: Arc<Mutex<HashMap<Uuid, Arc<tokio::sync::Mutex<()>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // ── Graceful shutdown ─────────────────────────────────────────────────
        // A watch channel delivers the shutdown signal across all select! points
        // in the reconnect loop without the permit-loss race of Notify.
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
            // Exit immediately if shutdown was signalled between iterations.
            if *shutdown_rx.borrow() {
                break;
            }

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
                processed_ts: processed_ts.clone(),
                started_at,
                storage: self.storage.clone(),
                conv_locks: conv_locks.clone(),
                pending_messages: Arc::new(Mutex::new(HashMap::new())),
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

            // Race connection setup against shutdown.
            let connected = tokio::select! {
                result = socket_mode_listener.listen_for(&app_token) => {
                    match result {
                        Ok(()) => true,
                        Err(e) => {
                            warn!(error = %e, delay_secs = backoff.as_secs(),
                                  "Slack connection failed, retrying");
                            false
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    info!("Shutdown during connection, exiting");
                    return Ok(());
                }
            };

            if connected {
                // Race the event-serve loop against shutdown.
                let clean_close = tokio::select! {
                    _ = socket_mode_listener.serve() => {
                        info!("Slack Socket Mode connection closed, reconnecting…");
                        true
                    }
                    _ = shutdown_rx.changed() => {
                        info!("Shutdown during serve, exiting");
                        return Ok(());
                    }
                };
                // A clean server-initiated close means the connection was healthy;
                // reset the backoff so the next reconnect happens quickly.
                if clean_close {
                    backoff = std::time::Duration::from_secs(1);
                }
            }

            // Backoff sleep — also interruptible by shutdown.
            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                _ = shutdown_rx.changed() => {
                    info!("Shutdown during backoff, exiting");
                    return Ok(());
                }
            }

            backoff = (backoff * 2).min(std::time::Duration::from_secs(60));
        }

        info!("Slack interface stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::Entry;
    use std::collections::HashMap;
    use std::sync::Arc;

    use assistant_core::SlackConfig;
    use slack_morphism::prelude::*;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use super::{
        build_batch_prompt, classify_history_msg, incoming_from_message_event,
        incoming_from_reaction_event, PendingMessage, SlackIncomingKind,
    };

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

    fn make_message_event(text: &str) -> SlackMessageEvent {
        SlackMessageEvent {
            origin: SlackMessageOrigin {
                ts: SlackTs("1700000000.000001".to_string()),
                channel: Some(SlackChannelId("C001".to_string())),
                channel_type: Some(SlackChannelType("channel".to_string())),
                thread_ts: None,
                client_msg_id: Some(SlackClientMessageId("client-1".to_string())),
            },
            content: Some(SlackMessageContent::new().with_text(text.to_string())),
            sender: SlackMessageSender {
                user: Some(SlackUserId("U001".to_string())),
                bot_id: None,
                username: None,
                display_as_bot: Some(false),
                user_profile: None,
                bot_profile: None,
            },
            subtype: None,
            hidden: None,
            message: None,
            previous_message: None,
            deleted_ts: None,
        }
    }

    #[test]
    fn incoming_from_message_event_produces_turn() {
        let msg = make_message_event("Hello Slack");
        let incoming = incoming_from_message_event(&msg).expect("expected event");
        assert!(matches!(incoming.kind, SlackIncomingKind::Message));
        assert_eq!(incoming.channel_id, "C001");
        assert_eq!(incoming.user_id, "U001");
        assert_eq!(incoming.text, "Hello Slack");
        assert_eq!(incoming.thread_ts.0, "1700000000.000001");
    }

    #[test]
    fn incoming_from_reaction_event_mentions_emoji() {
        let history_msg = SlackHistoryMessage {
            origin: SlackMessageOrigin {
                ts: SlackTs("1700000000.000050".to_string()),
                channel: Some(SlackChannelId("C001".to_string())),
                channel_type: Some(SlackChannelType("channel".to_string())),
                thread_ts: None,
                client_msg_id: Some(SlackClientMessageId("hist-1".to_string())),
            },
            content: SlackMessageContent::new().with_text("Original message".to_string()),
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
        };
        let reaction_event = SlackReactionAddedEvent {
            user: SlackUserId("U999".to_string()),
            reaction: SlackReactionName("eyes".to_string()),
            item_user: Some(SlackUserId("U001".to_string())),
            item: SlackReactionsItem::Message(history_msg),
            event_ts: SlackTs("1700000000.123456".to_string()),
        };

        let incoming = incoming_from_reaction_event(&reaction_event).expect("reaction event");
        match incoming.kind {
            SlackIncomingKind::Reaction { emoji } => assert_eq!(emoji, "eyes"),
            _ => panic!("expected reaction kind"),
        }
        assert_eq!(incoming.channel_id, "C001");
        assert_eq!(incoming.user_id, "U999");
        assert!(incoming.text.contains("Reaction :eyes:"));
        assert!(incoming.text.contains("U999"));
        assert_eq!(incoming.thread_ts.0, "1700000000.000050");
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

    // ── Pending-message batch tests ──────────────────────────────────────────

    fn make_pending(user: &str, channel: &str, text: &str, ts: &str) -> PendingMessage {
        PendingMessage {
            text: text.to_string(),
            user_id: user.to_string(),
            channel_id: channel.to_string(),
            thread_ts: SlackTs("1700000000.000000".to_string()),
            msg_ts: SlackTs(ts.to_string()),
        }
    }

    #[test]
    fn batch_prompt_single_message() {
        let batch = vec![make_pending("U001", "C001", "hello world", "1.0")];
        let prompt = build_batch_prompt(&batch);
        assert_eq!(prompt, "[Slack: user=U001 channel=C001]\nhello world");
    }

    #[test]
    fn batch_prompt_multiple_messages_preserves_order_and_headers() {
        let batch = vec![
            make_pending("U001", "C001", "first", "1.0"),
            make_pending("U002", "C001", "second", "2.0"),
            make_pending("U001", "C001", "third", "3.0"),
        ];
        let prompt = build_batch_prompt(&batch);
        let expected = "\
[Slack: user=U001 channel=C001]\nfirst\n\n\
[Slack: user=U002 channel=C001]\nsecond\n\n\
[Slack: user=U001 channel=C001]\nthird";
        assert_eq!(prompt, expected);
    }

    #[test]
    fn batch_prompt_empty_batch_produces_empty_string() {
        let batch: Vec<PendingMessage> = vec![];
        let prompt = build_batch_prompt(&batch);
        assert!(prompt.is_empty(), "empty batch must produce empty string");
    }

    #[tokio::test]
    async fn pending_queue_drain_returns_all_and_leaves_empty() {
        let pending: Arc<Mutex<HashMap<Uuid, Vec<PendingMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let conv_id = Uuid::new_v4();

        // Push three messages.
        {
            let mut q = pending.lock().await;
            let entry = q.entry(conv_id).or_default();
            entry.push(make_pending("U001", "C001", "msg-a", "1.0"));
            entry.push(make_pending("U002", "C001", "msg-b", "2.0"));
            entry.push(make_pending("U003", "C001", "msg-c", "3.0"));
        }

        // Drain — should get all three.
        let batch: Vec<PendingMessage> = {
            let mut q = pending.lock().await;
            q.remove(&conv_id).unwrap_or_default()
        };
        assert_eq!(batch.len(), 3, "drain must return all queued messages");
        assert_eq!(batch[0].text, "msg-a");
        assert_eq!(batch[1].text, "msg-b");
        assert_eq!(batch[2].text, "msg-c");

        // Second drain — must be empty (another handler would early-return).
        let second: Vec<PendingMessage> = {
            let mut q = pending.lock().await;
            q.remove(&conv_id).unwrap_or_default()
        };
        assert!(
            second.is_empty(),
            "second drain must return empty — messages already consumed"
        );
    }

    #[tokio::test]
    async fn pending_queue_separate_conversations_are_isolated() {
        let pending: Arc<Mutex<HashMap<Uuid, Vec<PendingMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let conv_a = Uuid::new_v4();
        let conv_b = Uuid::new_v4();

        {
            let mut q = pending.lock().await;
            q.entry(conv_a)
                .or_default()
                .push(make_pending("U001", "C001", "for-a", "1.0"));
            q.entry(conv_b)
                .or_default()
                .push(make_pending("U002", "C002", "for-b", "2.0"));
        }

        // Drain conv_a — must only get conv_a's message.
        let batch_a: Vec<PendingMessage> = {
            let mut q = pending.lock().await;
            q.remove(&conv_a).unwrap_or_default()
        };
        assert_eq!(batch_a.len(), 1);
        assert_eq!(batch_a[0].text, "for-a");

        // conv_b must still be intact.
        let batch_b: Vec<PendingMessage> = {
            let mut q = pending.lock().await;
            q.remove(&conv_b).unwrap_or_default()
        };
        assert_eq!(batch_b.len(), 1);
        assert_eq!(batch_b[0].text, "for-b");
    }
}
