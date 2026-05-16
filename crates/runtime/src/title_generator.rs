//! Background worker that auto-generates conversation titles.
//!
//! Consumes `turn.result` envelopes from the message bus.  For each
//! eligible conversation (unlocked, threshold met, titling enabled) it
//! asks the LLM to produce a short title and persists it via
//! `ConversationStore::update_title`, which transitively broadcasts the
//! new title to reactive UIs.
//!
//! The worker is intentionally separated from the orchestrator so that:
//!  - title generation does not block turn latency,
//!  - it survives process restarts via the bus's at-least-once delivery,
//!  - the same code path serves every interface (web, messengers, CLI, MCP).

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow};
use assistant_core::bus::{ClaimFilter, MessageBus};
use assistant_core::bus_messages;
use assistant_core::types::features::TitlingConfig;
use assistant_core::{ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse};
use assistant_storage::StorageLayer;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Hard upper bound on the title returned by the worker, regardless of what
/// the model emits.  Matches the column intent (short, glanceable).
const MAX_TITLE_CHARS: usize = 60;

/// Maximum number of characters of conversation history fed to the title model.
/// Keeps the prompt cheap on Haiku-class models and stays well below any
/// provider context limit.
const MAX_HISTORY_CHARS: usize = 2_000;

const TITLE_SYSTEM_PROMPT: &str = "You write very short conversation titles. Given the start of a chat, \
     produce a single descriptive title in 6 words or fewer (max 60 characters). \
     Respond with only the title text — no quotes, no markdown, no trailing \
     punctuation, no prefixes like \"Title:\".";

/// Ask `llm` to summarise `history` into a short conversation title.
///
/// Post-processing:
///  - trim surrounding whitespace
///  - strip a single pair of wrapping quotes (curly or straight)
///  - drop a trailing `.` if present
///  - cap to [`MAX_TITLE_CHARS`] characters
///
/// Returns `Err` if the provider fails or returns a non-text variant.
pub(crate) async fn generate_title(
    history: &[ChatHistoryMessage],
    llm: &dyn LlmProvider,
) -> Result<String> {
    let prompt_history = build_title_prompt_history(history);

    let response = llm
        .chat(TITLE_SYSTEM_PROMPT, &prompt_history, &[])
        .await
        .context("title LLM call failed")?;

    let raw = match response {
        LlmResponse::FinalAnswer(text, _) => text,
        LlmResponse::ToolCalls(_) | LlmResponse::Thinking(_, _) => {
            return Err(anyhow!(
                "title LLM returned non-FinalAnswer variant; refusing to persist"
            ));
        }
    };

    let cleaned = clean_title(&raw);
    if cleaned.is_empty() {
        return Err(anyhow!("title LLM returned empty/whitespace response"));
    }
    Ok(cleaned)
}

/// Pack the conversation history into the prompt for the title model.
///
/// Concatenates user/assistant text content blocks in order, prefixed by the
/// role so the model can see who said what, and truncates the total to
/// [`MAX_HISTORY_CHARS`] characters by trimming from the end.  Tool messages
/// and tool-call requests are skipped — titles should reflect user intent,
/// not internal mechanics.
fn build_title_prompt_history(history: &[ChatHistoryMessage]) -> Vec<ChatHistoryMessage> {
    let mut joined = String::new();
    for msg in history {
        let (label, text) = match msg {
            ChatHistoryMessage::Text { role, content } => {
                let label = match role {
                    ChatRole::User => "User",
                    ChatRole::Assistant => "Assistant",
                    ChatRole::System => continue,
                    ChatRole::Tool => continue,
                };
                (label, content.as_str())
            }
            ChatHistoryMessage::MultimodalUser { content } => {
                let mut text_only = String::new();
                for block in content {
                    if let assistant_core::llm::types::ContentBlock::Text(t) = block {
                        if !text_only.is_empty() {
                            text_only.push('\n');
                        }
                        text_only.push_str(t);
                    }
                }
                if text_only.is_empty() {
                    continue;
                }
                // Stash to avoid borrowing a local in the outer match arm.
                joined.push_str("User: ");
                push_truncated(&mut joined, &text_only);
                joined.push_str("\n\n");
                continue;
            }
            ChatHistoryMessage::AssistantToolCalls(_) | ChatHistoryMessage::ToolResult { .. } => {
                continue;
            }
        };
        joined.push_str(label);
        joined.push_str(": ");
        push_truncated(&mut joined, text);
        joined.push_str("\n\n");
    }

    let trimmed = if joined.chars().count() > MAX_HISTORY_CHARS {
        joined.chars().take(MAX_HISTORY_CHARS).collect::<String>()
    } else {
        joined
    };

    vec![ChatHistoryMessage::Text {
        role: ChatRole::User,
        content: format!(
            "Produce the title for this conversation:\n\n{}",
            trimmed.trim()
        ),
    }]
}

