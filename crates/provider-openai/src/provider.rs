//! `OpenAIProvider` — [`LlmProvider`] implementation backed by the OpenAI Chat Completions API.
//!
//! Supports two authentication modes:
//! - **API key** — standard `OPENAI_API_KEY` bearer token.
//! - **OAuth PKCE** — Codex subscription via ChatGPT sign-in.

use std::sync::Arc;

use async_openai::config::OpenAIConfig as AsyncOpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionTools, CompletionUsage,
    CreateChatCompletionRequestArgs, FunctionCall, FunctionObjectArgs, WebSearchContextSize,
    WebSearchLocation, WebSearchOptions, WebSearchUserLocation, WebSearchUserLocationType,
};
use async_openai::types::embeddings::CreateEmbeddingRequestArgs;
use async_openai::Client;
use async_trait::async_trait;
use futures::StreamExt as _;
use serde_json::Value;
use tokio::sync::{mpsc, RwLock};
use tracing::debug;

use assistant_core::types::OpenAIUserLocation;
use assistant_core::LlmConfig;
use assistant_llm::{
    Capabilities, ChatHistoryMessage, ChatRole, ContentBlock, HostedTool, LlmProvider, LlmResponse,
    LlmResponseMeta, ToolCallItem, ToolSpec, ToolSupport,
};

use crate::oauth::OAuthManager;

// ── OpenAIProviderConfig ──────────────────────────────────────────────────────

/// Configuration for the OpenAI backend.
#[derive(Debug, Clone)]
pub struct OpenAIProviderConfig {
    /// Model ID, e.g. `"gpt-4o"`, `"gpt-4.1"`.
    pub model: String,
    /// Base URL (default: `"https://api.openai.com/v1"`).
    pub base_url: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// `max_completion_tokens` sent in every request.
    pub max_tokens: u32,
    /// Embedding model for vector search.
    pub embedding_model: String,
    /// Hosted web-search config (Chat Completions `web_search_options`).
    pub web_search: Option<OpenAIWebSearchConfig>,
}

impl Default for OpenAIProviderConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            timeout_secs: 120,
            max_tokens: 8192,
            embedding_model: "text-embedding-3-small".to_string(),
            web_search: None,
        }
    }
}

/// Resolved web-search configuration for the OpenAI provider.
#[derive(Debug, Clone)]
pub struct OpenAIWebSearchConfig {
    pub search_context_size: Option<String>,
    pub user_location: Option<OpenAIUserLocation>,
}

// ── OpenAIProvider ────────────────────────────────────────────────────────────

/// [`LlmProvider`] backed by the OpenAI Chat Completions API.
///
/// Uses `async-openai` for HTTP transport.  For OAuth mode the internal
/// `Client` is recreated whenever the access token is refreshed.
pub struct OpenAIProvider {
    /// Current async-openai client.  Held behind a lock so OAuth token
    /// refreshes can swap in a new client atomically.
    client: RwLock<Client<AsyncOpenAIConfig>>,
    config: OpenAIProviderConfig,
    /// OAuth manager — `None` for API-key mode.
    oauth: Option<Arc<OAuthManager>>,
}

impl OpenAIProvider {
    /// Build from explicit config + an initial API key or OAuth token.
    pub fn new(config: OpenAIProviderConfig, api_key: &str) -> anyhow::Result<Self> {
        let oai_cfg = AsyncOpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(&config.base_url);
        let client = Client::with_config(oai_cfg);

        Ok(Self {
            client: RwLock::new(client),
            config,
            oauth: None,
        })
    }

    /// Build from explicit config + OAuth manager.
    pub fn new_with_oauth(
        config: OpenAIProviderConfig,
        oauth: OAuthManager,
    ) -> anyhow::Result<Self> {
        // Use a placeholder key; `ensure_fresh_client` will swap in the real token.
        let oai_cfg = AsyncOpenAIConfig::new()
            .with_api_key("oauth-placeholder")
            .with_api_base(&config.base_url);
        let client = Client::with_config(oai_cfg);

        Ok(Self {
            client: RwLock::new(client),
            config,
            oauth: Some(Arc::new(oauth)),
        })
    }

