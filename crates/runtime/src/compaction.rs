//! Context compaction — automatic history truncation when the context window
//! approaches its limit.
//!
//! When the LLM reports that `input_tokens` has crossed the soft threshold
//! (`context_window - reserve_floor - soft_threshold`), the orchestrator
//! calls [`maybe_compact`] which:
//!
//! 1. Asks the LLM to produce a short summary of the conversation so far.
//! 2. Replaces the in-memory `history` with a single summary turn followed
//!    by the `keep_recent_turns` most recent turns.
//! 3. Persists the compacted representation back to storage so compaction
//!    survives turn boundaries.
//!
//! If the old-turns transcript is larger than [`CHUNK_CHARS`], the history is
//! summarised in chunks before being merged into a final summary, preventing
//! the summary request itself from overflowing the context window.
//!
//! This mirrors OpenClaw's `contextWindow - reserveFloor(20k) - softThreshold(4k)`
//! trigger logic.

use std::sync::Arc;

use anyhow::Result;
use assistant_core::types::features::CompactionConfig;
use assistant_core::{
    ChatHistoryMessage, ChatRole, ContentBlock, LlmProvider, LlmResponse, ToolCallItem,
    types::conversation::Message, types::conversation::MessageRole,
};
use assistant_storage::conversations::ConversationStore;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Maximum characters to include in a single summarisation request.
/// Transcripts larger than this are summarised in chunks first.
const CHUNK_CHARS: usize = 12_000;

/// Returns `true` when the reported `input_tokens` has crossed the soft
/// compaction threshold defined by `cfg`.
pub fn should_compact(input_tokens: u64, cfg: &CompactionConfig) -> bool {
    if !cfg.enabled {
        return false;
    }
    let trigger = cfg
        .context_window_tokens
        .saturating_sub(cfg.reserve_floor_tokens)
        .saturating_sub(cfg.soft_threshold_tokens);
    input_tokens >= trigger
}

/// Estimate the number of tokens in `history` using a simple 4-chars-per-token
/// heuristic.  This is used as a pre-call fallback when the provider has not
/// yet returned token metadata.
pub fn estimate_tokens(history: &[ChatHistoryMessage]) -> u64 {
    let chars: usize = history.iter().map(message_estimated_chars).sum();
    (chars / 4) as u64
}

/// Estimate total input tokens including the system prompt and tool specs.
///
/// The basic `estimate_tokens` only counts history characters. The actual
/// request also includes the system prompt and tool definitions, which can
/// add tens of thousands of tokens. This function provides a more accurate
/// pre-call estimate.
pub fn estimate_total_tokens(
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    tool_specs: &[assistant_core::ToolSpec],
) -> u64 {
    let history_chars: usize = history.iter().map(message_estimated_chars).sum();
    let system_chars = system_prompt.len();
    let tool_chars: usize = tool_specs
        .iter()
        .map(|t| t.name.len() + t.description.len() + t.params_schema.to_string().len())
        .sum();
    ((history_chars + system_chars + tool_chars) / 4) as u64
}

/// Hard-truncate history by dropping the oldest messages when the estimated
/// token count exceeds the hard limit (`context_window - reserve_floor`).
///
/// Unlike soft compaction (which summarises old turns via an LLM call), this
/// is a synchronous, zero-cost safety net that prevents sending an
/// over-limit request. It drops whole messages from the front of the
/// history until the estimate fits.
///
/// Returns `true` if any messages were dropped.
pub fn hard_truncate(
    system_prompt: &str,
    history: &mut Vec<ChatHistoryMessage>,
    tool_specs: &[assistant_core::ToolSpec],
    cfg: &CompactionConfig,
) -> bool {
    let hard_limit = cfg
        .context_window_tokens
        .saturating_sub(cfg.reserve_floor_tokens);

    if hard_limit == 0 {
        return false;
    }

    let initial_len = history.len();
    while history.len() > 1 {
        let estimated = estimate_total_tokens(system_prompt, history, tool_specs);
        if estimated < hard_limit {
            break;
        }
        // Drop the oldest message.
        history.remove(0);
    }

    let dropped = history.len() < initial_len;
    if dropped {
        let n = initial_len - history.len();
        warn!(
            dropped_messages = n,
            hard_limit, "Hard-truncated {n} oldest messages to fit within context window"
        );
    }
    dropped
}