fn push_truncated(buf: &mut String, text: &str) {
    // Single-message cap: keep things readable in the prompt.
    let max = MAX_HISTORY_CHARS / 2;
    if text.chars().count() > max {
        buf.extend(text.chars().take(max));
        buf.push('…');
    } else {
        buf.push_str(text);
    }
}

/// Sanitize a raw LLM response into a usable title.
fn clean_title(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    // Strip a single matching pair of wrapping quotes (straight or curly).
    let pairs: &[(char, char)] = &[('"', '"'), ('\'', '\''), ('“', '”'), ('‘', '’')];
    for &(open, close) in pairs {
        if s.starts_with(open) && s.ends_with(close) && s.chars().count() >= 2 {
            s = s
                .strip_prefix(open)
                .and_then(|t| t.strip_suffix(close))
                .map(|t| t.to_string())
                .unwrap_or(s);
            break;
        }
    }
    // Strip a common "Title:" prefix the model might emit despite the prompt.
    if let Some(rest) = s
        .strip_prefix("Title:")
        .or_else(|| s.strip_prefix("title:"))
    {
        s = rest.trim().to_string();
    }
    // Strip a single trailing period — titles shouldn't carry punctuation.
    if s.ends_with('.') && !s.ends_with("..") {
        s.pop();
    }
    // Hard cap on length.
    if s.chars().count() > MAX_TITLE_CHARS {
        s = s.chars().take(MAX_TITLE_CHARS).collect();
    }
    s.trim().to_string()
}

/// Spawn the title-generator worker as a background task.
///
/// Returns a [`tokio::task::JoinHandle`] for the loop.  Mirrors the
/// signature of [`crate::bootstrap::spawn_memory_indexer`] so callers
/// (CLI, web-ui binary) integrate it the same way.
pub fn spawn_title_generator_worker(
    bus: Arc<dyn MessageBus>,
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
    config: TitlingConfig,
    agent_id: impl Into<String>,
) -> tokio::task::JoinHandle<()> {
    let worker = TitleGeneratorWorker::new(bus, storage, llm, config, agent_id.into());
    tokio::spawn(async move { worker.run().await })
}

/// Backoff schedule for transient LLM failures.  Mirrors the orchestrator
/// worker's spirit: a small ladder so a flaky provider doesn't hot-loop.
const RETRY_BACKOFF: &[Duration] = &[
    Duration::from_secs(30),
    Duration::from_secs(60),
    Duration::from_secs(120),
    Duration::from_secs(240),
];

/// Bus-consumer worker that auto-titles conversations.
///
/// One worker is scoped to a single `agent_id` so the claim filter keeps
/// messengers' personas from stealing each other's `turn.result` events.
pub struct TitleGeneratorWorker {
    bus: Arc<dyn MessageBus>,
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
    config: TitlingConfig,
    agent_id: String,
}

impl TitleGeneratorWorker {
    pub fn new(
        bus: Arc<dyn MessageBus>,
        storage: Arc<StorageLayer>,
        llm: Arc<dyn LlmProvider>,
        config: TitlingConfig,
        agent_id: String,
    ) -> Self {
        Self {
            bus,
            storage,
            llm,
            config,
            agent_id,
        }
    }

    /// Stable worker identity used for bus claim attribution.
    ///
    /// The id is embedded in the NATS JetStream consumer name (via
    /// `bus-nats::consumer_for`, which does
    /// `format!("{worker_id}-{topic}")`). NATS consumer names only accept
    /// `[A-Za-z0-9_-]`, so this method MUST avoid slashes and other special
    /// characters — historically `title-generator/<agent_id>` produced an
    /// invalid consumer name that `get_or_create_consumer` rejected every
    /// poll, surfacing as a per-second `claim failed` WARN in production.
    fn worker_id(&self) -> String {
        format!("title-generator-{}", self.agent_id)
    }

