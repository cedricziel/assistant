//! Chat Completions tool-related helpers.
//!
//! Converts [`ToolSpec`] to `async-openai` function tool types and parses
//! tool-call responses back.

use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionTool, CompletionUsage, FunctionObjectArgs,
};
use serde_json::Value;

use assistant_core::{LlmResponseMeta, ToolCallItem, ToolSpec};

/// Convert a [`ToolSpec`] to an `async-openai` `ChatCompletionTool`.
///
/// Returns an error if the `FunctionObject` builder fails (in practice this
/// only happens when a required field is missing, which our deterministic
/// construction below cannot trigger — but we propagate rather than panic).
pub fn tool_spec_to_chat(tool: &ToolSpec) -> anyhow::Result<ChatCompletionTool> {
    let function = FunctionObjectArgs::default()
        .name(&tool.name)
        .description(&tool.description)
        .parameters(tool.normalized_params_schema())
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build OpenAI FunctionObject: {e}"))?;

    Ok(ChatCompletionTool { function })
}

/// Extract response metadata from a Chat Completions response.
pub fn extract_chat_meta(
    model: &str,
    id: &str,
    usage: &Option<CompletionUsage>,
) -> LlmResponseMeta {
    LlmResponseMeta {
        model: Some(model.to_string()),
        response_id: Some(id.to_string()),
        input_tokens: usage.as_ref().map(|u| u.prompt_tokens as u64),
        output_tokens: usage.as_ref().map(|u| u.completion_tokens as u64),
        finish_reason: None,
    }
}

/// Parse `async-openai` tool calls into [`ToolCallItem`]s.
pub fn parse_tool_calls(tool_calls: &[ChatCompletionMessageToolCalls]) -> Vec<ToolCallItem> {
    tool_calls
        .iter()
        .filter_map(|tc_enum| match tc_enum {
            ChatCompletionMessageToolCalls::Function(tc) => {
                if tc.function.name.is_empty() {
                    return None;
                }
                let params = serde_json::from_str::<Value>(&tc.function.arguments)
                    .unwrap_or(Value::Object(serde_json::Map::new()));
                Some(ToolCallItem {
                    name: tc.function.name.clone(),
                    params,
                    id: Some(tc.id.clone()),
                })
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_openai::types::chat::{
        ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls, FunctionCall,
    };

    #[test]
    fn tool_spec_to_chat_builds_function_object() {
        let spec = ToolSpec {
            name: "echo".into(),
            description: "Echo a string".into(),
            params_schema: serde_json::json!({"type": "object"}),
            is_mutating: false,
            requires_confirmation: false,
        };
        let tool = tool_spec_to_chat(&spec).unwrap();
        assert_eq!(tool.function.name, "echo");
        assert_eq!(tool.function.description.as_deref(), Some("Echo a string"));
    }

    #[test]
    fn extract_chat_meta_includes_usage_when_present() {
        let usage = Some(CompletionUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        });
        let meta = extract_chat_meta("gpt-4o", "chatcmpl-1", &usage);
        assert_eq!(meta.model.as_deref(), Some("gpt-4o"));
        assert_eq!(meta.response_id.as_deref(), Some("chatcmpl-1"));
        assert_eq!(meta.input_tokens, Some(10));
        assert_eq!(meta.output_tokens, Some(5));
        assert!(meta.finish_reason.is_none());
    }

    #[test]
    fn extract_chat_meta_handles_no_usage() {
        let meta = extract_chat_meta("m", "i", &None);
        assert!(meta.input_tokens.is_none());
        assert!(meta.output_tokens.is_none());
    }

    #[test]
    fn parse_tool_calls_parses_arguments_to_value() {
        let calls = vec![ChatCompletionMessageToolCalls::Function(
            ChatCompletionMessageToolCall {
                id: "call_1".into(),
                function: FunctionCall {
                    name: "shell".into(),
                    arguments: r#"{"cmd":"ls"}"#.into(),
                },
            },
        )];
        let parsed = parse_tool_calls(&calls);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "shell");
        assert_eq!(parsed[0].params["cmd"], "ls");
        assert_eq!(parsed[0].id.as_deref(), Some("call_1"));
    }

    #[test]
    fn parse_tool_calls_skips_calls_with_empty_name() {
        let calls = vec![ChatCompletionMessageToolCalls::Function(
            ChatCompletionMessageToolCall {
                id: "call_x".into(),
                function: FunctionCall {
                    name: "".into(),
                    arguments: "{}".into(),
                },
            },
        )];
        let parsed = parse_tool_calls(&calls);
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_tool_calls_falls_back_to_empty_object_on_bad_arguments() {
        let calls = vec![ChatCompletionMessageToolCalls::Function(
            ChatCompletionMessageToolCall {
                id: "call_2".into(),
                function: FunctionCall {
                    name: "shell".into(),
                    arguments: "not-json".into(),
                },
            },
        )];
        let parsed = parse_tool_calls(&calls);
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].params.is_object());
        assert!(parsed[0].params.as_object().unwrap().is_empty());
    }
}
