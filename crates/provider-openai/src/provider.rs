//! `OpenAIProvider` — [`LlmProvider`] implementation backed by the OpenAI **Responses API**.
//!
//! Migrated from the Chat Completions API to gain native `web_search` tool
//! support on all modern models (gpt-5, gpt-4o, o4-mini, …) rather than being
//! restricted to the three `*-search-preview` models.
//!
//! Supports two authentication modes:
//! - **API key** — standard `OPENAI_API_KEY` bearer token.
//! - **OAuth PKCE** — Codex subscription via ChatGPT sign-in.

use std::sync::Arc;
use std::time::Duration;

use async_openai::config::OpenAIConfig as AsyncOpenAIConfig;
use async_openai::types::embeddings::CreateEmbeddingRequestArgs;
use async_openai::types::responses::{
    CreateResponseArgs, EasyInputMessage, FunctionCallOutput, FunctionCallOutputItemParam,
    FunctionTool, FunctionToolCall, InputContent, InputItem, InputParam, InputTextContent, Item,
    MessageType, OutputItem, OutputMessage, OutputMessageContent, OutputStatus, Response, Role,
    Tool, WebSearchApproximateLocation, WebSearchApproximateLocationType, WebSearchTool,
    WebSearchToolSearchContextSize,
};
use async_openai::Client;
use async_trait::async_trait;
use futures::StreamExt as _;
use serde_json::Value;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

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
    /// `max_output_tokens` sent in every request.
    pub max_tokens: u32,
    /// Embedding model for vector search.
    pub embedding_model: String,
    /// Hosted web-search config (Responses API `Tool::WebSearch`).
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

/// [`LlmProvider`] backed by the OpenAI **Responses API**.
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
        let client = Client::with_config(oai_cfg)
            .with_http_client(build_reqwest_client(config.timeout_secs)?);

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
        let client = Client::with_config(oai_cfg)
            .with_http_client(build_reqwest_client(config.timeout_secs)?);

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
        *self.client.write().await = Client::with_config(oai_cfg)
            .with_http_client(build_reqwest_client(self.config.timeout_secs)?);

        Ok(())
    }

    // ── Responses API tools ───────────────────────────────────────────────

    /// Build the `tools` array for the Responses API request.
    ///
    /// Includes function tools for the assistant's builtin tools, plus
    /// `Tool::WebSearch` when web search is configured.
    fn build_tools(&self, tool_specs: &[ToolSpec]) -> Vec<Tool> {
        let mut tools: Vec<Tool> = tool_specs.iter().map(tool_spec_to_responses).collect();

        if let Some(ref ws) = self.config.web_search {
            tools.push(build_web_search_tool(ws));
        }

        tools
    }

    // ── Non-streaming chat ────────────────────────────────────────────────

    async fn chat_non_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        self.ensure_fresh_client().await?;

        debug!(model = %self.config.model, tools = tools.len(), "Sending Responses API request to OpenAI");

        let input_items = build_input_items(history);
        let api_tools = self.build_tools(tools);

        let mut builder = CreateResponseArgs::default();
        builder
            .model(&self.config.model)
            .input(InputParam::Items(input_items))
            .max_output_tokens(self.config.max_tokens)
            .store(false);

        if !system_prompt.is_empty() {
            builder.instructions(system_prompt);
        }
        if !api_tools.is_empty() {
            builder.tools(api_tools);
        }

        let request = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build OpenAI Responses request: {e}"))?;

        let client = self.client.read().await;
        let response = client
            .responses()
            .create(request)
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI Responses request failed: {e}"))?;

        debug!("OpenAI Responses API response received");

        parse_response(response)
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

        debug!(model = %self.config.model, "Sending streaming Responses API request to OpenAI");

        let input_items = build_input_items(history);
        let api_tools = self.build_tools(tools);

        let mut builder = CreateResponseArgs::default();
        builder
            .model(&self.config.model)
            .input(InputParam::Items(input_items))
            .max_output_tokens(self.config.max_tokens)
            .store(false);

        if !system_prompt.is_empty() {
            builder.instructions(system_prompt);
        }
        if !api_tools.is_empty() {
            builder.tools(api_tools);
        }

        let request = builder.build().map_err(|e| {
            anyhow::anyhow!("Failed to build OpenAI streaming Responses request: {e}")
        })?;

        let client = self.client.read().await;
        let mut stream = client
            .responses()
            .create_stream(request)
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI streaming Responses request failed: {e}"))?;

        // Accumulate text deltas and watch for the completed response.
        let mut text_buf = String::new();
        let mut completed_response: Option<Response> = None;

        while let Some(result) = stream.next().await {
            let event =
                result.map_err(|e| anyhow::anyhow!("OpenAI Responses stream error: {e}"))?;

            use async_openai::types::responses::ResponseStreamEvent;
            match event {
                ResponseStreamEvent::ResponseOutputTextDelta(delta) => {
                    text_buf.push_str(&delta.delta);
                    if let Some(ref sink) = token_sink {
                        let _ = sink.send(delta.delta).await;
                    }
                }
                ResponseStreamEvent::ResponseCompleted(completed) => {
                    completed_response = Some(completed.response);
                }
                // Ignore all other event types — we extract the final result
                // from the completed response.
                _ => {}
            }
        }

        debug!("OpenAI Responses SSE stream complete");

        // Prefer the completed response (it has structured output items + usage).
        if let Some(response) = completed_response {
            return parse_response(response);
        }

        // Fallback: no completed event (shouldn't happen normally).
        warn!("OpenAI Responses stream ended without a completed event, using buffered text");
        Ok(LlmResponse::FinalAnswer(
            text_buf,
            LlmResponseMeta::default(),
        ))
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