    /// Run a single claim → process → ack/nack iteration.
    ///
    /// Returns `Ok(())` whether or not a message was claimed.  Provider
    /// errors are absorbed (the message is `nack_delayed`ed so the bus can
    /// redeliver); only programming bugs or storage failures bubble up.
    pub async fn run_one(&self) -> Result<()> {
        let filter = ClaimFilter::new().with_agent_id(self.agent_id.clone());
        let claimed = self
            .bus
            .claim_filtered(bus_messages::topic::TURN_RESULT, &self.worker_id(), &filter)
            .await
            .context("title-generator: claim failed")?;

        let Some(msg) = claimed else {
            return Ok(());
        };

        let payload: bus_messages::TurnResult = match serde_json::from_value(msg.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    msg_id = %msg.id,
                    error = %e,
                    "title-generator: failed to deserialize TurnResult; dropping message",
                );
                let _ = self.bus.fail(msg.id).await;
                return Ok(());
            }
        };

        let conv_id = payload.conversation_id;
        let store = self.storage.conversation_store_for_agent(&self.agent_id);

        let conv = match store.get_conversation(conv_id).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                debug!(
                    %conv_id,
                    "title-generator: conversation not found for this agent; skipping",
                );
                let _ = self.bus.ack(msg.id).await;
                return Ok(());
            }
            Err(e) => {
                warn!(%conv_id, error = %e, "title-generator: get_conversation failed");
                let _ = self.bus.nack_delayed(msg.id, RETRY_BACKOFF[0]).await;
                return Ok(());
            }
        };

        let history = match store.load_history(conv_id).await {
            Ok(h) => h,
            Err(e) => {
                warn!(%conv_id, error = %e, "title-generator: load_history failed");
                let _ = self.bus.nack_delayed(msg.id, RETRY_BACKOFF[0]).await;
                return Ok(());
            }
        };
        let first_user_len = history
            .iter()
            .find(|m| {
                matches!(
                    m.role,
                    assistant_core::types::conversation::MessageRole::User
                )
            })
            .map(|m| m.content.chars().count())
            .unwrap_or(0);

        if !is_eligible(
            payload.turn,
            first_user_len,
            &self.config,
            conv.title_locked,
        ) {
            debug!(
                %conv_id,
                turn = payload.turn,
                title_locked = conv.title_locked,
                "title-generator: conversation not eligible; acking",
            );
            let _ = self.bus.ack(msg.id).await;
            return Ok(());
        }

        let chat_history = history
            .iter()
            .map(message_to_chat_history)
            .collect::<Vec<_>>();

        match generate_title(&chat_history, self.llm.as_ref()).await {
            Ok(title) => {
                if let Err(e) = store.update_title(conv_id, &title).await {
                    warn!(%conv_id, error = %e, "title-generator: update_title failed");
                    let _ = self.bus.nack_delayed(msg.id, RETRY_BACKOFF[0]).await;
                    return Ok(());
                }
                info!(%conv_id, title = %title, "title-generator: auto-titled conversation");
                let _ = self.bus.ack(msg.id).await;
            }
            Err(e) => {
                warn!(%conv_id, error = %e, "title-generator: LLM call failed; nacking for redelivery");
                let delay = backoff_for_delivery(msg.delivery_count);
                let _ = self.bus.nack_delayed(msg.id, delay).await;
            }
        }
        Ok(())
    }

    /// Long-running loop: claim, process, ack/nack in a tight cycle with a
    /// short sleep when the topic is empty so we don't spin the CPU.
    pub async fn run(&self) {
        info!(worker_id = %self.worker_id(), "Title-generator worker started");
        loop {
            if let Err(e) = self.run_one().await {
                // `{:#}` walks the full anyhow context chain — without this,
                // operators only see the outermost context ("claim failed")
                // and can't tell what the underlying bus / provider failure
                // actually was.
                warn!(
                    error = format!("{e:#}"),
                    "title-generator: run_one returned an error"
                );
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            // Cheap poll back-off when the topic is empty.
            sleep(Duration::from_millis(500)).await;
        }
    }
}

fn backoff_for_delivery(delivery: u32) -> Duration {
    let idx = (delivery.saturating_sub(1) as usize).min(RETRY_BACKOFF.len() - 1);
    RETRY_BACKOFF[idx]
}

fn message_to_chat_history(m: &assistant_core::types::conversation::Message) -> ChatHistoryMessage {
    let role = match m.role {
        assistant_core::types::conversation::MessageRole::User => ChatRole::User,
        assistant_core::types::conversation::MessageRole::Assistant => ChatRole::Assistant,
        assistant_core::types::conversation::MessageRole::System => ChatRole::System,
        assistant_core::types::conversation::MessageRole::Tool => ChatRole::Tool,
    };
    ChatHistoryMessage::Text {
        role,
        content: m.content.clone(),
    }
}

