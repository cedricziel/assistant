//! Builds OpenRouter chat requests.

use serde_json::{Value, json};

use assistant_core::llm::call_context::LlmCallContext;
use assistant_core::types::llm::{
    OpenRouterCache, OpenRouterCacheTtl, OpenRouterReasoning, OpenRouterRouting,
    OpenRouterWebSearch,
};
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
    pub cache: OpenRouterCache,
    pub web_search: OpenRouterWebSearch,
}

/// OpenRouter's `web` plugin with the configured options, when enabled.
fn web_plugin(search: &OpenRouterWebSearch) -> Option<Value> {
    if !search.enabled {
        return None;
    }
    let mut plugin = json!({"id": "web"});
    if let Some(engine) = search.engine {
        plugin["engine"] = json!(engine);
    }
    if let Some(max_results) = search.max_results {
        plugin["max_results"] = json!(max_results);
    }
    if let Some(prompt) = &search.search_prompt {
        plugin["search_prompt"] = json!(prompt);
    }
    if let Some(domains) = &search.include_domains {
        plugin["include_domains"] = json!(domains);
    }
    if let Some(domains) = &search.exclude_domains {
        plugin["exclude_domains"] = json!(domains);
    }
    Some(plugin)
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
            apply_cache_breakpoints(&mut messages, &settings.model, &settings.cache);
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
        plugins: web_plugin(&settings.web_search).into_iter().collect(),
        web_search_options: settings
            .web_search
            .search_context_size
            .filter(|_| settings.web_search.enabled)
            .map(|size| json!({ "search_context_size": size })),
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

/// Adds `cache_control` breakpoints for models whose author needs them:
/// Anthropic, Google and Qwen get one on the system prompt, and Anthropic
/// also on the last part of the last user message. Other upstreams cache
/// automatically, so their requests get none.
pub fn apply_cache_breakpoints(messages: &mut [Value], model: &str, cache: &OpenRouterCache) {
    if !cache.enabled {
        return;
    }
    let author = model.split('/').next().unwrap_or_default();
    if !matches!(author, "anthropic" | "google" | "qwen") {
        return;
    }
    let mut control = json!({"type": "ephemeral"});
    // Only Claude caches take a TTL; Gemini and Qwen caches last 5 minutes.
    if author == "anthropic" && cache.ttl == OpenRouterCacheTtl::OneHour {
        control["ttl"] = json!("1h");
    }
    if let Some(system) = messages.iter_mut().find(|m| m["role"] == "system") {
        mark_last_part(system, &control);
    }
    if author == "anthropic"
        && let Some(user) = messages.iter_mut().rev().find(|m| m["role"] == "user")
    {
        mark_last_part(user, &control);
    }
}

/// Sets `cache_control` on the last content part of `message`, turning
/// string content into a single text part first.
fn mark_last_part(message: &mut Value, control: &Value) {
    if let Some(text) = message["content"].as_str() {
        message["content"] = json!([{"type": "text", "text": text}]);
    }
    if let Some(part) = message["content"]
        .as_array_mut()
        .and_then(|parts| parts.last_mut())
    {
        part["cache_control"] = control.clone();
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use assistant_core::types::llm::{OpenRouterCache, OpenRouterCacheTtl};

    use super::apply_cache_breakpoints;

    fn messages() -> Vec<Value> {
        vec![
            json!({"role": "system", "content": "sys"}),
            json!({"role": "user", "content": "first"}),
            json!({"role": "assistant", "content": "ok"}),
            json!({"role": "user", "content": [
                {"type": "text", "text": "look"},
                {"type": "image_url", "image_url": {"url": "data:image/png;base64,aGk="}},
            ]}),
        ]
    }

    /// Indices of messages that carry a breakpoint, and the breakpoints.
    fn breakpoints(messages: &[Value]) -> Vec<(usize, Value)> {
        messages
            .iter()
            .enumerate()
            .flat_map(|(i, m)| {
                m["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(move |part| part.get("cache_control").map(|c| (i, c.clone())))
            })
            .collect()
    }

    fn placed(model: &str, cache: OpenRouterCache) -> (Vec<Value>, Vec<(usize, Value)>) {
        let mut messages = messages();
        apply_cache_breakpoints(&mut messages, model, &cache);
        let found = breakpoints(&messages);
        (messages, found)
    }

    #[test]
    fn breakpoints_by_model_author() {
        let ephemeral = json!({"type": "ephemeral"});
        let cases: [(&str, Vec<usize>); 5] = [
            ("anthropic/claude-sonnet-4.5", vec![0, 3]),
            ("google/gemini-2.5-pro", vec![0]),
            ("qwen/qwen3-max", vec![0]),
            ("openai/gpt-4o", vec![]),
            ("meta-llama/llama-3.3-70b", vec![]),
        ];
        for (model, expected) in cases {
            let (_, found) = placed(model, OpenRouterCache::default());
            let at: Vec<usize> = found.iter().map(|(i, _)| *i).collect();
            assert_eq!(at, expected, "{model}");
            assert!(
                found.iter().all(|(_, c)| *c == ephemeral),
                "{model}: {found:?}"
            );
        }
    }

    #[test]
    fn system_prompt_becomes_a_text_part_and_the_last_part_is_marked() {
        let (messages, _) = placed("anthropic/claude-sonnet-4.5", OpenRouterCache::default());
        assert_eq!(
            messages[0]["content"],
            json!([{"type": "text", "text": "sys", "cache_control": {"type": "ephemeral"}}])
        );
        let last_user = messages[3]["content"].as_array().unwrap();
        assert!(last_user[0].get("cache_control").is_none());
        assert!(
            last_user[1].get("cache_control").is_some(),
            "the last part carries it"
        );
        assert_eq!(
            messages[1]["content"], "first",
            "earlier user messages are untouched"
        );
    }

    #[test]
    fn one_hour_ttl_is_sent_to_anthropic_only() {
        let cache = OpenRouterCache {
            ttl: OpenRouterCacheTtl::OneHour,
            ..OpenRouterCache::default()
        };
        let one_hour = json!({"type": "ephemeral", "ttl": "1h"});
        let (_, found) = placed("anthropic/claude-sonnet-4.5", cache.clone());
        assert_eq!(found, [(0, one_hour.clone()), (3, one_hour)]);

        // Gemini and Qwen caches always last five minutes.
        for model in ["google/gemini-2.5-pro", "qwen/qwen3-max"] {
            let (_, found) = placed(model, cache.clone());
            assert_eq!(found, [(0, json!({"type": "ephemeral"}))], "{model}");
        }
    }

    #[test]
    fn disabled_cache_sends_no_breakpoints() {
        let cache = OpenRouterCache {
            enabled: false,
            ..OpenRouterCache::default()
        };
        let (messages, found) = placed("anthropic/claude-sonnet-4.5", cache);
        assert!(found.is_empty(), "{found:?}");
        assert_eq!(messages, self::messages(), "messages are unchanged");
    }

    #[test]
    fn string_user_content_becomes_a_marked_text_part() {
        let mut messages = vec![
            json!({"role": "system", "content": "sys"}),
            json!({"role": "user", "content": "hi"}),
        ];
        apply_cache_breakpoints(
            &mut messages,
            "anthropic/claude-sonnet-4.5",
            &OpenRouterCache::default(),
        );
        assert_eq!(
            messages[1]["content"],
            json!([{"type": "text", "text": "hi", "cache_control": {"type": "ephemeral"}}])
        );
    }
}
