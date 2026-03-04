//! `MoonshotProvider` — [`LlmProvider`] implementation for the Moonshot AI
//! (Kimi) chat completions API.
//!
//! Uses `async-openai`'s Chat Completions client directly (Moonshot's API is
//! OpenAI-compatible).  The `$web_search` builtin is handled via raw HTTP
//! because it uses the non-standard `"type": "builtin_function"` tool spec
//! and requires an echo-back loop.

use std::time::Duration;

use async_openai::config::OpenAIConfig as AsyncOpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionTools, CompletionUsage,
    CreateChatCompletionRequestArgs, FunctionCall, FunctionObjectArgs,
};
use async_openai::Client;
use async_trait::async_trait;
use futures::StreamExt as _;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use assistant_core::LlmConfig;
use assistant_llm::{
    is_transient_error_message, with_retry, Capabilities, ChatHistoryMessage, ChatRole,
    ContentBlock, HostedTool, LlmProvider, LlmResponse, LlmResponseMeta, RetryConfig, ToolCallItem,
    ToolSpec, ToolSupport,
};

// ── Defaults ──────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL: &str = "https://api.moonshot.ai/v1";
const DEFAULT_MAX_TOKENS: u32 = 8192;

/// Upper bound on the number of `$web_search` echo-back rounds to prevent
/// infinite loops if the API never returns `finish_reason: "stop"`.
const MAX_WEB_SEARCH_ROUNDS: usize = 5;

// ── MoonshotProvider ──────────────────────────────────────────────────────────

/// [`LlmProvider`] backed by the Moonshot AI (Kimi) chat completions API.
///
/// Uses `async-openai` for the standard chat path (messages + function tools).
/// When `web_search` is enabled, requests go through raw HTTP instead, because
/// the `$web_search` tool uses the non-standard `"type": "builtin_function"`
/// and requires a multi-round echo-back loop.
pub struct MoonshotProvider {
    /// async-openai client configured for the Moonshot endpoint.
    client: Client<AsyncOpenAIConfig>,
    model: String,
    base_url: String,
    max_tokens: u32,
    /// API key — needed for the raw-HTTP web-search path.
    api_key: String,
    web_search_enabled: bool,
    /// Pre-configured HTTP client (with tracing middleware) for the raw-HTTP
    /// web-search path.
    http_client: reqwest_middleware::ClientWithMiddleware,
}

impl MoonshotProvider {
    /// Create from explicit config values.
    pub fn new(
        model: String,
        base_url: String,
        api_key: &str,
        timeout_secs: u64,
        max_tokens: u32,
        web_search_enabled: bool,
    ) -> anyhow::Result<Self> {
        let oai_cfg = AsyncOpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(&base_url);
        let http = build_reqwest_client(timeout_secs)?;
        let client = Client::with_config(oai_cfg).with_http_client(http);

        let traced_client =
            assistant_llm::build_http_client(timeout_secs, &assistant_llm::RetryConfig::default())?;

        debug!(
            model = %model,
            base_url = %base_url,
            web_search = web_search_enabled,
            "Moonshot provider initialised"
        );

        Ok(Self {
            client,
            model,
            base_url,
            max_tokens,
            api_key: api_key.to_string(),
            web_search_enabled,
            http_client: traced_client,
        })
    }