/// Decide whether a conversation is eligible for an auto-title.
///
/// The check is intentionally pure and synchronous so the worker can run it
/// before any LLM round-trip.
///
/// # Parameters
/// - `turn` — the turn number from the just-completed `turn.result`.
/// - `first_message_len` — character count of the first user message in
///   the conversation.  Used by the long-first-message escape hatch.
/// - `cfg` — the per-org titling configuration.
/// - `title_locked` — current value of `conversations.title_locked` for
///   this conversation; once `true`, the worker must never overwrite.
pub(crate) fn is_eligible(
    turn: i64,
    first_message_len: usize,
    cfg: &TitlingConfig,
    title_locked: bool,
) -> bool {
    if !cfg.enabled {
        return false;
    }
    if title_locked {
        return false;
    }
    if turn >= cfg.min_turns {
        return true;
    }
    if turn >= 1 && first_message_len > cfg.long_first_message_chars {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::llm::provider::{Capabilities, ToolSupport};
    use assistant_core::llm::stream_chunk::StreamChunk;
    use assistant_core::llm::tool_spec::ToolSpec;
    use assistant_core::llm::types::LlmResponseMeta;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::mpsc;

    fn default_cfg() -> TitlingConfig {
        TitlingConfig::default()
    }

    /// Mock LLM provider for title-generator tests. Returns a configured
    /// response (or error) and counts invocations so tests can assert the
    /// worker called or skipped the LLM.
    struct MockLlm {
        response: Result<String, ()>,
        chat_calls: AtomicUsize,
    }

    impl MockLlm {
        fn new_ok(response: impl Into<String>) -> Self {
            Self {
                response: Ok(response.into()),
                chat_calls: AtomicUsize::new(0),
            }
        }

        fn new_err() -> Self {
            Self {
                response: Err(()),
                chat_calls: AtomicUsize::new(0),
            }
        }

        fn chat_calls(&self) -> usize {
            self.chat_calls.load(Ordering::SeqCst)
        }
    }

    /// Regression test: the title-generator's `worker_id()` is embedded in
    /// the NATS JetStream consumer name via
    /// `bus-nats::consumer_for` (`format!("{worker_id}-{topic}")`). NATS
    /// rejects any consumer name containing characters outside
    /// `[A-Za-z0-9_-]`. The original implementation used a slash separator
    /// (`title-generator/<agent_id>`) which silently failed every poll,
    /// producing the per-second `claim failed` WARN spam observed on
    /// schorschvm. Lock the invariant in.
    #[tokio::test]
    async fn worker_id_is_nats_safe() {
        let (bus, storage) = make_bus_and_storage().await;
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new_ok("ignored"));

        // Realistic agent_ids the system actually generates. Non-ASCII
        // edge cases (e.g. user-supplied `team-α`) are covered by
        // `bus-nats::sanitize_consumer_segment` instead — title-gen's own
        // output just needs to be safe for the ASCII-only agent_ids the
        // runtime currently mints.
        for agent_id in &["default", "work", "ops-team", "agent_42"] {
            let worker = TitleGeneratorWorker::new(
                bus.clone(),
                storage.clone(),
                llm.clone(),
                default_cfg(),
                (*agent_id).to_string(),
            );
            let id = worker.worker_id();

            // Whole id, plus the joined `<id>-<topic>` consumer name, must
            // match `[A-Za-z0-9_-]+`.
            let joined = format!("{id}-turn-result");
            assert!(
                joined
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "worker_id-derived consumer name {joined:?} contains characters \
                 NATS rejects; only [A-Za-z0-9_-] is allowed"
            );
            assert!(
                !id.contains('/'),
                "worker_id {id:?} contains a slash — NATS consumer names cannot include `/`"
            );
        }
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                tools: ToolSupport::None,
                streaming: false,
                vision: false,
                hosted_tools: vec![],
                context_window_tokens: None,
            }
        }

        async fn chat(
            &self,
            _system_prompt: &str,
            _history: &[ChatHistoryMessage],
            _tools: &[ToolSpec],
        ) -> Result<LlmResponse> {
            self.chat_calls.fetch_add(1, Ordering::SeqCst);
            match &self.response {
                Ok(text) => Ok(LlmResponse::FinalAnswer(
                    text.clone(),
                    LlmResponseMeta::default(),
                )),
                Err(()) => Err(anyhow!("mock LLM error")),
            }
        }

        async fn chat_streaming(
            &self,
            system_prompt: &str,
            history: &[ChatHistoryMessage],
            tools: &[ToolSpec],
            _sink: Option<mpsc::Sender<StreamChunk>>,
        ) -> Result<LlmResponse> {
            self.chat(system_prompt, history, tools).await
        }
    }

    fn sample_history() -> Vec<ChatHistoryMessage> {
        vec![
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "I'm thinking about adding real-time collaboration to my app".into(),
            },
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "Real-time collab covers a spectrum from presence to full CRDT sync. Where's your head at?".into(),
            },
        ]
    }

    #[tokio::test]
    async fn test_generate_title_returns_trimmed_short_string() {
        let llm: Arc<dyn LlmProvider> =
            Arc::new(MockLlm::new_ok("  \"Real-time Collaboration Design\"  \n"));
        let title = generate_title(&sample_history(), llm.as_ref())
            .await
            .expect("generate_title should succeed on happy path");
        assert!(!title.is_empty(), "title must not be empty");
        assert!(
            title.chars().count() <= MAX_TITLE_CHARS,
            "title must fit within {MAX_TITLE_CHARS} characters; got {} chars: {title:?}",
            title.chars().count()
        );
        assert!(
            !title.starts_with('"') && !title.ends_with('"'),
            "wrapping quotes must be stripped: {title:?}"
        );
        assert!(
            title == title.trim(),
            "surrounding whitespace must be trimmed: {title:?}"
        );
    }

    #[tokio::test]
    async fn test_generate_title_truncates_very_long_responses() {
        // Even if the model ignores the prompt and emits a paragraph, the
        // worker must cap the title to MAX_TITLE_CHARS.
        let long = "x".repeat(500);
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new_ok(long));
        let title = generate_title(&sample_history(), llm.as_ref())
            .await
            .unwrap();
        assert_eq!(title.chars().count(), MAX_TITLE_CHARS);
    }

    #[tokio::test]
    async fn test_generate_title_handles_llm_error() {
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new_err());
        let result = generate_title(&sample_history(), llm.as_ref()).await;
        assert!(
            result.is_err(),
            "provider error must propagate as Err — got {result:?}"
        );
    }

    // ---------------------------------------------------------------
    // Worker integration tests (in-memory bus + storage)
    // ---------------------------------------------------------------

    use assistant_core::bus::{MessageBus, PublishRequest};
    use assistant_core::bus_messages;
    use assistant_core::types::conversation::Message as CoreMessage;
    use assistant_storage::StorageLayer;
    use assistant_storage::message_bus::SqliteMessageBus;

    async fn make_bus_and_storage() -> (Arc<dyn MessageBus>, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let bus: Arc<dyn MessageBus> = Arc::new(SqliteMessageBus::new(storage.pool.clone()));
        (bus, storage)
    }

    async fn seed_conversation_with_messages(
        storage: &StorageLayer,
        agent_id: &str,
        user_msgs: &[&str],
        assistant_msgs: &[&str],
    ) -> uuid::Uuid {
        let store = storage.conversation_store_for_agent(agent_id);
        let conv = store.create_conversation(None).await.unwrap();
        let mut turn: i64 = 1;
        for (i, m) in user_msgs.iter().enumerate() {
            let mut msg = CoreMessage::user(conv.id, *m);
            msg.turn = turn;
            store.save_message(&msg).await.unwrap();
            turn += 1;
            if let Some(a) = assistant_msgs.get(i) {
                let mut reply = CoreMessage::assistant(conv.id, *a);
                reply.turn = turn;
                store.save_message(&reply).await.unwrap();
                turn += 1;
            }
        }
        conv.id
    }

    async fn publish_turn_result(
        bus: &dyn MessageBus,
        conv_id: uuid::Uuid,
        agent_id: &str,
        turn: i64,
    ) {
        publish_turn_result_with_interface(bus, conv_id, agent_id, turn, None).await
    }

    async fn publish_turn_result_with_interface(
        bus: &dyn MessageBus,
        conv_id: uuid::Uuid,
        agent_id: &str,
        turn: i64,
        interface: Option<&str>,
    ) {
        let payload = serde_json::to_value(&bus_messages::TurnResult {
            conversation_id: conv_id,
            content: "assistant reply".into(),
            turn,
            attachment_ids: vec![],
            message_id: None,
            had_errors: false,
        })
        .unwrap();
        let mut req = PublishRequest::new(bus_messages::topic::TURN_RESULT, payload)
            .with_agent_id(agent_id)
            .with_conversation_id(conv_id);
        if let Some(iface) = interface {
            req = req.with_interface(iface);
        }
        bus.publish(req).await.unwrap();
    }

    #[tokio::test]
    async fn test_worker_titles_eligible_conversation() {
        let (bus, storage) = make_bus_and_storage().await;
        let agent_id = "default";
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new_ok("Dinner Brainstorm"));

        let conv_id = seed_conversation_with_messages(
            &storage,
            agent_id,
            &["What should we cook?", "Something healthy"],
            &["I have ideas", "Like grilled chicken"],
        )
        .await;
        let turn = 2;

        publish_turn_result(bus.as_ref(), conv_id, agent_id, turn).await;

        let worker = TitleGeneratorWorker::new(
            bus.clone(),
            storage.clone(),
            llm.clone(),
            TitlingConfig::default(),
            agent_id.to_string(),
        );
        worker.run_one().await.expect("run_one should not error");

        let store = storage.conversation_store_for_agent(agent_id);
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert_eq!(conv.title.as_deref(), Some("Dinner Brainstorm"));
        assert!(conv.title_locked, "worker must lock the title");
    }

    #[tokio::test]
    async fn test_worker_skips_locked_conversation() {
        let (bus, storage) = make_bus_and_storage().await;
        let agent_id = "default";
        let llm = Arc::new(MockLlm::new_ok("Should Never Be Used"));
        let llm_dyn: Arc<dyn LlmProvider> = llm.clone();

        // Lock the conversation up-front.
        let store = storage.conversation_store_for_agent(agent_id);
        let conv = store
            .create_conversation(Some("User Picked This"))
            .await
            .unwrap();
        // Add enough history to exceed min_turns.
        let mut m1 = CoreMessage::user(conv.id, "hi");
        m1.turn = 1;
        store.save_message(&m1).await.unwrap();
        let mut m2 = CoreMessage::assistant(conv.id, "hello");
        m2.turn = 2;
        store.save_message(&m2).await.unwrap();

        publish_turn_result(bus.as_ref(), conv.id, agent_id, 2).await;

        let worker = TitleGeneratorWorker::new(
            bus.clone(),
            storage.clone(),
            llm_dyn,
            TitlingConfig::default(),
            agent_id.to_string(),
        );
        worker.run_one().await.expect("run_one should not error");

        assert_eq!(llm.chat_calls(), 0, "must not call LLM for locked conv");
        let loaded = store.get_conversation(conv.id).await.unwrap().unwrap();
        assert_eq!(loaded.title.as_deref(), Some("User Picked This"));
    }

    #[tokio::test]
    async fn test_worker_skips_below_threshold() {
        let (bus, storage) = make_bus_and_storage().await;
        let agent_id = "default";
        let llm = Arc::new(MockLlm::new_ok("Should Not Title Yet"));
        let llm_dyn: Arc<dyn LlmProvider> = llm.clone();

        // First-turn, short first message → ineligible under defaults.
        let conv_id =
            seed_conversation_with_messages(&storage, agent_id, &["hi"], &["hello"]).await;
        publish_turn_result(bus.as_ref(), conv_id, agent_id, 1).await;

        let worker = TitleGeneratorWorker::new(
            bus.clone(),
            storage.clone(),
            llm_dyn,
            TitlingConfig::default(),
            agent_id.to_string(),
        );
        worker.run_one().await.expect("run_one should not error");

        assert_eq!(
            llm.chat_calls(),
            0,
            "must not call LLM below the eligibility threshold"
        );
        let store = storage.conversation_store_for_agent(agent_id);
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert!(conv.title.is_none());
        assert!(!conv.title_locked);
    }

    #[tokio::test]
    async fn test_worker_idempotent_on_redelivery() {
        let (bus, storage) = make_bus_and_storage().await;
        let agent_id = "default";
        let llm = Arc::new(MockLlm::new_ok("First Title"));
        let llm_dyn: Arc<dyn LlmProvider> = llm.clone();

        let conv_id = seed_conversation_with_messages(
            &storage,
            agent_id,
            &["help me build a feature", "what kind"],
            &["sure, what?", "OK"],
        )
        .await;

        publish_turn_result(bus.as_ref(), conv_id, agent_id, 2).await;
        let worker = TitleGeneratorWorker::new(
            bus.clone(),
            storage.clone(),
            llm_dyn,
            TitlingConfig::default(),
            agent_id.to_string(),
        );
        worker.run_one().await.unwrap();
        assert_eq!(llm.chat_calls(), 1, "first delivery titles");

        // Simulate redelivery: publish the same payload again.
        publish_turn_result(bus.as_ref(), conv_id, agent_id, 2).await;
        worker.run_one().await.unwrap();
        assert_eq!(
            llm.chat_calls(),
            1,
            "redelivery must observe title_locked and skip LLM"
        );

        let store = storage.conversation_store_for_agent(agent_id);
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert_eq!(conv.title.as_deref(), Some("First Title"));
    }

    #[tokio::test]
    async fn test_worker_skips_when_disabled() {
        let (bus, storage) = make_bus_and_storage().await;
        let agent_id = "default";
        let llm = Arc::new(MockLlm::new_ok("Never"));
        let llm_dyn: Arc<dyn LlmProvider> = llm.clone();

        let conv_id =
            seed_conversation_with_messages(&storage, agent_id, &["help", "more"], &["yes", "ok"])
                .await;
        publish_turn_result(bus.as_ref(), conv_id, agent_id, 2).await;

        let cfg = TitlingConfig {
            enabled: false,
            ..Default::default()
        };
        let worker = TitleGeneratorWorker::new(
            bus.clone(),
            storage.clone(),
            llm_dyn,
            cfg,
            agent_id.to_string(),
        );
        worker.run_one().await.unwrap();

        assert_eq!(llm.chat_calls(), 0, "disabled config must skip LLM");
        let store = storage.conversation_store_for_agent(agent_id);
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert!(conv.title.is_none());
    }

    #[tokio::test]
    async fn test_worker_titles_across_every_interface() {
        // Property: the title-generator must title conversations from CLI,
        // Web, Slack, Matrix, MCP, Scheduler, etc. — anything that publishes
        // `turn.result` for the configured agent.  We exercise each interface
        // label and assert the worker doesn't filter any of them out.
        let (bus, storage) = make_bus_and_storage().await;
        let agent_id = "default";
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new_ok("Cross-interface Title"));

        let interfaces = &[
            "Cli",
            "Web",
            "Slack",
            "Matrix",
            "Mattermost",
            "Mcp",
            "Scheduler",
        ];
        let mut conv_ids = Vec::new();
        for iface in interfaces {
            let conv_id = seed_conversation_with_messages(
                &storage,
                agent_id,
                &["start", "more"],
                &["ok", "ok"],
            )
            .await;
            publish_turn_result_with_interface(bus.as_ref(), conv_id, agent_id, 2, Some(iface))
                .await;
            conv_ids.push((conv_id, *iface));
        }

        let worker = TitleGeneratorWorker::new(
            bus.clone(),
            storage.clone(),
            llm,
            TitlingConfig::default(),
            agent_id.to_string(),
        );
        for _ in 0..interfaces.len() {
            worker.run_one().await.unwrap();
        }

        let store = storage.conversation_store_for_agent(agent_id);
        for (id, iface) in &conv_ids {
            let conv = store.get_conversation(*id).await.unwrap().unwrap();
            assert_eq!(
                conv.title.as_deref(),
                Some("Cross-interface Title"),
                "conversation from interface={iface} was not titled",
            );
            assert!(conv.title_locked);
        }
    }

    #[tokio::test]
    async fn test_spawned_worker_titles_a_conversation_end_to_end() {
        let (bus, storage) = make_bus_and_storage().await;
        let agent_id = "default";
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new_ok("Spawned Title"));

        let conv_id = seed_conversation_with_messages(
            &storage,
            agent_id,
            &["a question", "more"],
            &["an answer", "yes"],
        )
        .await;

        let handle = spawn_title_generator_worker(
            bus.clone(),
            storage.clone(),
            llm,
            TitlingConfig::default(),
            agent_id,
        );

        // Publish AFTER spawn to exercise the live claim loop.
        publish_turn_result(bus.as_ref(), conv_id, agent_id, 2).await;

        // The worker polls every 500ms; give it a generous window.
        let store = storage.conversation_store_for_agent(agent_id);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let title = loop {
            if let Ok(Some(conv)) = store.get_conversation(conv_id).await
                && conv.title.is_some()
            {
                break conv.title;
            }
            if std::time::Instant::now() > deadline {
                panic!("worker did not title within 5s");
            }
            sleep(Duration::from_millis(50)).await;
        };
        assert_eq!(title.as_deref(), Some("Spawned Title"));
        handle.abort();
    }

    #[tokio::test]
    async fn test_titling_disabled_org_skips_llm() {
        // Configuration with enabled = false. Spawn the worker, publish, wait,
        // confirm no LLM call and no title.
        let (bus, storage) = make_bus_and_storage().await;
        let agent_id = "default";
        let llm_counter = Arc::new(MockLlm::new_ok("Never"));
        let llm: Arc<dyn LlmProvider> = llm_counter.clone();

        let conv_id =
            seed_conversation_with_messages(&storage, agent_id, &["x", "y"], &["a", "b"]).await;

        let cfg = TitlingConfig {
            enabled: false,
            ..Default::default()
        };
        let handle = spawn_title_generator_worker(bus.clone(), storage.clone(), llm, cfg, agent_id);

        publish_turn_result(bus.as_ref(), conv_id, agent_id, 2).await;
        sleep(Duration::from_millis(1200)).await;

        assert_eq!(
            llm_counter.chat_calls(),
            0,
            "disabled config must skip LLM even when running as a live worker"
        );
        let store = storage.conversation_store_for_agent(agent_id);
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert!(conv.title.is_none());
        handle.abort();
    }

    #[tokio::test]
    async fn test_worker_nack_on_transient_error_keeps_title_null() {
        let (bus, storage) = make_bus_and_storage().await;
        let agent_id = "default";
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new_err());

        let conv_id =
            seed_conversation_with_messages(&storage, agent_id, &["a", "b"], &["c", "d"]).await;
        publish_turn_result(bus.as_ref(), conv_id, agent_id, 2).await;

        let worker = TitleGeneratorWorker::new(
            bus.clone(),
            storage.clone(),
            llm,
            TitlingConfig::default(),
            agent_id.to_string(),
        );
        // The worker should not return Err from run_one — it absorbs the error
        // and nacks the message so the bus can redeliver later.
        worker
            .run_one()
            .await
            .expect("transient LLM error must be handled inside run_one");

        let store = storage.conversation_store_for_agent(agent_id);
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert!(conv.title.is_none(), "title must remain NULL on error");
        assert!(!conv.title_locked, "lock must remain false on error");
    }

    #[tokio::test]
    async fn test_generate_title_rejects_unexpected_response_variants() {
        // A ToolCalls or Thinking response is not a title; the worker must
        // refuse to persist garbage.
        struct ThinkingLlm;
        #[async_trait]
        impl LlmProvider for ThinkingLlm {
            fn capabilities(&self) -> Capabilities {
                Capabilities {
                    tools: ToolSupport::None,
                    streaming: false,
                    vision: false,
                    hosted_tools: vec![],
                    context_window_tokens: None,
                }
            }
            async fn chat(
                &self,
                _: &str,
                _: &[ChatHistoryMessage],
                _: &[ToolSpec],
            ) -> Result<LlmResponse> {
                Ok(LlmResponse::Thinking(
                    "hmm".into(),
                    LlmResponseMeta::default(),
                ))
            }
            async fn chat_streaming(
                &self,
                a: &str,
                b: &[ChatHistoryMessage],
                c: &[ToolSpec],
                _: Option<mpsc::Sender<StreamChunk>>,
            ) -> Result<LlmResponse> {
                self.chat(a, b, c).await
            }
        }
        let llm: Arc<dyn LlmProvider> = Arc::new(ThinkingLlm);
        let result = generate_title(&sample_history(), llm.as_ref()).await;
        assert!(result.is_err(), "non-FinalAnswer responses must Err");
    }

    #[test]
    fn test_eligibility_below_threshold_is_false() {
        assert!(!is_eligible(1, 50, &default_cfg(), false));
    }

    #[test]
    fn test_eligibility_at_or_above_min_turns_is_true() {
        assert!(is_eligible(2, 50, &default_cfg(), false));
        assert!(is_eligible(5, 50, &default_cfg(), false));
    }

    #[test]
    fn test_eligibility_long_first_message_after_turn_one_is_true() {
        // Above the default 200-char threshold.
        assert!(is_eligible(1, 300, &default_cfg(), false));
    }

    #[test]
    fn test_eligibility_locked_is_always_false() {
        // Even with maximally permissive inputs, the lock wins.
        assert!(!is_eligible(100, 9999, &default_cfg(), true));
    }

    #[test]
    fn test_eligibility_disabled_is_always_false() {
        let cfg = TitlingConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!is_eligible(100, 9999, &cfg, false));
        assert!(!is_eligible(2, 300, &cfg, false));
    }

    #[test]
    fn test_eligibility_respects_custom_min_turns() {
        let cfg = TitlingConfig {
            min_turns: 4,
            ..Default::default()
        };
        assert!(!is_eligible(3, 50, &cfg, false));
        assert!(is_eligible(4, 50, &cfg, false));
    }

    #[test]
    fn test_eligibility_respects_custom_long_first_message_chars() {
        let cfg = TitlingConfig {
            long_first_message_chars: 500,
            ..Default::default()
        };
        // 300 chars no longer qualifies under the higher threshold.
        assert!(!is_eligible(1, 300, &cfg, false));
        assert!(is_eligible(1, 600, &cfg, false));
    }
}
