//! Context compaction — automatic history truncation when the context window
//! approaches its limit.
//!
//! When the LLM reports that `input_tokens` has crossed the soft threshold
//! (`context_window - reserve_floor - soft_threshold`), the orchestrator
//! calls [`maybe_compact`] which:
//!
//! 1. Asks the LLM to produce a short summary of the conversation so far.
//! 2. Replaces the in-memory `history` with a single summary turn followed
//!    by the `keep_recent_turns` most recent messages.
//!
//! This mirrors OpenClaw's `contextWindow - reserveFloor(20k) - softThreshold(4k)`
//! trigger logic.

use std::sync::Arc;

use assistant_core::CompactionConfig;
use assistant_llm::{ChatHistoryMessage, ChatRole, LlmProvider};
use tracing::{debug, info, warn};

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

/// Compact the conversation history in-place.
///
/// Produces a summary of the old turns via the LLM, then replaces the
/// `history` slice with:
///   `[summary turn, …keep_recent_turns most-recent messages]`
///
/// Returns `true` if compaction was performed, `false` if there was nothing
/// to compact or the summary request failed.
pub async fn maybe_compact(
    history: &mut Vec<ChatHistoryMessage>,
    llm: &Arc<dyn LlmProvider>,
    cfg: &CompactionConfig,
) -> bool {
    let keep = cfg.keep_recent_turns;

    if history.len() <= keep {
        debug!("Skipping compaction: history shorter than keep_recent_turns");
        return false;
    }

    let split_at = history.len().saturating_sub(keep);
    let old_turns = &history[..split_at];

    // Build a compact transcript of the turns being summarised.
    let transcript: String = old_turns
        .iter()
        .enumerate()
        .map(|(i, msg)| {
            let (role_label, content) = match msg {
                ChatHistoryMessage::Text { role, content } => {
                    let label = match role {
                        ChatRole::User => "User",
                        ChatRole::Assistant => "Assistant",
                        ChatRole::System => "System",
                        ChatRole::Tool => "Tool",
                    };
                    (label, content.as_str())
                }
                ChatHistoryMessage::MultimodalUser { .. } => ("User", "[multimodal message]"),
                ChatHistoryMessage::AssistantToolCalls(_) => ("Assistant", "[tool call]"),
                ChatHistoryMessage::ToolResult { .. } => ("Tool", "[tool result]"),
            };
            format!("[Turn {i}] {role_label}: {content}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let summary_prompt = format!(
        "Summarise the following conversation history concisely, \
         preserving all important facts, decisions, and context. \
         Output only the summary, no preamble.\n\n{transcript}"
    );

    info!(
        old_turns = split_at,
        keep_recent = keep,
        "Compacting context window"
    );

    // Ask the LLM for a summary (no tools, simple chat).
    let summary = match llm
        .chat(
            "You are a helpful assistant that summarises conversations.",
            &[ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: summary_prompt,
            }],
            &[],
        )
        .await
    {
        Ok(response) => match response {
            assistant_llm::LlmResponse::FinalAnswer(text, _) => text,
            _ => {
                warn!("Compaction summary returned unexpected response variant");
                return false;
            }
        },
        Err(e) => {
            warn!(error = %e, "Compaction summary LLM call failed; skipping compaction");
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
    true
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::CompactionConfig;

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
        let c = cfg(true, 200_000, 20_000, 4_000, 10);
        // trigger = 200_000 - 20_000 - 4_000 = 176_000
        assert!(!should_compact(175_999, &c));
    }

    #[test]
    fn should_compact_at_threshold() {
        let c = cfg(true, 200_000, 20_000, 4_000, 10);
        assert!(should_compact(176_000, &c));
    }

    #[test]
    fn should_compact_above_threshold() {
        let c = cfg(true, 200_000, 20_000, 4_000, 10);
        assert!(should_compact(190_000, &c));
    }

    #[test]
    fn should_compact_saturating_sub() {
        // reserve + soft > window → trigger saturates to 0 → always compact
        let c = cfg(true, 1_000, 800, 500, 5);
        assert!(should_compact(0, &c));
    }
}
