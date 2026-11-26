//! Builds OpenRouter chat requests.

use serde_json::json;

use assistant_core::llm::call_context::LlmCallContext;
use assistant_core::types::llm::OpenRouterRouting;
use assistant_core::{ChatHistoryMessage, ToolSpec};

use super::wire::ChatRequest;
use crate::chat_completions::build_raw_messages;

/// The per-provider part of every request.
pub struct RequestSettings {
    pub model: String,
    pub max_tokens: u32,
    /// Fallback models after the primary one; empty when none are set.
    pub fallback_models: Vec<String>,
    pub routing: Option<OpenRouterRouting>,
}

/// Builds a request. Inside a turn, [`LlmCallContext`] supplies
/// `session_id` and `user`; outside one, both are omitted.
pub fn build_chat_request(
    settings: &RequestSettings,
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    tools: &[ToolSpec],
) -> ChatRequest {
    let context = LlmCallContext::current();
    let models = if settings.fallback_models.is_empty() {
        Vec::new()
    } else {
        std::iter::once(settings.model.clone())
            .chain(settings.fallback_models.iter().cloned())
            .collect()
    };
    ChatRequest {
        model: settings.model.clone(),
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
        max_completion_tokens: settings.max_tokens,
        stream: false,
        models,
        provider: settings.routing.clone(),
        session_id: context.as_ref().map(|c| c.conversation_id.to_string()),
        user: context.and_then(|c| c.user_hash),
    }
}
