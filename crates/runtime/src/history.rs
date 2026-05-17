//! Conversation history management helpers for the orchestrator.
//!
//! These functions handle converting persisted [`Message`] records into the
//! [`ChatHistoryMessage`] format the LLM expects, repairing structural issues
//! (orphaned messages, missing tool results), and error recovery.

use std::collections::HashMap;

use tracing::{debug, warn};
use uuid::Uuid;

use assistant_core::types::conversation::{Message, MessageRole};
use assistant_core::{ChatHistoryMessage, ChatRole, ContentBlock, ToolCallItem};
use assistant_storage::conversations::ConversationStore;

/// Pre-loaded attachment content blocks keyed by message ID.
///
/// Built by callers that load bytes from the [`AttachmentStore`] and convert
/// them into the appropriate [`ContentBlock`] variant (Image, Document, or
/// Text) before replaying history.
pub(crate) type AttachmentMap = HashMap<Uuid, Vec<ContentBlock>>;

/// Convert a sequence of persisted [`Message`] records into
/// [`ChatHistoryMessage`] values suitable for sending to the LLM.
///
/// When `attachment_map` is non-empty, user messages whose ID appears in the
/// map are emitted as [`ChatHistoryMessage::MultimodalUser`] with inline
/// image blocks so the LLM can see previously-sent images on history replay.
pub(crate) fn messages_to_chat_history(
    messages: Vec<Message>,
    attachment_map: &AttachmentMap,
) -> Vec<ChatHistoryMessage> {
    messages
        .into_iter()
        .filter_map(|m| match m.role {
            MessageRole::User => {
                if let Some(atts) = attachment_map.get(&m.id)
                    && !atts.is_empty()
                {
                    let mut blocks = vec![ContentBlock::Text(m.content)];
                    blocks.extend(atts.iter().cloned());
                    return Some(ChatHistoryMessage::MultimodalUser { content: blocks });
                }
                Some(ChatHistoryMessage::Text {
                    role: ChatRole::User,
                    content: m.content,
                })
            }
            MessageRole::Assistant => {
                if let Some(tc_json) = m.tool_calls_json
                    && let Ok(items) = serde_json::from_str::<Vec<ToolCallItem>>(&tc_json)
                    && !items.is_empty()
                {
                    return Some(ChatHistoryMessage::AssistantToolCalls(items));
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

    // --- Pass 1: drop orphaned ToolResult messages -------------------------
    //
    // A ToolResult is valid only when it immediately follows an
    // AssistantToolCalls (or another valid ToolResult within the same
    // batch).  System-injected results (e.g. from skill-learner) that
    // have no preceding AssistantToolCalls cause providers to reject the
    // request because the tool_call_id has no matching tool call.
    {
        let mut i = 0;
        let mut remaining_results: usize = 0; // results still expected from current batch
        while i < history.len() {
            match &history[i] {
                ChatHistoryMessage::AssistantToolCalls(calls) => {
                    remaining_results = calls.len();
                    i += 1;
                }
                ChatHistoryMessage::ToolResult { name, .. } => {
                    if remaining_results > 0 {
                        remaining_results -= 1;
                        i += 1;
                    } else {
                        debug!(
                            tool = %name,
                            "Sanitizing history: dropping orphaned ToolResult with no preceding tool call"
                        );
                        history.remove(i);
                        // don't advance i; the next element slid into this position
                    }
                }
                _ => {
                    remaining_results = 0;
                    i += 1;
                }
            }
        }
    }

    // --- Pass 2: fill in missing tool results for orphaned tool calls ------
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
pub(crate) async fn persist_error_recovery(
    conv_store: &dyn ConversationStore,
    conversation_id: Uuid,
) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::conversation::MessageRole;

    fn make_user_msg(id: Uuid, content: &str) -> Message {
        Message {
            id,
            conversation_id: Uuid::new_v4(),
            role: MessageRole::User,
            content: content.to_string(),
            skill_name: None,
            tool_calls_json: None,
            turn: 0,
            created_at: chrono::Utc::now(),
            sender_user_id: None,
        }
    }

    fn make_assistant_msg(content: &str) -> Message {
        Message {
            id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: content.to_string(),
            skill_name: None,
            tool_calls_json: None,
            turn: 1,
            created_at: chrono::Utc::now(),
            sender_user_id: None,
        }
    }

    #[test]
    fn history_without_attachment_map_produces_plain_text() {
        let msgs = vec![
            make_user_msg(Uuid::new_v4(), "hello"),
            make_assistant_msg("hi there"),
        ];
        let map = AttachmentMap::new();
        let history = messages_to_chat_history(msgs, &map);

        assert_eq!(history.len(), 2);
        assert!(matches!(
            &history[0],
            ChatHistoryMessage::Text { role: ChatRole::User, content } if content == "hello"
        ));
    }

    #[test]
    fn history_with_attachment_map_produces_multimodal_user() {
        let msg_id = Uuid::new_v4();
        let msgs = vec![
            make_user_msg(msg_id, "look at this image"),
            make_assistant_msg("I see it"),
        ];

        let mut map = AttachmentMap::new();
        map.insert(
            msg_id,
            vec![ContentBlock::Image {
                media_type: "image/png".to_string(),
                data: "base64data".to_string(),
            }],
        );

        let history = messages_to_chat_history(msgs, &map);
        assert_eq!(history.len(), 2);

        match &history[0] {
            ChatHistoryMessage::MultimodalUser { content } => {
                assert_eq!(content.len(), 2, "text + image");
                assert!(matches!(&content[0], ContentBlock::Text(t) if t == "look at this image"));
                assert!(matches!(
                    &content[1],
                    ContentBlock::Image { media_type, data }
                    if media_type == "image/png" && data == "base64data"
                ));
            }
            other => panic!("expected MultimodalUser, got {:?}", other),
        }
    }

    #[test]
    fn history_with_document_attachment_produces_multimodal_user() {
        let msg_id = Uuid::new_v4();
        let msgs = vec![make_user_msg(msg_id, "summarize this")];

        let mut map = AttachmentMap::new();
        map.insert(
            msg_id,
            vec![ContentBlock::Document {
                media_type: "application/pdf".to_string(),
                data: "JVBERi0=".to_string(),
            }],
        );

        let history = messages_to_chat_history(msgs, &map);
        match &history[0] {
            ChatHistoryMessage::MultimodalUser { content } => {
                assert_eq!(content.len(), 2, "text + document");
                assert!(matches!(
                    &content[1],
                    ContentBlock::Document { media_type, .. }
                    if media_type == "application/pdf"
                ));
            }
            other => panic!("expected MultimodalUser, got {:?}", other),
        }
    }

    #[test]
    fn history_with_text_attachment_produces_multimodal_user() {
        let msg_id = Uuid::new_v4();
        let msgs = vec![make_user_msg(msg_id, "check this")];

        let mut map = AttachmentMap::new();
        map.insert(
            msg_id,
            vec![ContentBlock::Text(
                "--- file: notes.md ---\n# Notes\n--- end file ---".to_string(),
            )],
        );

        let history = messages_to_chat_history(msgs, &map);
        match &history[0] {
            ChatHistoryMessage::MultimodalUser { content } => {
                assert_eq!(content.len(), 2, "text + inlined file");
                assert!(matches!(&content[1], ContentBlock::Text(t) if t.contains("notes.md")));
            }
            other => panic!("expected MultimodalUser, got {:?}", other),
        }
    }

    #[test]
    fn history_with_empty_attachment_list_produces_plain_text() {
        let msg_id = Uuid::new_v4();
        let msgs = vec![make_user_msg(msg_id, "no images")];

        let mut map = AttachmentMap::new();
        map.insert(msg_id, vec![]);

        let history = messages_to_chat_history(msgs, &map);
        assert!(matches!(
            &history[0],
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                ..
            }
        ));
    }
}