    /// Convenience constructor directly from [`LlmConfig`].
    ///
    /// Resolves the API key from `config.api_key` or the `MOONSHOT_API_KEY`
    /// environment variable.
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        let api_key = cfg
            .api_key
            .clone()
            .or_else(|| std::env::var("MOONSHOT_API_KEY").ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Moonshot API key not found. Set api_key in [llm] config or \
                     MOONSHOT_API_KEY environment variable."
                )
            })?;

        // If the base_url is still the default Ollama value, swap in the
        // Moonshot default.
        let base_url = if cfg.base_url == "http://localhost:11434" {
            DEFAULT_BASE_URL.to_string()
        } else {
            cfg.base_url.clone()
        };

        let max_tokens = cfg.moonshot.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
        let web_search_enabled = cfg.moonshot.web_search.enabled;

        Self::new(
            cfg.model.clone(),
            base_url,
            &api_key,
            cfg.timeout_secs,
            max_tokens,
            web_search_enabled,
        )
    }

    // ── Non-streaming chat (async-openai) ─────────────────────────────────

    async fn chat_non_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        debug!(model = %self.model, tools = tools.len(), "Sending request to Moonshot");

        let (messages, _pending) = build_chat_messages(system_prompt, history);
        let openai_tools: Vec<ChatCompletionTools> = tools
            .iter()
            .map(|t| ChatCompletionTools::Function(tool_spec_to_chat(t)))
            .collect();

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder
            .model(&self.model)
            .messages(messages)
            .max_completion_tokens(self.max_tokens);
        if !openai_tools.is_empty() {
            builder.tools(openai_tools);
        }
        let request = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Moonshot request: {e}"))?;

        let retry_config = RetryConfig::default();
        let response = with_retry(
            &retry_config,
            "Moonshot",
            |e: &anyhow::Error| is_transient_error_message(&e.to_string()),
            || {
                let req = request.clone();
                let client = &self.client;
                async move {
                    client
                        .chat()
                        .create(req)
                        .await
                        .map_err(|e| anyhow::anyhow!("Moonshot request failed: {e}"))
                }
            },
        )
        .await?;

        debug!("Moonshot response received");

        let meta = extract_chat_meta(&response.model, &response.id, &response.usage);
        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("Moonshot returned empty choices"))?;

        if let Some(ref tool_calls) = choice.message.tool_calls {
            let items = parse_tool_calls(tool_calls);
            if !items.is_empty() {
                debug!(count = items.len(), "Moonshot: tool calls received");
                return Ok(LlmResponse::ToolCalls(items, meta));
            }
        }

        let content = choice.message.content.as_deref().unwrap_or("").to_string();
        Ok(LlmResponse::FinalAnswer(content, meta))
    }

    // ── SSE streaming chat (async-openai) ─────────────────────────────────

    async fn chat_sse(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        debug!(model = %self.model, "Sending streaming request to Moonshot");

        let (messages, _pending) = build_chat_messages(system_prompt, history);
        let openai_tools: Vec<ChatCompletionTools> = tools
            .iter()
            .map(|t| ChatCompletionTools::Function(tool_spec_to_chat(t)))
            .collect();

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder
            .model(&self.model)
            .messages(messages)
            .max_completion_tokens(self.max_tokens);
        if !openai_tools.is_empty() {
            builder.tools(openai_tools);
        }
        let request = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Moonshot streaming request: {e}"))?;

        let retry_config = RetryConfig::default();
        let mut stream = with_retry(
            &retry_config,
            "Moonshot",
            |e: &anyhow::Error| is_transient_error_message(&e.to_string()),
            || {
                let req = request.clone();
                let client = &self.client;
                async move {
                    client
                        .chat()
                        .create_stream(req)
                        .await
                        .map_err(|e| anyhow::anyhow!("Moonshot streaming request failed: {e}"))
                }
            },
        )
        .await?;

        let mut text_buf = String::new();
        let mut tool_map: Vec<(String, String, String)> = Vec::new();
        let mut model_name = String::new();
        let mut response_id = String::new();
        let mut finish_reason: Option<String> = None;
        let mut input_tokens: Option<u64> = None;
        let mut output_tokens: Option<u64> = None;

        while let Some(result) = stream.next().await {
            let chunk = result.map_err(|e| anyhow::anyhow!("Moonshot stream error: {e}"))?;

            if model_name.is_empty() {
                model_name.clone_from(&chunk.model);
            }
            if response_id.is_empty() {
                response_id.clone_from(&chunk.id);
            }
            if let Some(ref usage) = chunk.usage {
                input_tokens = Some(usage.prompt_tokens as u64);
                output_tokens = Some(usage.completion_tokens as u64);
            }

            for choice in &chunk.choices {
                if let Some(ref reason) = choice.finish_reason {
                    finish_reason = Some(format!("{reason:?}").to_lowercase());
                }

                let delta = &choice.delta;

                if let Some(ref content) = delta.content {
                    text_buf.push_str(content);
                    if let Some(ref sink) = token_sink {
                        let _ = sink.send(content.clone()).await;
                    }
                }

                if let Some(ref tc_chunks) = delta.tool_calls {
                    for tc in tc_chunks {
                        let idx = tc.index as usize;
                        while tool_map.len() <= idx {
                            tool_map.push((String::new(), String::new(), String::new()));
                        }
                        if let Some(ref id) = tc.id {
                            tool_map[idx].0.clone_from(id);
                        }
                        if let Some(ref func) = tc.function {
                            if let Some(ref name) = func.name {
                                tool_map[idx].1.clone_from(name);
                            }
                            if let Some(ref args) = func.arguments {
                                tool_map[idx].2.push_str(args);
                            }
                        }
                    }
                }
            }
        }

        debug!("Moonshot SSE stream complete");

        let meta = LlmResponseMeta {
            model: if model_name.is_empty() {
                None
            } else {
                Some(model_name)
            },
            response_id: if response_id.is_empty() {
                None
            } else {
                Some(response_id)
            },
            finish_reason,
            input_tokens,
            output_tokens,
        };

        if !tool_map.is_empty() {
            let items: Vec<ToolCallItem> = tool_map
                .into_iter()
                .filter(|(_, name, _)| !name.is_empty())
                .map(|(id, name, args_json)| {
                    let params = serde_json::from_str::<Value>(&args_json)
                        .unwrap_or(Value::Object(serde_json::Map::new()));
                    ToolCallItem {
                        name,
                        params,
                        id: if id.is_empty() { None } else { Some(id) },
                    }
                })
                .collect();
            if !items.is_empty() {
                debug!(count = items.len(), "Moonshot SSE: tool calls received");
                return Ok(LlmResponse::ToolCalls(items, meta));
            }
        }

        Ok(LlmResponse::FinalAnswer(text_buf, meta))
    }

    // ── Raw-HTTP chat (web search path) ───────────────────────────────────

    /// Send a chat request via raw HTTP, injecting the `$web_search` builtin
    /// and handling the echo-back loop internally.
    async fn chat_with_web_search(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        // Build the initial messages array.
        let mut messages = build_raw_messages(system_prompt, history);

        // Build tools: user-defined function tools + the builtin $web_search.
        let mut request_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.params_schema,
                    }
                })
            })
            .collect();

        // Inject $web_search builtin.
        request_tools.push(json!({
            "type": "builtin_function",
            "function": {
                "name": "$web_search"
            }
        }));

        // Echo-back loop: keep sending until we get a final answer or a
        // non-$web_search tool call.
        for round in 0..MAX_WEB_SEARCH_ROUNDS {
            debug!(round, "Moonshot web-search: sending request");

            // NOTE: Thinking mode must be disabled when $web_search is
            // active.  Moonshot's builtin does not return `reasoning_content`
            // in tool-call responses, causing the API to reject the echo-back
            // with "thinking is enabled but reasoning_content is missing".
            let body = json!({
                "model": self.model,
                "messages": messages,
                "tools": request_tools,
                "max_tokens": self.max_tokens,
                "thinking": { "type": "disabled" },
            });

            let resp = self
                .http_client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Moonshot request failed: {e}"))?;

            let status = resp.status();
            let resp_body: Value = resp
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Moonshot response parse failed: {e}"))?;

            if !status.is_success() {
                let err_msg = resp_body["error"]["message"]
                    .as_str()
                    .unwrap_or("unknown error");
                anyhow::bail!("Moonshot API error ({}): {}", status, err_msg);
            }

            let choice = resp_body["choices"]
                .get(0)
                .ok_or_else(|| anyhow::anyhow!("Moonshot returned empty choices"))?;

            let finish_reason = choice["finish_reason"].as_str().unwrap_or("");
            let message = &choice["message"];

            let meta = LlmResponseMeta {
                model: resp_body["model"].as_str().map(String::from),
                response_id: resp_body["id"].as_str().map(String::from),
                finish_reason: Some(finish_reason.to_string()),
                input_tokens: resp_body["usage"]["prompt_tokens"].as_u64(),
                output_tokens: resp_body["usage"]["completion_tokens"].as_u64(),
            };

            // Check for tool calls.
            if finish_reason == "tool_calls" {
                if let Some(tool_calls) = message["tool_calls"].as_array() {
                    let mut web_search_calls: Vec<&Value> = Vec::new();
                    let mut regular_calls: Vec<ToolCallItem> = Vec::new();

                    for tc in tool_calls {
                        let name = tc["function"]["name"].as_str().unwrap_or("");
                        if name == "$web_search" {
                            web_search_calls.push(tc);
                        } else {
                            let params: Value = serde_json::from_str(
                                tc["function"]["arguments"].as_str().unwrap_or("{}"),
                            )
                            .unwrap_or(json!({}));
                            regular_calls.push(ToolCallItem {
                                name: name.to_string(),
                                params,
                                id: tc["id"].as_str().map(String::from),
                            });
                        }
                    }

                    if !regular_calls.is_empty() {
                        debug!(
                            count = regular_calls.len(),
                            "Moonshot: regular tool calls received alongside web search"
                        );
                        return Ok(LlmResponse::ToolCalls(regular_calls, meta));
                    }

                    if !web_search_calls.is_empty() {
                        messages.push(message.clone());

                        for tc in &web_search_calls {
                            let call_id = tc["id"].as_str().unwrap_or("");
                            let arguments = tc["function"]["arguments"].as_str().unwrap_or("{}");

                            debug!(
                                call_id,
                                query = arguments,
                                "Moonshot: echoing $web_search arguments"
                            );

                            messages.push(json!({
                                "role": "tool",
                                "tool_call_id": call_id,
                                "name": "$web_search",
                                "content": arguments,
                            }));
                        }

                        continue;
                    }
                }
            }

            let content = message["content"].as_str().unwrap_or("").to_string();
            debug!("Moonshot web-search: final answer received");
            return Ok(LlmResponse::FinalAnswer(content, meta));
        }

        warn!("Moonshot web-search: max rounds ({MAX_WEB_SEARCH_ROUNDS}) exceeded");
        anyhow::bail!("Moonshot $web_search echo-back loop exceeded {MAX_WEB_SEARCH_ROUNDS} rounds")
    }
}

