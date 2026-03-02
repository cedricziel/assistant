//! Conversation history management helpers for the orchestrator.
//!
//! These functions handle converting persisted [`Message`] records into the
//! [`ChatHistoryMessage`] format the LLM expects, repairing structural issues
//! (orphaned messages, missing tool results), and error recovery.

use assistant_core::{Message, MessageRole};
use assistant_llm::{ChatHistoryMessage, ChatRole};
use assistant_storage::conversations::ConversationStore;
use tracing::{debug, warn};
use uuid::Uuid;

/// Convert a sequence of persisted [`Message`] records into
/// [`ChatHistoryMessage`] values suitable for sending to the LLM.
pub(crate) fn messages_to_chat_history(messages: Vec<Message>) -> Vec<ChatHistoryMessage> {
    messages
        .into_iter()
        .filter_map(|m| match m.role {
            MessageRole::User => Some(ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: m.content,
            }),
            MessageRole::Assistant => {
                if let Some(tc_json) = m.tool_calls_json {
                    if let Ok(items) =
                        serde_json::from_str::<Vec<assistant_llm::ToolCallItem>>(&tc_json)
                    {
                        if !items.is_empty() {
                            return Some(ChatHistoryMessage::AssistantToolCalls(items));
                        }
                    }
                }
                Some(ChatHistoryMessage::Text {
                    role: ChatRole::Assistant,
                    content: m.content,
                })
            }
            MessageRole::Tool => Some(ChatHistoryMessage::ToolResult {
                name: m.skill_name.unwrap_or_default(),
                content: m.content,
            }),
            _ => None,
        })
        .collect()
}

/// Repair structural problems in a loaded conversation history.
///
/// Two issues are addressed:
///
/// 1. **Trailing orphaned user message** – a prior turn may have failed
///    after persisting the user message but before any assistant response
///    was saved.  A synthetic assistant message is inserted so the caller
///    can safely append a new user message without creating consecutive
///    user entries (which Anthropic rejects outright and which confuse
///    most tool-calling models).
///
/// 2. **Orphaned `AssistantToolCalls`** – the process may have crashed
///    after persisting a tool-call message but before all `ToolResult`
///    messages were written.  Missing results are filled in with a
///    synthetic error result so providers that require tool results
///    (Ollama, Anthropic) do not reject the request.
pub(crate) fn sanitize_history(history: &mut Vec<ChatHistoryMessage>) {
    if history.is_empty() {
        return;
    }

    // --- Pass 1: fill in missing tool results for orphaned tool calls ------
    //
    // Walk the history and, for every AssistantToolCalls, count how many
    // ToolResult messages follow (before the next non-ToolResult entry or
    // the end of the list).  If fewer results exist than calls, insert
    // synthetic ones.
    let mut i = 0;
    while i < history.len() {
        if let ChatHistoryMessage::AssistantToolCalls(calls) = &history[i] {
            let expected = calls.len();
            let call_names: Vec<String> = calls.iter().map(|c| c.name.clone()).collect();

            // Count consecutive ToolResult messages immediately following.
            let mut result_count = 0;
            while i + 1 + result_count < history.len() {
                if matches!(
                    history[i + 1 + result_count],
                    ChatHistoryMessage::ToolResult { .. }
                ) {
                    result_count += 1;
                } else {
                    break;
                }
            }

            if result_count < expected {
                let insert_at = i + 1 + result_count;
                let missing = expected - result_count;
                debug!(
                    expected,
                    result_count, missing, "Sanitizing history: inserting synthetic tool results"
                );
                for j in result_count..expected {
                    let name = call_names
                        .get(j)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    history.insert(
                        insert_at + (j - result_count),
                        ChatHistoryMessage::ToolResult {
                            name,
                            content: "[error: result lost due to a prior crash]".to_string(),
                        },
                    );
                }
                // Advance past the newly inserted results.
                i = insert_at + missing;
                continue;
            }
        }
        i += 1;
    }

    // --- Pass 2: trailing orphaned user message ----------------------------
    let is_trailing_user = matches!(
        history.last(),
        Some(ChatHistoryMessage::Text {
            role: ChatRole::User,
            ..
        }) | Some(ChatHistoryMessage::MultimodalUser { .. })
    );

    if is_trailing_user {
        debug!(
            "Sanitizing history: inserting synthetic assistant message after orphaned user message"
        );
        history.push(ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content: "[An error occurred processing the previous message.]".to_string(),
        });
    }
}

/// Persist a synthetic assistant message so the conversation history
/// maintains proper User→Assistant alternation after a turn error.
///
/// Called when the tool-calling loop (or the LLM call itself) fails.
/// The user message was already persisted by `prepare_history`; without
/// this recovery message the orphaned user entry would poison subsequent
/// turns.
pub(crate) async fn persist_error_recovery(conv_store: &ConversationStore, conversation_id: Uuid) {
    let error_msg = Message::assistant(
        conversation_id,
        "[An error occurred processing this message.]",
    );
    if let Err(e) = conv_store.save_message(&error_msg).await {
        warn!("Failed to persist error recovery assistant message: {e}");
    }
}

/// Append a tool result to the running chat history.
pub(crate) fn append_tool_result(history: &mut Vec<ChatHistoryMessage>, name: &str, content: &str) {
    history.push(ChatHistoryMessage::ToolResult {
        name: name.to_string(),
        content: content.to_string(),
    });
}
