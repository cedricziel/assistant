//! Builds OpenRouter chat requests.

use serde_json::json;

use assistant_core::{ChatHistoryMessage, ToolSpec};

use super::wire::ChatRequest;
use crate::chat_completions::build_raw_messages;

pub fn build_chat_request(
    model: &str,
    max_tokens: u32,
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    tools: &[ToolSpec],
) -> ChatRequest {
    ChatRequest {
        model: model.to_owned(),
        messages: build_raw_messages(system_prompt, history),
        tools: tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.params_schema,
                    },
                })
            })
            .collect(),
        max_completion_tokens: max_tokens,
        stream: false,
    }
}