/// Estimate character count for a message, including binary payload sizes for
/// image/document blocks that `message_text` would skip.
fn message_estimated_chars(msg: &ChatHistoryMessage) -> usize {
    match msg {
        ChatHistoryMessage::Text { content, .. } => content.len(),
        ChatHistoryMessage::MultimodalUser { content } => content
            .iter()
            .map(|b| match b {
                ContentBlock::Text(t) => t.len(),
                ContentBlock::Image { data, .. } | ContentBlock::Document { data, .. } => {
                    data.len()
                }
            })
            .sum(),
        _ => message_text(msg).len(),
    }
}

/// Find the message index at which to split history so that the tail contains
/// at most `keep_turns` complete turns.
///
/// A "turn" begins at every user message (`Text { role: User }` or
/// `MultimodalUser`).  Because one turn can expand into many
/// `ChatHistoryMessage` entries (tool calls + results), splitting on raw
/// message count would silently prune far more context than configured.
fn find_split_for_turns(history: &[ChatHistoryMessage], keep_turns: usize) -> usize {
    if keep_turns == 0 {
        return history.len();
    }
    let mut turns_seen = 0usize;
    for (i, msg) in history.iter().enumerate().rev() {
        if is_user_turn_start(msg) {
            turns_seen += 1;
            if turns_seen == keep_turns {
                return i;
            }
        }
    }
    // Fewer turns in history than keep_turns — nothing to compact.
    history.len()
}

/// Returns `true` if `msg` marks the start of a new user turn.
fn is_user_turn_start(msg: &ChatHistoryMessage) -> bool {
    matches!(
        msg,
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            ..
        } | ChatHistoryMessage::MultimodalUser { .. }
    )
}

/// Extract the most meaningful text representation of a history message for
/// use in a summarisation transcript.
fn message_text(msg: &ChatHistoryMessage) -> String {
    match msg {
        ChatHistoryMessage::Text { content, .. } => content.clone(),
        ChatHistoryMessage::MultimodalUser { content } => content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text(t) => Some(t.as_str()),
                ContentBlock::Image { .. } | ContentBlock::Document { .. } => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        ChatHistoryMessage::AssistantToolCalls(calls) => {
            let summaries: Vec<String> = calls
                .iter()
                .map(|c: &ToolCallItem| {
                    let args = serde_json::to_string(&c.params).unwrap_or_default();
                    format!("{}({})", c.name, args)
                })
                .collect();
            format!("[tool calls: {}]", summaries.join(", "))
        }
        ChatHistoryMessage::ToolResult { name, content } => {
            format!("[tool result for {name}]: {content}")
        }
    }
}

