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
pub fn tool_spec_to_chat(tool: &ToolSpec) -> ChatCompletionTool {
    let function = FunctionObjectArgs::default()
        .name(&tool.name)
        .description(&tool.description)
        .parameters(tool.normalized_params_schema())
        .build()
        .expect("FunctionObject build should not fail");

    ChatCompletionTool { function }
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
