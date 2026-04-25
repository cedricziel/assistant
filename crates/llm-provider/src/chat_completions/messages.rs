//! Chat Completions message conversion helpers.
//!
//! Converts [`ChatHistoryMessage`] slices into the `async-openai` Chat
//! Completions request format.  Shared by all OpenAI-compatible providers
//! (Moonshot, OpenRouter, …).

use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, FunctionCall,
};
use serde_json::{Value, json};
use tracing::warn;

use assistant_core::{ChatHistoryMessage, ChatRole, ContentBlock};

// ── async-openai typed messages ──────────────────────────────────────────────

/// Build `async-openai` Chat Completions messages from conversation history.
pub fn build_chat_messages(
    system_prompt: &str,
    history: &[ChatHistoryMessage],
) -> (Vec<ChatCompletionRequestMessage>, Vec<(String, String)>) {
    let mut messages: Vec<ChatCompletionRequestMessage> = Vec::with_capacity(history.len() + 1);
    let mut pending_ids: Vec<(String, String)> = Vec::new();

    if !system_prompt.is_empty()
        && let Ok(msg) = ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt)
            .build()
    {
        messages.push(ChatCompletionRequestMessage::System(msg));
    }

    for entry in history {
        match entry {
            ChatHistoryMessage::Text { role, content } => match role {
                ChatRole::System => {
                    if let Ok(msg) = ChatCompletionRequestSystemMessageArgs::default()
                        .content(content.as_str())
                        .build()
                    {
                        messages.push(ChatCompletionRequestMessage::System(msg));
                    }
                }
                ChatRole::User => {
                    if let Ok(msg) = ChatCompletionRequestUserMessageArgs::default()
                        .content(content.as_str())
                        .build()
                    {
                        messages.push(ChatCompletionRequestMessage::User(msg));
                    }
                }
                ChatRole::Assistant => {
                    if let Ok(msg) = ChatCompletionRequestAssistantMessageArgs::default()
                        .content(content.as_str())
                        .build()
                    {
                        messages.push(ChatCompletionRequestMessage::Assistant(msg));
                    }
                }
                ChatRole::Tool => {
                    if let Ok(msg) = ChatCompletionRequestUserMessageArgs::default()
                        .content(content.as_str())
                        .build()
                    {
                        messages.push(ChatCompletionRequestMessage::User(msg));
                    }
                }
            },

            ChatHistoryMessage::MultimodalUser { content } => {
                let parts_json: Vec<Value> = content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text(text) => Some(json!({"type": "text", "text": text})),
                        ContentBlock::Image { media_type, data } => {
                            let data_uri = format!("data:{media_type};base64,{data}");
                            Some(json!({
                                "type": "image_url",
                                "image_url": { "url": data_uri }
                            }))
                        }
                        ContentBlock::Document { .. } => None,
                    })
                    .collect();

                let msg_json = json!({"role": "user", "content": parts_json});
                match serde_json::from_value::<ChatCompletionRequestMessage>(msg_json) {
                    Ok(msg) => messages.push(msg),
                    Err(e) => {
                        warn!(
                            error = %e,
                            "Failed to deserialize multimodal message; falling back to text-only"
                        );
                        let text: String = content
                            .iter()
                            .filter_map(|b| match b {
                                ContentBlock::Text(t) => Some(t.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        if let Ok(msg) = ChatCompletionRequestUserMessageArgs::default()
                            .content(text.as_str())
                            .build()
                        {
                            messages.push(ChatCompletionRequestMessage::User(msg));
                        }
                    }
                }
            }

            ChatHistoryMessage::AssistantToolCalls(calls) => {
                let tc_enums: Vec<ChatCompletionMessageToolCalls> = calls
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let id = c.id.clone().unwrap_or_else(|| format!("call_{i}"));
                        pending_ids.push((c.name.clone(), id.clone()));
                        ChatCompletionMessageToolCalls::Function(ChatCompletionMessageToolCall {
                            id,
                            function: FunctionCall {
                                name: c.name.clone(),
                                arguments: serde_json::to_string(&c.params)
                                    .unwrap_or_else(|_| "{}".to_string()),
                            },
                        })
                    })
                    .collect();

                if let Ok(msg) = ChatCompletionRequestAssistantMessageArgs::default()
                    .tool_calls(tc_enums)
                    .build()
                {
                    messages.push(ChatCompletionRequestMessage::Assistant(msg));
                }
            }

            ChatHistoryMessage::ToolResult { name, content } => {
                let pos = pending_ids.iter().position(|(n, _)| n == name);
                let tool_call_id = if let Some(idx) = pos {
                    pending_ids.remove(idx).1
                } else {
                    warn!(tool = %name, "no pending tool-call ID for tool result, using fallback");
                    format!("call_unknown_{name}")
                };

                if let Ok(msg) = ChatCompletionRequestToolMessageArgs::default()
                    .tool_call_id(&tool_call_id)
                    .content(content.as_str())
                    .build()
                {
                    messages.push(ChatCompletionRequestMessage::Tool(msg));
                }
            }
        }
    }

    (messages, pending_ids)
}