/// Format a slice of messages into a labelled transcript string.
fn build_transcript(messages: &[ChatHistoryMessage]) -> String {
    messages
        .iter()
        .enumerate()
        .map(|(i, msg)| {
            let role_label = match msg {
                ChatHistoryMessage::Text { role, .. } => match role {
                    ChatRole::User => "User",
                    ChatRole::Assistant => "Assistant",
                    ChatRole::System => "System",
                    ChatRole::Tool => "Tool",
                },
                ChatHistoryMessage::MultimodalUser { .. } => "User",
                ChatHistoryMessage::AssistantToolCalls(_) => "Assistant",
                ChatHistoryMessage::ToolResult { .. } => "Tool",
            };
            format!("[Turn {i}] {role_label}: {}", message_text(msg))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Ask the LLM to summarise `transcript`.  Returns `None` on failure.
async fn summarise(llm: &Arc<dyn LlmProvider>, transcript: &str) -> Option<String> {
    let prompt = format!(
        "Summarise the following conversation history concisely, \
         preserving all important facts, decisions, and context. \
         Output only the summary, no preamble.\n\n{transcript}"
    );
    match llm
        .chat(
            "You are a helpful assistant that summarises conversations.",
            &[ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: prompt,
            }],
            &[],
        )
        .await
    {
        Ok(LlmResponse::FinalAnswer(text, _)) => Some(text),
        Ok(_) => {
            warn!("Compaction summary returned unexpected response variant");
            None
        }
        Err(e) => {
            warn!(error = %e, "Compaction summary LLM call failed");
            None
        }
    }
}

/// Summarise `messages` using chunked iterative summarisation when the
/// transcript exceeds [`CHUNK_CHARS`].
///
/// Returns `None` if every LLM call fails.
async fn summarise_messages(
    llm: &Arc<dyn LlmProvider>,
    messages: &[ChatHistoryMessage],
) -> Option<String> {
    let transcript = build_transcript(messages);

    if transcript.len() <= CHUNK_CHARS {
        return summarise(llm, &transcript).await;
    }

    // Split into char-bounded chunks and summarise each independently.
    let chunks: Vec<&str> = transcript
        .as_bytes()
        .chunks(CHUNK_CHARS)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    let mut chunk_summaries: Vec<String> = Vec::with_capacity(chunks.len());
    for chunk in &chunks {
        match summarise(llm, chunk).await {
            Some(s) => chunk_summaries.push(s),
            None => {
                warn!("Chunk summarisation failed; truncating transcript instead");
                // Fall back to truncating: keep only the most recent CHUNK_CHARS.
                let truncated = &transcript[transcript.len().saturating_sub(CHUNK_CHARS)..];
                return summarise(llm, truncated).await;
            }
        }
    }

    // Merge chunk summaries into a final summary.
    let merged = chunk_summaries.join("\n\n---\n\n");
    if merged.len() <= CHUNK_CHARS {
        summarise(llm, &merged).await
    } else {
        // Merge is still large; do one more round of summarisation on it.
        summarise(llm, &merged[..CHUNK_CHARS]).await
    }
}

/// Convert a `ChatHistoryMessage` back to a storable [`Message`].
///
/// Tool-calls JSON is re-serialised for `AssistantToolCalls`; the `skill_name`
/// field is preserved for `ToolResult` messages.  Turn indices are assigned
/// sequentially starting from `turn_offset`.
fn chat_history_to_message(
    msg: &ChatHistoryMessage,
    conversation_id: Uuid,
    turn: i64,
) -> Option<Message> {
    match msg {
        ChatHistoryMessage::Text { role, content } => {
            let r = match role {
                ChatRole::User => MessageRole::User,
                ChatRole::Assistant => MessageRole::Assistant,
                ChatRole::System => return None, // system messages live in the prompt
                ChatRole::Tool => MessageRole::Tool,
            };
            let mut m = Message::new(conversation_id, r, content.clone());
            m.turn = turn;
            Some(m)
        }
        ChatHistoryMessage::MultimodalUser { content } => {
            let text = content
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text(t) => Some(t.as_str()),
                    ContentBlock::Image { .. } | ContentBlock::Document { .. } => None,
                })
                .collect::<Vec<_>>()
                .join(" ");
            let mut m = Message::new(conversation_id, MessageRole::User, text);
            m.turn = turn;
            Some(m)
        }
        ChatHistoryMessage::AssistantToolCalls(calls) => {
            let json = serde_json::to_string(calls).ok()?;
            let mut m = Message::new(conversation_id, MessageRole::Assistant, String::new());
            m.tool_calls_json = Some(json);
            m.turn = turn;
            Some(m)
        }
        ChatHistoryMessage::ToolResult { name, content } => {
            let mut m = Message::new(conversation_id, MessageRole::Tool, content.clone());
            m.skill_name = Some(name.clone());
            m.turn = turn;
            Some(m)
        }
    }
}

/// Persist the compacted history back to storage so it survives turn
/// boundaries.  All existing messages for the conversation are replaced with
/// the compacted set.
async fn persist_compacted(
    conv_store: &dyn ConversationStore,
    conversation_id: Uuid,
    compacted: &[ChatHistoryMessage],
) -> Result<()> {
    conv_store
        .replace_history(
            conversation_id,
            &compacted
                .iter()
                .enumerate()
                .filter_map(|(i, msg)| chat_history_to_message(msg, conversation_id, i as i64))
                .collect::<Vec<_>>(),
        )
        .await
}