// ── LlmProvider ───────────────────────────────────────────────────────────────

#[async_trait]
impl LlmProvider for MoonshotProvider {
    fn capabilities(&self) -> Capabilities {
        let mut hosted_tools = Vec::new();
        if self.web_search_enabled {
            hosted_tools.push(HostedTool::WebSearch);
        }
        Capabilities {
            tools: ToolSupport::Native,
            streaming: true,
            vision: true,
            hosted_tools,
        }
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        if self.web_search_enabled {
            self.chat_with_web_search(system_prompt, history, tools)
                .await
        } else {
            self.chat_non_streaming(system_prompt, history, tools).await
        }
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        if self.web_search_enabled {
            // $web_search echo-back loop is not compatible with SSE streaming;
            // fall back to non-streaming.
            let result = self
                .chat_with_web_search(system_prompt, history, tools)
                .await?;
            if let LlmResponse::FinalAnswer(ref text, _) = result {
                if let Some(sink) = token_sink {
                    let _ = sink.send(text.clone()).await;
                }
            }
            Ok(result)
        } else {
            self.chat_sse(system_prompt, history, tools, token_sink)
                .await
        }
    }

    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!(
            "Moonshot AI does not support text embeddings"
        ))
    }

    fn provider_name(&self) -> &str {
        "moonshot"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn server_address(&self) -> &str {
        &self.base_url
    }
}