// ── Input conversion helpers ──────────────────────────────────────────────────

/// Build the Responses API input items from conversation history.
///
/// Maps our internal [`ChatHistoryMessage`] types to the Responses API
/// [`InputItem`] format.
fn build_input_items(history: &[ChatHistoryMessage]) -> Vec<InputItem> {
    let mut items: Vec<InputItem> = Vec::with_capacity(history.len());

    for entry in history {
        match entry {
            ChatHistoryMessage::Text { role, content } => {
                let api_role = match role {
                    ChatRole::System => Role::Developer,
                    ChatRole::User => Role::User,
                    ChatRole::Assistant => Role::Assistant,
                    ChatRole::Tool => {
                        // Bare tool-role text — shouldn't happen normally.
                        // Send as user message to avoid dropping it.
                        Role::User
                    }
                };
                items.push(InputItem::EasyMessage(EasyInputMessage {
                    r#type: MessageType::Message,
                    role: api_role,
                    content: async_openai::types::responses::EasyInputContent::Text(
                        content.clone(),
                    ),
                }));
            }

            ChatHistoryMessage::MultimodalUser { content } => {
                let parts: Vec<InputContent> = content
                    .iter()
                    .map(|block| match block {
                        ContentBlock::Text(text) => {
                            InputContent::InputText(InputTextContent { text: text.clone() })
                        }
                        ContentBlock::Image { media_type, data } => {
                            let data_uri = format!("data:{media_type};base64,{data}");
                            InputContent::InputImage(
                                async_openai::types::responses::InputImageContent {
                                    detail: async_openai::types::responses::ImageDetail::Auto,
                                    file_id: None,
                                    image_url: Some(data_uri),
                                },
                            )
                        }
                    })
                    .collect();

                items.push(InputItem::EasyMessage(EasyInputMessage {
                    r#type: MessageType::Message,
                    role: Role::User,
                    content: async_openai::types::responses::EasyInputContent::ContentList(parts),
                }));
            }

            ChatHistoryMessage::AssistantToolCalls(calls) => {
                // Each tool call becomes a FunctionCall item that we feed back
                // as input for conversation context.
                for call in calls {
                    let call_id = call
                        .id
                        .clone()
                        .unwrap_or_else(|| "call_unknown".to_string());
                    items.push(InputItem::Item(Item::FunctionCall(FunctionToolCall {
                        arguments: serde_json::to_string(&call.params)
                            .unwrap_or_else(|_| "{}".to_string()),
                        call_id: call_id.clone(),
                        name: call.name.clone(),
                        id: None,
                        status: Some(OutputStatus::Completed),
                    })));
                }
            }

            ChatHistoryMessage::ToolResult { name: _, content } => {
                // Find the call_id for the most recent unmatched FunctionCall.
                let call_id = find_preceding_call_id(&items);
                items.push(InputItem::Item(Item::FunctionCallOutput(
                    FunctionCallOutputItemParam {
                        call_id,
                        output: FunctionCallOutput::Text(content.clone()),
                        id: None,
                        status: None,
                    },
                )));
            }
        }
    }

    items
}