/// Compact the conversation history in-place.
///
/// Produces a summary of the old turns via the LLM, then replaces the
/// `history` slice with:
///   `[summary turn, …keep_recent_turns most-recent turns]`
///
/// If `conv_store` and `conversation_id` are provided the compacted state is
/// also written back to storage so that the next turn does not reload stale
/// full history.
///
/// Returns `true` if compaction was performed, `false` if there was nothing
/// to compact or the summary request failed.
pub async fn maybe_compact(
    history: &mut Vec<ChatHistoryMessage>,
    llm: &Arc<dyn LlmProvider>,
    cfg: &CompactionConfig,
    conv_store: Option<(&dyn ConversationStore, Uuid)>,
) -> bool {
    let keep = cfg.keep_recent_turns;

    let split_at = find_split_for_turns(history, keep);

    if split_at == 0 || split_at >= history.len() {
        debug!("Skipping compaction: not enough turns to compact");
        return false;
    }

    let old_turns = &history[..split_at];

    info!(
        old_messages = split_at,
        keep_recent_turns = keep,
        "Compacting context window"
    );

    let summary = match summarise_messages(llm, old_turns).await {
        Some(s) => s,
        None => {
            warn!("All compaction summarisation attempts failed; skipping compaction");
            return false;
        }
    };

    let recent = history.split_off(split_at);
    let summary_msg = ChatHistoryMessage::Text {
        role: ChatRole::Assistant,
        content: format!("[Conversation summary — earlier context compacted]\n\n{summary}"),
    };

    *history = std::iter::once(summary_msg).chain(recent).collect();

    info!(
        new_history_len = history.len(),
        "Context compaction complete"
    );

    // Persist so the next turn does not reload the full history.
    if let Some((store, conv_id)) = conv_store
        && let Err(e) = persist_compacted(store, conv_id, history).await
    {
        warn!(error = %e, "Failed to persist compacted history; in-memory compaction still applies");
    }

    true
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::features::CompactionConfig;

    fn cfg(enabled: bool, window: u64, floor: u64, soft: u64, keep: usize) -> CompactionConfig {
        CompactionConfig {
            enabled,
            context_window_tokens: window,
            reserve_floor_tokens: floor,
            soft_threshold_tokens: soft,
            keep_recent_turns: keep,
        }
    }

    #[test]
    fn should_compact_disabled() {
        let c = cfg(false, 200_000, 20_000, 4_000, 10);
        assert!(!should_compact(190_000, &c));
    }

    #[test]
    fn should_compact_below_threshold() {
        let c = cfg(true, 200_000, 20_000, 30_000, 10);
        // trigger = 200_000 - 20_000 - 30_000 = 150_000
        assert!(!should_compact(149_999, &c));
    }

    #[test]
    fn should_compact_at_threshold() {
        let c = cfg(true, 200_000, 20_000, 30_000, 10);
        // trigger = 150_000
        assert!(should_compact(150_000, &c));
    }

    #[test]
    fn should_compact_above_threshold() {
        let c = cfg(true, 200_000, 20_000, 30_000, 10);
        assert!(should_compact(175_000, &c));
    }

    #[test]
    fn should_compact_saturating_sub() {
        // reserve + soft > window → trigger saturates to 0 → always compact
        let c = cfg(true, 1_000, 800, 500, 5);
        assert!(should_compact(0, &c));
    }

    #[test]
    fn find_split_counts_turns_not_messages() {
        // 2 turns, each with a user msg + tool call + tool result + assistant reply
        let history = vec![
            // turn 1
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "q1".into(),
            },
            ChatHistoryMessage::AssistantToolCalls(vec![]),
            ChatHistoryMessage::ToolResult {
                name: "t".into(),
                content: "r".into(),
            },
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "a1".into(),
            },
            // turn 2
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "q2".into(),
            },
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "a2".into(),
            },
        ];

        // keep 1 turn → split at the start of turn 2 (index 4)
        assert_eq!(find_split_for_turns(&history, 1), 4);
        // keep 2 turns → split at start of turn 1 (index 0) → nothing to compact
        assert_eq!(find_split_for_turns(&history, 2), 0);
    }

    #[test]
    fn message_text_extracts_multimodal() {
        let msg = ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("hello".into()),
                ContentBlock::Image {
                    data: "base64data".into(),
                    media_type: "image/png".into(),
                },
                ContentBlock::Text(" world".into()),
            ],
        };
        assert_eq!(message_text(&msg), "hello  world");
    }

    #[test]
    fn message_text_tool_result_uses_content() {
        let msg = ChatHistoryMessage::ToolResult {
            name: "bash".into(),
            content: "exit code 0".into(),
        };
        assert!(message_text(&msg).contains("exit code 0"));
        assert!(message_text(&msg).contains("bash"));
    }

    #[test]
    fn estimate_tokens_rough() {
        let history = vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "a".repeat(400),
        }];
        // 400 chars / 4 = 100 tokens
        assert_eq!(estimate_tokens(&history), 100);
    }

    /// Regression: a conversation with ~703k bytes of content (~175,780 estimated
    /// tokens) must trigger compaction with the default config.  The old default
    /// soft_threshold of 4,000 produced a trigger at 176,000 — just 220 tokens
    /// above the estimate — causing real conversations to silently exceed the
    /// context window.
    #[test]
    fn default_config_triggers_on_near_limit_conversation() {
        let default_cfg = CompactionConfig::default();

        // 703,122 bytes of content (matches the real stuck conversation on schorschvm).
        // estimate_tokens uses 4 chars/token → 703_122 / 4 = 175_780 tokens.
        let large_tool_output = "x".repeat(703_122);
        let history = vec![ChatHistoryMessage::ToolResult {
            name: "bash".into(),
            content: large_tool_output,
        }];

        let estimated = estimate_tokens(&history);
        assert!(
            should_compact(estimated, &default_cfg),
            "expected compaction to trigger for estimated={estimated} tokens with default config \
             (window={}, floor={}, soft={}), trigger={}",
            default_cfg.context_window_tokens,
            default_cfg.reserve_floor_tokens,
            default_cfg.soft_threshold_tokens,
            default_cfg
                .context_window_tokens
                .saturating_sub(default_cfg.reserve_floor_tokens)
                .saturating_sub(default_cfg.soft_threshold_tokens),
        );
    }

    #[test]
    fn estimate_total_tokens_includes_system_and_tools() {
        let history = vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "a".repeat(400),
        }];
        let system_prompt = "b".repeat(200);
        let tools = vec![assistant_core::ToolSpec {
            name: "test-tool".into(),
            description: "A test tool".into(),
            params_schema: serde_json::json!({"type": "object"}),
            is_mutating: false,
            requires_confirmation: false,
        }];

        let history_only = estimate_tokens(&history);
        let total = estimate_total_tokens(&system_prompt, &history, &tools);

        assert!(
            total > history_only,
            "total estimate ({total}) should be larger than history-only ({history_only})"
        );
    }

    #[test]
    fn hard_truncate_drops_messages_when_over_limit() {
        // Context window = 1000 tokens, reserve = 200 → hard limit = 800 tokens.
        // Each message is 400 chars = 100 estimated tokens.
        let c = cfg(true, 1_000, 200, 100, 2);
        let mut history: Vec<ChatHistoryMessage> = (0..10)
            .map(|i| ChatHistoryMessage::Text {
                role: if i % 2 == 0 {
                    ChatRole::User
                } else {
                    ChatRole::Assistant
                },
                content: "x".repeat(400),
            })
            .collect();

        // 10 messages × 100 tokens = 1000 tokens > 800 hard limit
        let dropped = hard_truncate("", &mut history, &[], &c);
        assert!(dropped, "should have dropped messages");
        assert!(
            history.len() < 10,
            "should have fewer messages after truncation"
        );

        // The remaining history should fit within the hard limit.
        let remaining_estimate = estimate_total_tokens("", &history, &[]);
        assert!(
            remaining_estimate < 800,
            "remaining estimate ({remaining_estimate}) should be below hard limit (800)"
        );
    }

    #[test]
    fn hard_truncate_noop_when_under_limit() {
        let c = cfg(true, 200_000, 20_000, 30_000, 10);
        let mut history = vec![
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "hello".into(),
            },
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "hi there".into(),
            },
        ];
        let original_len = history.len();
        let dropped = hard_truncate("system prompt", &mut history, &[], &c);
        assert!(!dropped, "should not have dropped any messages");
        assert_eq!(history.len(), original_len);
    }

    #[test]
    fn hard_truncate_preserves_at_least_one_message() {
        // Even when a single message exceeds the limit, we keep at least 1.
        let c = cfg(true, 100, 50, 10, 1);
        let mut history = vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "x".repeat(10_000),
        }];
        hard_truncate("", &mut history, &[], &c);
        assert_eq!(history.len(), 1, "should keep at least one message");
    }
}
