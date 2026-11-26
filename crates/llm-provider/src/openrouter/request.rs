//! Builds OpenRouter chat requests.

use serde_json::json;

use assistant_core::llm::call_context::LlmCallContext;
use assistant_core::types::llm::{OpenRouterReasoning, OpenRouterRouting};
use assistant_core::{
    ChatHistoryMessage, ChatOptions, ContentBlock, ResponseFormat, ToolChoice, ToolSpec,
};

use super::reasoning::attach_details;
use super::wire::ChatRequest;
use crate::chat_completions::build_raw_messages;

/// The per-provider part of every request.
pub struct RequestSettings {
    pub model: String,
    pub max_tokens: u32,
    /// Fallback models after the primary one; empty when none are set.
    pub fallback_models: Vec<String>,
    pub routing: Option<OpenRouterRouting>,
    pub reasoning: Option<OpenRouterReasoning>,
}

/// Stands in for an image sent to a model that doesn't accept images.
const IMAGE_OMITTED: &str = "[An image was attached, but this model cannot view images.]";

/// Builds a request. Inside a turn, [`LlmCallContext`] supplies
/// `session_id` and `user`; outside one, both are omitted. Without
/// `vision`, each image becomes a short text note.
pub fn build_chat_request(
    settings: &RequestSettings,
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    tools: &[ToolSpec],
    vision: bool,
) -> ChatRequest {
    let text_only;
    let history = if vision {
        history
    } else {
        text_only = without_images(history);
        &text_only[..]
    };
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
        messages: {
            let mut messages = build_raw_messages(system_prompt, history);
            attach_details(&mut messages, history, &settings.model);
            messages
        },
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
        reasoning: settings.reasoning.clone(),
        session_id: context.as_ref().map(|c| c.conversation_id.to_string()),
        user: context.and_then(|c| c.user_hash),
        tool_choice: None,
        parallel_tool_calls: None,
        response_format: None,
        plugins: Vec::new(),
    }
}

fn without_images(history: &[ChatHistoryMessage]) -> Vec<ChatHistoryMessage> {
    history
        .iter()
        .map(|message| match message {
            ChatHistoryMessage::MultimodalUser { content } => ChatHistoryMessage::MultimodalUser {
                content: content
                    .iter()
                    .map(|block| match block {
                        ContentBlock::Image { .. } => ContentBlock::Text(IMAGE_OMITTED.into()),
                        other => other.clone(),
                    })
                    .collect(),
            },
            other => other.clone(),
        })
        .collect()
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