// ── HTTP client helper ────────────────────────────────────────────────────────

/// Build a `reqwest::Client` with the configured timeout for `async-openai`.
fn build_reqwest_client(timeout_secs: u64) -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))
}

// ── Chat Completions message conversion ───────────────────────────────────────

/// Build `async-openai` Chat Completions messages from conversation history.
fn build_chat_messages(
    system_prompt: &str,
    history: &[ChatHistoryMessage],
) -> (Vec<ChatCompletionRequestMessage>, Vec<(String, String)>) {
    let mut messages: Vec<ChatCompletionRequestMessage> = Vec::with_capacity(history.len() + 1);
    let mut pending_ids: Vec<(String, String)> = Vec::new();

    if !system_prompt.is_empty() {
        if let Ok(msg) = ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt)
            .build()
        {
            messages.push(ChatCompletionRequestMessage::System(msg));
        }
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
                    .map(|block| match block {
                        ContentBlock::Text(text) => json!({"type": "text", "text": text}),
                        ContentBlock::Image { media_type, data } => {
                            let data_uri = format!("data:{media_type};base64,{data}");
                            json!({
                                "type": "image_url",
                                "image_url": { "url": data_uri }
                            })
                        }
                    })
                    .collect();

                let msg_json = json!({"role": "user", "content": parts_json});
                if let Ok(msg) = serde_json::from_value::<ChatCompletionRequestMessage>(msg_json) {
                    messages.push(msg);
                }
            }

            ChatHistoryMessage::AssistantToolCalls(calls) => {
                pending_ids.clear();
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

/// Convert a [`ToolSpec`] to `async-openai` `ChatCompletionTool`.
fn tool_spec_to_chat(tool: &ToolSpec) -> ChatCompletionTool {
    let schema = &tool.params_schema;

    let parameters = if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
        schema.clone()
    } else if schema.as_object().is_some() {
        json!({"type": "object", "properties": schema})
    } else {
        json!({"type": "object", "properties": {}, "required": []})
    };

    let function = FunctionObjectArgs::default()
        .name(&tool.name)
        .description(&tool.description)
        .parameters(parameters)
        .build()
        .expect("FunctionObject build should not fail");

    ChatCompletionTool { function }
}

