//! Builds OpenRouter chat requests.

use serde_json::json;

use assistant_core::llm::call_context::LlmCallContext;
use assistant_core::types::llm::OpenRouterRouting;
use assistant_core::{ChatHistoryMessage, ChatOptions, ResponseFormat, ToolChoice, ToolSpec};

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
        tool_choice: None,
        parallel_tool_calls: None,
        response_format: None,
        plugins: Vec::new(),
    }
}

/// Adds `options` to `request` in OpenRouter's wire format. The caller
/// passes only options the model supports. `tool_choice` and
/// `response_format` also set `provider.require_parameters`, so routing
/// picks only endpoints that honor them; a non-streaming JSON-schema call
/// adds the `response-healing` plugin.
pub fn apply_options(request: &mut ChatRequest, options: &ChatOptions) {
    let has_tools = !request.tools.is_empty();
    if has_tools {
        request.tool_choice = match &options.tool_choice {
            ToolChoice::Auto => None,
            ToolChoice::None => Some(json!("none")),
            ToolChoice::Required => Some(json!("required")),
            ToolChoice::Tool(name) => Some(json!({"type": "function", "function": {"name": name}})),
        };
        request.parallel_tool_calls = options.parallel_tool_calls;
    }
    if let ResponseFormat::JsonSchema {
        name,
        schema,
        strict,
    } = &options.response_format
    {
        request.response_format = Some(json!({
            "type": "json_schema",
            "json_schema": {"name": name, "strict": strict, "schema": schema},
        }));
        if !request.stream {
            request.plugins.push(json!({"id": "response-healing"}));
        }
    }
    if request.tool_choice.is_some() || request.response_format.is_some() {
        request
            .provider
            .get_or_insert_with(Default::default)
            .require_parameters = Some(true);
    }
}