    /// Convenience constructor directly from [`LlmConfig`].
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        let base_url = normalise_base_url(&cfg.base_url);
        let max_tokens = cfg.openai.max_tokens.unwrap_or(8192);

        let web_search = if cfg.openai.web_search.enabled {
            Some(OpenAIWebSearchConfig {
                search_context_size: cfg.openai.web_search.search_context_size.clone(),
                user_location: cfg.openai.web_search.user_location.clone(),
            })
        } else {
            None
        };

        let provider_cfg = OpenAIProviderConfig {
            model: cfg.model.clone(),
            base_url,
            timeout_secs: cfg.timeout_secs,
            max_tokens,
            embedding_model: cfg.embedding_model.clone(),
            web_search,
        };

        match cfg.openai.auth_mode {
            assistant_core::OpenAIAuthMode::ApiKey => {
                let api_key = cfg
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "OpenAI API key not found. Set api_key in [llm] config or \
                             OPENAI_API_KEY environment variable."
                        )
                    })?;
                Self::new(provider_cfg, &api_key)
            }
            assistant_core::OpenAIAuthMode::OAuth => {
                let client_id = cfg.openai.oauth_client_id.clone().unwrap_or_default();
                anyhow::ensure!(
                    !client_id.is_empty(),
                    "OAuth mode requires openai.oauth_client_id in config"
                );
                let oauth = OAuthManager::new(client_id)?;
                Self::new_with_oauth(provider_cfg, oauth)
            }
        }
    }

    // ── Token management ──────────────────────────────────────────────────

    /// Ensure the client holds a valid token.  No-op for API-key mode.
    async fn ensure_fresh_client(&self) -> anyhow::Result<()> {
        let Some(ref oauth) = self.oauth else {
            return Ok(());
        };

        let token = oauth.ensure_valid_token().await?;

        let oai_cfg = AsyncOpenAIConfig::new()
            .with_api_key(&token)
            .with_api_base(&self.config.base_url);
        *self.client.write().await = Client::with_config(oai_cfg);

        Ok(())
    }

    // ── Non-streaming chat ────────────────────────────────────────────────

    async fn chat_non_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        self.ensure_fresh_client().await?;

        debug!(model = %self.config.model, tools = tools.len(), "Sending request to OpenAI");

        let (messages, _pending) = build_openai_messages(system_prompt, history);
        let openai_tools: Vec<ChatCompletionTools> = tools
            .iter()
            .map(|t| ChatCompletionTools::Function(tool_spec_to_openai(t)))
            .collect();

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder
            .model(&self.config.model)
            .messages(messages)
            .max_completion_tokens(self.config.max_tokens);
        if !openai_tools.is_empty() {
            builder.tools(openai_tools);
        }
        if let Some(ref ws) = self.config.web_search {
            builder.web_search_options(build_web_search_options(ws));
        }
        let request = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build OpenAI request: {e}"))?;

        let client = self.client.read().await;
        let response = client
            .chat()
            .create(request)
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI request failed: {e}"))?;

        debug!("OpenAI response received");

        let meta = extract_response_meta(&response.model, &response.id, &response.usage);

        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("OpenAI returned empty choices"))?;

        // Check for tool calls.
        if let Some(ref tool_calls) = choice.message.tool_calls {
            let items = parse_tool_calls_enum(tool_calls);
            if !items.is_empty() {
                debug!(count = items.len(), "OpenAI: tool calls received");
                return Ok(LlmResponse::ToolCalls(items, meta));
            }
        }

        let content = choice.message.content.as_deref().unwrap_or("").to_string();
        debug!("OpenAI: final answer received");
        Ok(LlmResponse::FinalAnswer(content, meta))
    }

    // ── SSE streaming chat ────────────────────────────────────────────────

    async fn chat_sse(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        self.ensure_fresh_client().await?;

        debug!(model = %self.config.model, "Sending streaming request to OpenAI");

        let (messages, _pending) = build_openai_messages(system_prompt, history);
        let openai_tools: Vec<ChatCompletionTools> = tools
            .iter()
            .map(|t| ChatCompletionTools::Function(tool_spec_to_openai(t)))
            .collect();

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder
            .model(&self.config.model)
            .messages(messages)
            .max_completion_tokens(self.config.max_tokens);
        if !openai_tools.is_empty() {
            builder.tools(openai_tools);
        }
        if let Some(ref ws) = self.config.web_search {
            builder.web_search_options(build_web_search_options(ws));
        }
        let request = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build OpenAI streaming request: {e}"))?;

        let client = self.client.read().await;
        let mut stream = client
            .chat()
            .create_stream(request)
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI streaming request failed: {e}"))?;

        // Accumulation state.
        let mut text_buf = String::new();
        // tool_call_index → (id, name, arguments_buf)
        let mut tool_map: Vec<(String, String, String)> = Vec::new();
        let mut model_name = String::new();
        let mut response_id = String::new();
        let mut finish_reason: Option<String> = None;
        let mut input_tokens: Option<u64> = None;
        let mut output_tokens: Option<u64> = None;

        while let Some(result) = stream.next().await {
            let chunk = result.map_err(|e| anyhow::anyhow!("OpenAI stream error: {e}"))?;

            if model_name.is_empty() {
                model_name.clone_from(&chunk.model);
            }
            if response_id.is_empty() {
                response_id.clone_from(&chunk.id);
            }

            // Extract usage from the final chunk (OpenAI sends it when stream_options.include_usage is set,
            // but also in the last chunk of some API versions).
            if let Some(ref usage) = chunk.usage {
                input_tokens = Some(usage.prompt_tokens as u64);
                output_tokens = Some(usage.completion_tokens as u64);
            }

            for choice in &chunk.choices {
                if let Some(ref reason) = choice.finish_reason {
                    finish_reason = Some(format!("{reason:?}").to_lowercase());
                }

                let delta = &choice.delta;

                // Text content.
                if let Some(ref content) = delta.content {
                    text_buf.push_str(content);
                    if let Some(ref sink) = token_sink {
                        let _ = sink.send(content.clone()).await;
                    }
                }

                // Tool call chunks.
                if let Some(ref tc_chunks) = delta.tool_calls {
                    for tc in tc_chunks {
                        let idx = tc.index as usize;
                        // Extend the tool_map to fit this index.
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

        debug!("OpenAI SSE stream complete");

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

        // Priority: tool calls > text.
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
                debug!(count = items.len(), "OpenAI SSE: tool calls received");
                return Ok(LlmResponse::ToolCalls(items, meta));
            }
        }

        Ok(LlmResponse::FinalAnswer(text_buf, meta))
    }
}

// ── LlmProvider impl ─────────────────────────────────────────────────────────

#[async_trait]
impl LlmProvider for OpenAIProvider {
    fn capabilities(&self) -> Capabilities {
        let mut hosted_tools = Vec::new();
        if self.config.web_search.is_some() {
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
        self.chat_non_streaming(system_prompt, history, tools).await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        self.chat_sse(system_prompt, history, tools, token_sink)
            .await
    }

    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        self.ensure_fresh_client().await?;

        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.config.embedding_model)
            .input(text)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build embedding request: {e}"))?;

        let client = self.client.read().await;
        let response = client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI embedding request failed: {e}"))?;

        let embedding = response
            .data
            .first()
            .ok_or_else(|| anyhow::anyhow!("OpenAI returned empty embedding data"))?;

        Ok(embedding.embedding.clone())
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn server_address(&self) -> &str {
        &self.config.base_url
    }
}

// ── Message conversion helpers ────────────────────────────────────────────────

/// Build the OpenAI-format messages array from conversation history.
///
/// Returns `(messages, pending_ids)` where `pending_ids` tracks
/// `(tool_name, tool_call_id)` pairs from the most recent
/// `AssistantToolCalls` block (used to match `ToolResult` messages).
fn build_openai_messages(
    system_prompt: &str,
    history: &[ChatHistoryMessage],
) -> (Vec<ChatCompletionRequestMessage>, Vec<(String, String)>) {
    let mut messages: Vec<ChatCompletionRequestMessage> = Vec::with_capacity(history.len() + 1);
    let mut pending_ids: Vec<(String, String)> = Vec::new();

    // System prompt.
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
                    // Bare tool-role text — shouldn't happen normally but handle gracefully.
                    if let Ok(msg) = ChatCompletionRequestUserMessageArgs::default()
                        .content(content.as_str())
                        .build()
                    {
                        messages.push(ChatCompletionRequestMessage::User(msg));
                    }
                }
            },

            ChatHistoryMessage::MultimodalUser { content } => {
                // Build content parts for the OpenAI API.
                // Images are sent as data URIs: data:<media_type>;base64,<data>
                let parts_json: Vec<Value> = content
                    .iter()
                    .map(|block| match block {
                        ContentBlock::Text(text) => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        ContentBlock::Image { media_type, data } => {
                            let data_uri = format!("data:{media_type};base64,{data}");
                            serde_json::json!({
                                "type": "image_url",
                                "image_url": { "url": data_uri }
                            })
                        }
                    })
                    .collect();

                // Construct via serde round-trip to avoid fighting content part types.
                let msg_json = serde_json::json!({
                    "role": "user",
                    "content": parts_json,
                });
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
                // Consume the first matching pending entry.
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

/// Convert a [`ToolSpec`] to the OpenAI `ChatCompletionTool` struct.
fn tool_spec_to_openai(tool: &ToolSpec) -> ChatCompletionTool {
    let schema = &tool.params_schema;

    let parameters = if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
        schema.clone()
    } else if schema.as_object().is_some() {
        serde_json::json!({"type": "object", "properties": schema})
    } else {
        serde_json::json!({"type": "object", "properties": {}, "required": []})
    };

    let function = FunctionObjectArgs::default()
        .name(&tool.name)
        .description(&tool.description)
        .parameters(parameters)
        .build()
        .expect("FunctionObject build should not fail");

    ChatCompletionTool { function }
}