/// Find the call_id for the most recent unmatched FunctionCall item.
///
/// The Responses API requires `FunctionCallOutput.call_id` to match a preceding
/// `FunctionCall.call_id`. We track which call_ids have already been consumed
/// by a FunctionCallOutput and return the first unmatched one.
fn find_preceding_call_id(items: &[InputItem]) -> String {
    let mut function_call_ids: Vec<String> = Vec::new();
    let mut matched_ids: Vec<String> = Vec::new();

    for item in items {
        match item {
            InputItem::Item(Item::FunctionCall(fc)) => {
                function_call_ids.push(fc.call_id.clone());
            }
            InputItem::Item(Item::FunctionCallOutput(fco)) => {
                matched_ids.push(fco.call_id.clone());
            }
            _ => {}
        }
    }

    // Find the first unmatched call_id
    for id in &function_call_ids {
        if !matched_ids.contains(id) {
            return id.clone();
        }
    }

    // Fallback — shouldn't happen in well-formed conversations
    "call_unknown".to_string()
}

// ── Tool conversion helpers ───────────────────────────────────────────────────

/// Convert a [`ToolSpec`] to the Responses API `Tool::Function`.
fn tool_spec_to_responses(tool: &ToolSpec) -> Tool {
    let schema = &tool.params_schema;

    let parameters = if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
        Some(schema.clone())
    } else if schema.as_object().is_some() {
        Some(serde_json::json!({"type": "object", "properties": schema}))
    } else {
        Some(serde_json::json!({"type": "object", "properties": {}, "required": []}))
    };

    Tool::Function(FunctionTool {
        name: tool.name.clone(),
        description: Some(tool.description.clone()),
        parameters,
        strict: Some(false),
    })
}

// ── Web search helpers ────────────────────────────────────────────────────────

/// Build a `Tool::WebSearch` from the provider-level config.
///
/// The Responses API web search works with all models (gpt-5, gpt-4o,
/// o4-mini, etc.) — the model decides per-turn whether to search.
fn build_web_search_tool(cfg: &OpenAIWebSearchConfig) -> Tool {
    let search_context_size = cfg.search_context_size.as_deref().and_then(|s| match s {
        "low" => Some(WebSearchToolSearchContextSize::Low),
        "medium" => Some(WebSearchToolSearchContextSize::Medium),
        "high" => Some(WebSearchToolSearchContextSize::High),
        _ => None,
    });

    let user_location = cfg
        .user_location
        .as_ref()
        .map(|loc| WebSearchApproximateLocation {
            r#type: WebSearchApproximateLocationType::Approximate,
            city: loc.city.clone(),
            country: loc.country.clone(),
            region: loc.region.clone(),
            timezone: loc.timezone.clone(),
        });

    Tool::WebSearch(WebSearchTool {
        filters: None,
        user_location,
        search_context_size,
    })
}

// ── Response parsing helpers ──────────────────────────────────────────────────

/// Parse a completed [`Response`] into our [`LlmResponse`].
///
/// The Responses API returns a flat `output: Vec<OutputItem>` containing
/// messages, function calls, web search calls, reasoning items, etc.
/// We extract function calls and text content from the output.
fn parse_response(response: Response) -> anyhow::Result<LlmResponse> {
    let meta = extract_response_meta(&response);

    let mut tool_calls: Vec<ToolCallItem> = Vec::new();
    let mut text_parts: Vec<String> = Vec::new();

    for item in &response.output {
        match item {
            OutputItem::FunctionCall(fc) => {
                let params = serde_json::from_str::<Value>(&fc.arguments)
                    .unwrap_or(Value::Object(serde_json::Map::new()));
                tool_calls.push(ToolCallItem {
                    name: fc.name.clone(),
                    params,
                    id: Some(fc.call_id.clone()),
                });
            }
            OutputItem::Message(msg) => {
                extract_message_text(msg, &mut text_parts);
            }
            OutputItem::WebSearchCall(_) => {
                // Web search calls are handled internally by the API;
                // the result appears as message text with citations.
                debug!("OpenAI: web search call in output (handled by API)");
            }
            OutputItem::Reasoning(_) => {
                // Reasoning items from o-series models — we could surface
                // these but for now just log.
                debug!("OpenAI: reasoning item in output");
            }
            other => {
                debug!(?other, "OpenAI: unhandled output item type");
            }
        }
    }

    // Priority: tool calls > text.
    if !tool_calls.is_empty() {
        debug!(
            count = tool_calls.len(),
            "OpenAI Responses: tool calls received"
        );
        return Ok(LlmResponse::ToolCalls(tool_calls, meta));
    }

    let content = text_parts.join("");
    debug!("OpenAI Responses: final answer received");
    Ok(LlmResponse::FinalAnswer(content, meta))
}