// ── Chat Completions response helpers ─────────────────────────────────────────

fn extract_chat_meta(model: &str, id: &str, usage: &Option<CompletionUsage>) -> LlmResponseMeta {
    LlmResponseMeta {
        model: Some(model.to_string()),
        response_id: Some(id.to_string()),
        input_tokens: usage.as_ref().map(|u| u.prompt_tokens as u64),
        output_tokens: usage.as_ref().map(|u| u.completion_tokens as u64),
        finish_reason: None,
    }
}

fn parse_tool_calls(tool_calls: &[ChatCompletionMessageToolCalls]) -> Vec<ToolCallItem> {
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

// ── Raw-HTTP message conversion (web-search path) ─────────────────────────────

fn build_raw_messages(system_prompt: &str, history: &[ChatHistoryMessage]) -> Vec<Value> {
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
                    .map(|block| match block {
                        ContentBlock::Text(text) => json!({"type": "text", "text": text}),
                        ContentBlock::Image { media_type, data } => {
                            let data_uri = format!("data:{media_type};base64,{data}");
                            json!({"type": "image_url", "image_url": {"url": data_uri}})
                        }
                    })
                    .collect();
                messages.push(json!({"role": "user", "content": parts}));
            }

            ChatHistoryMessage::AssistantToolCalls(calls) => {
                pending_ids.clear();
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn default_constants_are_sensible() {
        assert_eq!(DEFAULT_BASE_URL, "https://api.moonshot.ai/v1");
        assert!(DEFAULT_MAX_TOKENS > 0);
    }

    // ── async-openai message builder tests ────────────────────────────────

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

    // ── Raw-HTTP message builder tests ────────────────────────────────────

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

    // ── Capabilities ──────────────────────────────────────────────────────

    #[test]
    fn capabilities_with_web_search_enabled() {
        let p = MoonshotProvider::new(
            "kimi-k2.5".into(),
            DEFAULT_BASE_URL.into(),
            "test-key",
            120,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .unwrap();
        assert!(p
            .capabilities()
            .hosted_tools
            .contains(&HostedTool::WebSearch));
    }

    #[test]
    fn capabilities_without_web_search() {
        let p = MoonshotProvider::new(
            "kimi-k2.5".into(),
            DEFAULT_BASE_URL.into(),
            "test-key",
            120,
            DEFAULT_MAX_TOKENS,
            false,
        )
        .unwrap();
        assert!(!p
            .capabilities()
            .hosted_tools
            .contains(&HostedTool::WebSearch));
    }

    // ── Web-search echo-back tests (wiremock) ─────────────────────────────

    fn ws_tool_call_response(call_id: &str, query: &str) -> Value {
        json!({
            "id": "resp_001", "model": "kimi-k2.5",
            "choices": [{"index": 0, "finish_reason": "tool_calls", "message": {
                "role": "assistant", "content": "",
                "tool_calls": [{"id": call_id, "type": "function", "function": {
                    "name": "$web_search",
                    "arguments": json!({"query": query}).to_string()
                }}]
            }}],
            "usage": {"prompt_tokens": 50, "completion_tokens": 10}
        })
    }

    fn final_answer(content: &str) -> Value {
        json!({
            "id": "resp_002", "model": "kimi-k2.5",
            "choices": [{"index": 0, "finish_reason": "stop", "message": {
                "role": "assistant", "content": content
            }}],
            "usage": {"prompt_tokens": 100, "completion_tokens": 30}
        })
    }

    #[tokio::test]
    async fn web_search_echo_back_loop() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(ws_tool_call_response("call_ws_1", "latest AI news")),
            )
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(final_answer("Here are the latest AI news...")),
            )
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;

        let p = MoonshotProvider::new(
            "kimi-k2.5".into(),
            server.uri(),
            "test-key",
            30,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .unwrap();

        match p.chat("You are helpful.", &[], &[]).await.unwrap() {
            LlmResponse::FinalAnswer(text, meta) => {
                assert_eq!(text, "Here are the latest AI news...");
                assert_eq!(meta.model.as_deref(), Some("kimi-k2.5"));
            }
            other => panic!("expected FinalAnswer, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn web_search_regular_tool_calls_returned_to_caller() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "resp_003", "model": "kimi-k2.5",
                "choices": [{"index": 0, "finish_reason": "tool_calls", "message": {
                    "role": "assistant", "content": "",
                    "tool_calls": [{"id": "call_regular", "type": "function", "function": {
                        "name": "file-read",
                        "arguments": "{\"path\": \"/tmp/foo.txt\"}"
                    }}]
                }}],
                "usage": {"prompt_tokens": 50, "completion_tokens": 10}
            })))
            .expect(1)
            .mount(&server)
            .await;

        let p = MoonshotProvider::new(
            "kimi-k2.5".into(),
            server.uri(),
            "test-key",
            30,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .unwrap();

        let spec = ToolSpec {
            name: "file-read".into(),
            description: "Read a file".into(),
            params_schema: json!({"type": "object", "properties": {"path": {"type": "string"}}}),
            is_mutating: false,
            requires_confirmation: false,
        };

        match p.chat("You are helpful.", &[], &[spec]).await.unwrap() {
            LlmResponse::ToolCalls(calls, _) => {
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].name, "file-read");
            }
            other => panic!("expected ToolCalls, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn web_search_direct_answer_no_search() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(final_answer("2+2 is 4")))
            .expect(1)
            .mount(&server)
            .await;

        let p = MoonshotProvider::new(
            "kimi-k2.5".into(),
            server.uri(),
            "test-key",
            30,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .unwrap();

        match p.chat("You are helpful.", &[], &[]).await.unwrap() {
            LlmResponse::FinalAnswer(text, _) => assert_eq!(text, "2+2 is 4"),
            other => panic!("expected FinalAnswer, got {other:?}"),
        }
    }

    #[tokio::test]
    #[ignore = "requires MOONSHOT_API_KEY"]
    async fn live_web_search() {
        let api_key = std::env::var("MOONSHOT_API_KEY").expect("MOONSHOT_API_KEY must be set");
        let p = MoonshotProvider::new(
            "kimi-k2.5".into(),
            DEFAULT_BASE_URL.into(),
            &api_key,
            60,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .unwrap();

        let history = vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "What is today's date? Use web search to confirm.".into(),
        }];

        match p.chat("Be concise.", &history, &[]).await.unwrap() {
            LlmResponse::FinalAnswer(text, meta) => {
                eprintln!("Model: {:?}, Answer: {text}", meta.model);
                assert!(!text.is_empty());
            }
            other => panic!("expected FinalAnswer but got {other:?}"),
        }
    }
}