// ── Response helpers ──────────────────────────────────────────────────────────

/// Extract [`LlmResponseMeta`] from a non-streaming response.
fn extract_response_meta(
    model: &str,
    id: &str,
    usage: &Option<CompletionUsage>,
) -> LlmResponseMeta {
    LlmResponseMeta {
        model: Some(model.to_string()),
        response_id: Some(id.to_string()),
        input_tokens: usage.as_ref().map(|u| u.prompt_tokens as u64),
        output_tokens: usage.as_ref().map(|u| u.completion_tokens as u64),
        finish_reason: None, // set from choice below if needed
    }
}

/// Parse tool calls from a non-streaming response message.
///
/// The response uses `ChatCompletionMessageToolCalls` enum which wraps
/// the actual `ChatCompletionMessageToolCall` struct.
fn parse_tool_calls_enum(tool_calls: &[ChatCompletionMessageToolCalls]) -> Vec<ToolCallItem> {
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

// ── Web search helpers ────────────────────────────────────────────────────────

/// Convert the provider-level web search config into the `async-openai` typed
/// [`WebSearchOptions`] that gets serialised on the Chat Completions request.
fn build_web_search_options(cfg: &OpenAIWebSearchConfig) -> WebSearchOptions {
    let search_context_size = cfg.search_context_size.as_deref().and_then(|s| match s {
        "low" => Some(WebSearchContextSize::Low),
        "medium" => Some(WebSearchContextSize::Medium),
        "high" => Some(WebSearchContextSize::High),
        _ => None,
    });

    let user_location = cfg.user_location.as_ref().map(|loc| WebSearchUserLocation {
        r#type: WebSearchUserLocationType::Approximate,
        approximate: WebSearchLocation {
            country: loc.country.clone(),
            city: loc.city.clone(),
            region: loc.region.clone(),
            timezone: loc.timezone.clone(),
        },
    });

    WebSearchOptions {
        search_context_size,
        user_location,
    }
}

// ── URL normalisation ─────────────────────────────────────────────────────────

/// Ensure the base URL ends with `/v1` for the OpenAI API.
fn normalise_base_url(url: &str) -> String {
    // If the user set the default Ollama URL, replace with OpenAI default.
    if url == "http://localhost:11434" {
        return "https://api.openai.com/v1".to_string();
    }
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_llm::{ChatHistoryMessage, ChatRole, ToolCallItem};

    #[test]
    fn normalise_base_url_adds_v1() {
        assert_eq!(
            normalise_base_url("https://api.openai.com"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn normalise_base_url_keeps_existing_v1() {
        assert_eq!(
            normalise_base_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn normalise_base_url_replaces_ollama_default() {
        assert_eq!(
            normalise_base_url("http://localhost:11434"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn build_messages_system_prompt() {
        let (msgs, _) = build_openai_messages("You are helpful.", &[]);
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn build_messages_text_user_assistant() {
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
        let (msgs, _) = build_openai_messages("sys", &history);
        // system + user + assistant
        assert_eq!(msgs.len(), 3);
    }

    #[test]
    fn build_messages_tool_calls_and_results() {
        let history = vec![
            ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
                name: "my-tool".to_string(),
                params: serde_json::json!({"key": "val"}),
                id: Some("call_123".to_string()),
            }]),
            ChatHistoryMessage::ToolResult {
                name: "my-tool".to_string(),
                content: "result".to_string(),
            },
        ];
        let (msgs, pending) = build_openai_messages("", &history);
        // assistant (tool_calls) + tool (result)
        assert_eq!(msgs.len(), 2);
        // pending_ids should be consumed
        assert!(pending.is_empty());
    }

    #[test]
    fn tool_spec_conversion() {
        let spec = ToolSpec {
            name: "file-read".to_string(),
            description: "Read a file".to_string(),
            params_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
            is_mutating: false,
            requires_confirmation: false,
        };
        let tool = tool_spec_to_openai(&spec);
        assert_eq!(tool.function.name, "file-read");
        assert_eq!(tool.function.description, Some("Read a file".to_string()));
    }

    // ── Web search option tests ───────────────────────────────────────────

    #[test]
    fn build_web_search_options_defaults() {
        let cfg = OpenAIWebSearchConfig {
            search_context_size: None,
            user_location: None,
        };
        let opts = build_web_search_options(&cfg);
        assert!(opts.search_context_size.is_none());
        assert!(opts.user_location.is_none());
    }

    #[test]
    fn build_web_search_options_with_context_size() {
        for (input, expected) in [
            ("low", WebSearchContextSize::Low),
            ("medium", WebSearchContextSize::Medium),
            ("high", WebSearchContextSize::High),
        ] {
            let cfg = OpenAIWebSearchConfig {
                search_context_size: Some(input.to_string()),
                user_location: None,
            };
            let opts = build_web_search_options(&cfg);
            assert_eq!(opts.search_context_size, Some(expected));
        }
    }

    #[test]
    fn build_web_search_options_invalid_context_size_ignored() {
        let cfg = OpenAIWebSearchConfig {
            search_context_size: Some("ultra".to_string()),
            user_location: None,
        };
        let opts = build_web_search_options(&cfg);
        assert!(opts.search_context_size.is_none());
    }

    #[test]
    fn build_web_search_options_with_user_location() {
        let cfg = OpenAIWebSearchConfig {
            search_context_size: None,
            user_location: Some(OpenAIUserLocation {
                country: Some("GB".to_string()),
                city: Some("London".to_string()),
                region: Some("London".to_string()),
                timezone: Some("Europe/London".to_string()),
            }),
        };
        let opts = build_web_search_options(&cfg);
        let loc = opts.user_location.expect("user_location should be set");
        assert_eq!(loc.approximate.country, Some("GB".to_string()));
        assert_eq!(loc.approximate.city, Some("London".to_string()));
    }

    #[test]
    fn capabilities_without_web_search() {
        let provider =
            OpenAIProvider::new(OpenAIProviderConfig::default(), "test-key").expect("should build");
        let caps = provider.capabilities();
        assert!(caps.hosted_tools.is_empty());
    }

    #[test]
    fn capabilities_with_web_search() {
        let cfg = OpenAIProviderConfig {
            web_search: Some(OpenAIWebSearchConfig {
                search_context_size: None,
                user_location: None,
            }),
            ..Default::default()
        };
        let provider = OpenAIProvider::new(cfg, "test-key").expect("should build");
        let caps = provider.capabilities();
        assert_eq!(caps.hosted_tools, vec![HostedTool::WebSearch]);
    }
}