/// Extract text from an [`OutputMessage`].
fn extract_message_text(msg: &OutputMessage, text_parts: &mut Vec<String>) {
    for content in &msg.content {
        match content {
            OutputMessageContent::OutputText(ot) => {
                text_parts.push(ot.text.clone());
            }
            OutputMessageContent::Refusal(r) => {
                text_parts.push(format!("[Refusal: {}]", r.refusal));
            }
        }
    }
}

/// Extract [`LlmResponseMeta`] from a Responses API response.
fn extract_response_meta(response: &Response) -> LlmResponseMeta {
    let status = format!("{:?}", response.status).to_lowercase();

    LlmResponseMeta {
        model: Some(response.model.clone()),
        response_id: Some(response.id.clone()),
        input_tokens: response.usage.as_ref().map(|u| u.input_tokens as u64),
        output_tokens: response.usage.as_ref().map(|u| u.output_tokens as u64),
        finish_reason: Some(status),
    }
}

// ── HTTP client ───────────────────────────────────────────────────────────────

/// Build a `reqwest::Client` with the configured timeout.
///
/// `async-openai` does not support `reqwest-middleware`, so we cannot inject the
/// tracing middleware.  We do however set the request timeout so that it matches
/// the provider configuration rather than relying on async-openai's defaults.
fn build_reqwest_client(timeout_secs: u64) -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))
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
    fn build_input_items_user_assistant() {
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
        let items = build_input_items(&history);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn build_input_items_system_becomes_developer() {
        let history = vec![ChatHistoryMessage::Text {
            role: ChatRole::System,
            content: "you are helpful".to_string(),
        }];
        let items = build_input_items(&history);
        assert_eq!(items.len(), 1);
        match &items[0] {
            InputItem::EasyMessage(msg) => {
                assert_eq!(msg.role, Role::Developer);
            }
            _ => panic!("Expected EasyMessage"),
        }
    }

    #[test]
    fn build_input_items_tool_calls_and_results() {
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
        let items = build_input_items(&history);
        // FunctionCall + FunctionCallOutput
        assert_eq!(items.len(), 2);

        // Verify the call_id matches
        match (&items[0], &items[1]) {
            (
                InputItem::Item(Item::FunctionCall(fc)),
                InputItem::Item(Item::FunctionCallOutput(fco)),
            ) => {
                assert_eq!(fc.call_id, "call_123");
                assert_eq!(fco.call_id, "call_123");
            }
            _ => panic!("Expected FunctionCall and FunctionCallOutput"),
        }
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
        let tool = tool_spec_to_responses(&spec);
        match tool {
            Tool::Function(ft) => {
                assert_eq!(ft.name, "file-read");
                assert_eq!(ft.description, Some("Read a file".to_string()));
                assert!(ft.parameters.is_some());
            }
            _ => panic!("Expected Tool::Function"),
        }
    }

    // ── Web search tool tests ─────────────────────────────────────────────

    #[test]
    fn build_web_search_tool_defaults() {
        let cfg = OpenAIWebSearchConfig {
            search_context_size: None,
            user_location: None,
        };
        let tool = build_web_search_tool(&cfg);
        match tool {
            Tool::WebSearch(ws) => {
                assert!(ws.search_context_size.is_none());
                assert!(ws.user_location.is_none());
            }
            _ => panic!("Expected Tool::WebSearch"),
        }
    }

    #[test]
    fn build_web_search_tool_with_context_size() {
        for (input, expected) in [
            ("low", WebSearchToolSearchContextSize::Low),
            ("medium", WebSearchToolSearchContextSize::Medium),
            ("high", WebSearchToolSearchContextSize::High),
        ] {
            let cfg = OpenAIWebSearchConfig {
                search_context_size: Some(input.to_string()),
                user_location: None,
            };
            let tool = build_web_search_tool(&cfg);
            match tool {
                Tool::WebSearch(ws) => {
                    assert_eq!(ws.search_context_size, Some(expected));
                }
                _ => panic!("Expected Tool::WebSearch"),
            }
        }
    }

    #[test]
    fn build_web_search_tool_invalid_context_size_ignored() {
        let cfg = OpenAIWebSearchConfig {
            search_context_size: Some("ultra".to_string()),
            user_location: None,
        };
        let tool = build_web_search_tool(&cfg);
        match tool {
            Tool::WebSearch(ws) => {
                assert!(ws.search_context_size.is_none());
            }
            _ => panic!("Expected Tool::WebSearch"),
        }
    }

    #[test]
    fn build_web_search_tool_with_user_location() {
        let cfg = OpenAIWebSearchConfig {
            search_context_size: None,
            user_location: Some(OpenAIUserLocation {
                country: Some("GB".to_string()),
                city: Some("London".to_string()),
                region: Some("London".to_string()),
                timezone: Some("Europe/London".to_string()),
            }),
        };
        let tool = build_web_search_tool(&cfg);
        match tool {
            Tool::WebSearch(ws) => {
                let loc = ws.user_location.expect("user_location should be set");
                assert_eq!(loc.country, Some("GB".to_string()));
                assert_eq!(loc.city, Some("London".to_string()));
            }
            _ => panic!("Expected Tool::WebSearch"),
        }
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
        assert_eq!(
            caps.hosted_tools,
            vec![HostedTool::WebSearch],
            "Responses API web search works on all models"
        );
    }

    #[test]
    fn capabilities_with_web_search_any_model() {
        // Responses API web search works with gpt-4o, gpt-5, o4-mini, etc.
        for model in ["gpt-4o", "gpt-5", "o4-mini", "gpt-4.1"] {
            let cfg = OpenAIProviderConfig {
                model: model.to_string(),
                web_search: Some(OpenAIWebSearchConfig {
                    search_context_size: None,
                    user_location: None,
                }),
                ..Default::default()
            };
            let provider = OpenAIProvider::new(cfg, "test-key").expect("should build");
            let caps = provider.capabilities();
            assert_eq!(
                caps.hosted_tools,
                vec![HostedTool::WebSearch],
                "web search should work on model {model}"
            );
        }
    }

    #[test]
    fn build_input_items_multimodal() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("Look at this".to_string()),
                ContentBlock::Image {
                    media_type: "image/png".to_string(),
                    data: "base64data".to_string(),
                },
            ],
        }];
        let items = build_input_items(&history);
        assert_eq!(items.len(), 1);
        match &items[0] {
            InputItem::EasyMessage(msg) => {
                assert_eq!(msg.role, Role::User);
            }
            _ => panic!("Expected EasyMessage with content list"),
        }
    }

    #[test]
    fn find_preceding_call_id_matches_correctly() {
        let items = vec![
            InputItem::Item(Item::FunctionCall(FunctionToolCall {
                arguments: "{}".to_string(),
                call_id: "call_1".to_string(),
                name: "tool-a".to_string(),
                id: None,
                status: Some(OutputStatus::Completed),
            })),
            InputItem::Item(Item::FunctionCallOutput(FunctionCallOutputItemParam {
                call_id: "call_1".to_string(),
                output: FunctionCallOutput::Text("done".to_string()),
                id: None,
                status: None,
            })),
            InputItem::Item(Item::FunctionCall(FunctionToolCall {
                arguments: "{}".to_string(),
                call_id: "call_2".to_string(),
                name: "tool-b".to_string(),
                id: None,
                status: Some(OutputStatus::Completed),
            })),
        ];
        // call_1 is matched, call_2 is unmatched → should return call_2
        let id = find_preceding_call_id(&items);
        assert_eq!(id, "call_2");
    }
}