// ── Raw JSON messages (for non-standard request paths) ───────────────────────

/// Build raw JSON messages from conversation history.
///
/// Used by providers that need to send non-standard request bodies (e.g.
/// Moonshot's `$web_search` with `builtin_function` tools).
pub fn build_raw_messages(system_prompt: &str, history: &[ChatHistoryMessage]) -> Vec<Value> {
    let mut messages: Vec<Value> = Vec::with_capacity(history.len() + 1);
    let mut pending_ids: Vec<(String, String)> = Vec::new();

    if !system_prompt.is_empty() {
        messages.push(json!({"role": "system", "content": system_prompt}));
    }

    for entry in history {
        match entry {
            ChatHistoryMessage::Text { role, content } => {
                let role_str = match role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "user",
                };
                messages.push(json!({"role": role_str, "content": content}));
            }

            ChatHistoryMessage::MultimodalUser { content } => {
                let parts: Vec<Value> = content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text(text) => Some(json!({"type": "text", "text": text})),
                        ContentBlock::Image { media_type, data } => {
                            let data_uri = format!("data:{media_type};base64,{data}");
                            Some(json!({"type": "image_url", "image_url": {"url": data_uri}}))
                        }
                        ContentBlock::Document { .. } => None,
                    })
                    .collect();
                messages.push(json!({"role": "user", "content": parts}));
            }

            ChatHistoryMessage::AssistantToolCalls(calls) => {
                let tool_calls: Vec<Value> = calls
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let id = c.id.clone().unwrap_or_else(|| format!("call_{i}"));
                        pending_ids.push((c.name.clone(), id.clone()));
                        json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": c.name,
                                "arguments": serde_json::to_string(&c.params)
                                    .unwrap_or_else(|_| "{}".to_string()),
                            }
                        })
                    })
                    .collect();
                messages
                    .push(json!({"role": "assistant", "content": null, "tool_calls": tool_calls}));
            }

            ChatHistoryMessage::ToolResult { name, content } => {
                let pos = pending_ids.iter().position(|(n, _)| n == name);
                let tool_call_id = if let Some(idx) = pos {
                    pending_ids.remove(idx).1
                } else {
                    warn!(tool = %name, "no pending tool-call ID for tool result, using fallback");
                    format!("call_unknown_{name}")
                };
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "name": name,
                    "content": content,
                }));
            }
        }
    }

    messages
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::ToolCallItem;

    // ── Typed message builder tests ──────────────────────────────────────

    #[test]
    fn build_chat_messages_system_prompt() {
        let (msgs, _) = build_chat_messages("You are helpful.", &[]);
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn build_chat_messages_text_exchange() {
        let history = vec![
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "hello".to_string(),
            },
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "hi there".to_string(),
            },
        ];
        let (msgs, _) = build_chat_messages("sys", &history);
        assert_eq!(msgs.len(), 3);
    }

    #[test]
    fn build_chat_messages_tool_calls_and_results() {
        let history = vec![
            ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
                name: "my-tool".to_string(),
                params: json!({"key": "val"}),
                id: Some("call_123".to_string()),
            }]),
            ChatHistoryMessage::ToolResult {
                name: "my-tool".to_string(),
                content: "result".to_string(),
            },
        ];
        let (msgs, pending) = build_chat_messages("", &history);
        assert_eq!(msgs.len(), 2);
        assert!(pending.is_empty());
    }

    #[test]
    fn build_chat_messages_multi_turn_tool_calls() {
        let history = vec![
            ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
                name: "tool-a".to_string(),
                params: json!({}),
                id: Some("call_r1".to_string()),
            }]),
            ChatHistoryMessage::ToolResult {
                name: "tool-a".to_string(),
                content: "result-a".to_string(),
            },
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "thinking...".to_string(),
            },
            ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
                name: "tool-b".to_string(),
                params: json!({}),
                id: Some("call_r2".to_string()),
            }]),
            ChatHistoryMessage::ToolResult {
                name: "tool-b".to_string(),
                content: "result-b".to_string(),
            },
        ];
        let (msgs, pending) = build_chat_messages("sys", &history);
        assert_eq!(msgs.len(), 6);
        assert!(pending.is_empty(), "all tool results should be consumed");
    }

    #[test]
    fn build_chat_messages_late_tool_result_preserves_id() {
        let history = vec![
            ChatHistoryMessage::AssistantToolCalls(vec![
                ToolCallItem {
                    name: "tool-a".to_string(),
                    params: json!({}),
                    id: Some("call_a".to_string()),
                },
                ToolCallItem {
                    name: "tool-b".to_string(),
                    params: json!({}),
                    id: Some("call_b".to_string()),
                },
            ]),
            ChatHistoryMessage::ToolResult {
                name: "tool-a".to_string(),
                content: "result-a".to_string(),
            },
            ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
                name: "tool-c".to_string(),
                params: json!({}),
                id: Some("call_c".to_string()),
            }]),
            ChatHistoryMessage::ToolResult {
                name: "tool-b".to_string(),
                content: "result-b".to_string(),
            },
            ChatHistoryMessage::ToolResult {
                name: "tool-c".to_string(),
                content: "result-c".to_string(),
            },
        ];
        let (msgs, pending) = build_chat_messages("", &history);
        assert_eq!(msgs.len(), 5);
        assert!(pending.is_empty(), "all tool results should be consumed");
    }

    // ── Raw message builder tests ────────────────────────────────────────

    #[test]
    fn build_raw_messages_system_prompt() {
        let msgs = build_raw_messages("You are helpful.", &[]);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "system");
    }

    #[test]
    fn build_raw_messages_text_exchange() {
        let history = vec![
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "hello".to_string(),
            },
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "hi there".to_string(),
            },
        ];
        let msgs = build_raw_messages("sys", &history);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[2]["role"], "assistant");
    }

    #[test]
    fn build_raw_messages_tool_calls_and_results() {
        let history = vec![
            ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
                name: "my-tool".to_string(),
                params: json!({"key": "val"}),
                id: Some("call_123".to_string()),
            }]),
            ChatHistoryMessage::ToolResult {
                name: "my-tool".to_string(),
                content: "result".to_string(),
            },
        ];
        let msgs = build_raw_messages("", &history);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[1]["tool_call_id"], "call_123");
    }

    #[test]
    fn build_raw_messages_late_tool_result_preserves_id() {
        let history = vec![
            ChatHistoryMessage::AssistantToolCalls(vec![
                ToolCallItem {
                    name: "tool-a".to_string(),
                    params: json!({}),
                    id: Some("call_a".to_string()),
                },
                ToolCallItem {
                    name: "tool-b".to_string(),
                    params: json!({}),
                    id: Some("call_b".to_string()),
                },
            ]),
            ChatHistoryMessage::ToolResult {
                name: "tool-a".to_string(),
                content: "result-a".to_string(),
            },
            ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
                name: "tool-c".to_string(),
                params: json!({}),
                id: Some("call_c".to_string()),
            }]),
            ChatHistoryMessage::ToolResult {
                name: "tool-b".to_string(),
                content: "result-b".to_string(),
            },
            ChatHistoryMessage::ToolResult {
                name: "tool-c".to_string(),
                content: "result-c".to_string(),
            },
        ];
        let msgs = build_raw_messages("", &history);
        assert_eq!(msgs.len(), 5);
        assert_eq!(
            msgs[3]["tool_call_id"], "call_b",
            "tool-b must keep its original call ID"
        );
        assert_eq!(msgs[4]["tool_call_id"], "call_c");
    }
}
