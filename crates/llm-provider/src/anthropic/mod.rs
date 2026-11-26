//! `AnthropicProvider` — [`LlmProvider`] implementation backed by the Anthropic Messages API.

use async_trait::async_trait;
use futures::StreamExt as _;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::Secret;
use assistant_core::types::llm::{
    AnthropicUserLocation, AnthropicWebFetchOptions, AnthropicWebSearchOptions, LlmConfig,
};
use assistant_core::{
    Capabilities, ChatHistoryMessage, ChatOptions, ChatOptionsSupport, ChatRole, ContentBlock,
    HostedTool, LlmProvider, LlmResponse, LlmResponseMeta, StreamChunk, ToolCallItem,
    ToolCallResponse, ToolChoice, ToolSpec, ToolSupport,
};

// ── AnthropicConfig ───────────────────────────────────────────────────────────

/// Configuration for the Anthropic backend.
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// Model ID, e.g. `"claude-opus-4-6"`.
    pub model: String,
    /// Anthropic API key.  Falls back to `ANTHROPIC_API_KEY` env var at construction time.
    pub api_key: Secret,
    /// Base URL (default: `"https://api.anthropic.com"`).
    pub base_url: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// `max_tokens` sent in every request (Anthropic requires this field).
    pub max_tokens: u32,
    /// Optional hosted web-search configuration.
    pub web_search: Option<WebSearchConfig>,
    /// Optional hosted web-fetch configuration.
    pub web_fetch: Option<WebFetchConfig>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            model: "claude-opus-4-6".to_string(),
            api_key: Secret::new(""),
            base_url: "https://api.anthropic.com".to_string(),
            timeout_secs: 120,
            max_tokens: 8192,
            web_search: None,
            web_fetch: None,
        }
    }
}

impl AnthropicConfig {
    /// Build from an [`LlmConfig`], resolving the API key from config or env.
    ///
    /// Returns an error if no key is available.
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        let api_key = cfg
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok().map(Secret::from))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Anthropic API key not found. Set api_key in [llm] config or \
                     ANTHROPIC_API_KEY environment variable."
                )
            })?;

        Ok(Self {
            model: cfg.model.clone(),
            api_key,
            // base_url: use the configured value if it differs from the default Ollama one,
            // otherwise use the Anthropic default.
            base_url: if cfg.base_url == "http://localhost:11434" {
                "https://api.anthropic.com".to_string()
            } else {
                cfg.base_url.clone()
            },
            timeout_secs: cfg.timeout_secs,
            max_tokens: 8192,
            web_search: if cfg.anthropic.web_search.enabled {
                Some(WebSearchConfig::from(&cfg.anthropic.web_search))
            } else {
                None
            },
            web_fetch: if cfg.anthropic.web_fetch.enabled {
                Some(WebFetchConfig::from(&cfg.anthropic.web_fetch))
            } else {
                None
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct WebSearchConfig {
    pub max_uses: Option<u32>,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
    pub user_location: Option<WebSearchLocation>,
}

#[derive(Debug, Clone)]
pub struct WebSearchLocation {
    pub r#type: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    pub timezone: Option<String>,
}

impl From<&AnthropicWebSearchOptions> for WebSearchConfig {
    fn from(opts: &AnthropicWebSearchOptions) -> Self {
        Self {
            max_uses: opts.max_uses,
            allowed_domains: opts.allowed_domains.clone(),
            blocked_domains: opts.blocked_domains.clone(),
            user_location: opts.user_location.as_ref().map(WebSearchLocation::from),
        }
    }
}

impl From<&AnthropicUserLocation> for WebSearchLocation {
    fn from(loc: &AnthropicUserLocation) -> Self {
        Self {
            r#type: loc.r#type.clone(),
            city: loc.city.clone(),
            region: loc.region.clone(),
            country: loc.country.clone(),
            timezone: loc.timezone.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebFetchConfig {
    pub max_uses: Option<u32>,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
    pub citations_enabled: bool,
    pub max_content_tokens: Option<u32>,
}

impl From<&AnthropicWebFetchOptions> for WebFetchConfig {
    fn from(opts: &AnthropicWebFetchOptions) -> Self {
        Self {
            max_uses: opts.max_uses,
            allowed_domains: opts.allowed_domains.clone(),
            blocked_domains: opts.blocked_domains.clone(),
            citations_enabled: opts.citations.enabled,
            max_content_tokens: opts.max_content_tokens,
        }
    }
}

// ── AnthropicProvider ─────────────────────────────────────────────────────────

/// [`LlmProvider`] backed by the Anthropic Messages API.
pub struct AnthropicProvider {
    config: AnthropicConfig,
    http: reqwest_middleware::ClientWithMiddleware,
}

impl AnthropicProvider {
    /// Create from explicit config.
    pub fn new(config: AnthropicConfig) -> anyhow::Result<Self> {
        let http = crate::http::build_http_client(
            config.timeout_secs,
            &crate::retry::RetryConfig::default(),
        )?;
        Ok(Self { config, http })
    }

    /// Convenience constructor directly from [`LlmConfig`].
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        Self::new(AnthropicConfig::from_llm_config(cfg)?)
    }

    fn server_tool_specs(&self) -> Vec<Value> {
        let mut specs = Vec::new();
        if let Some(cfg) = &self.config.web_search {
            let mut entry = json!({
                "type": "web_search_20250305",
                "name": "web_search",
            });
            if let Some(max) = cfg.max_uses {
                entry["max_uses"] = json!(max);
            }
            if !cfg.allowed_domains.is_empty() {
                entry["allowed_domains"] = json!(cfg.allowed_domains);
            }
            if !cfg.blocked_domains.is_empty() {
                entry["blocked_domains"] = json!(cfg.blocked_domains);
            }
            if let Some(loc) = &cfg.user_location {
                let mut loc_json = serde_json::Map::new();
                if let Some(t) = &loc.r#type {
                    loc_json.insert("type".to_string(), json!(t));
                }
                if let Some(city) = &loc.city {
                    loc_json.insert("city".to_string(), json!(city));
                }
                if let Some(region) = &loc.region {
                    loc_json.insert("region".to_string(), json!(region));
                }
                if let Some(country) = &loc.country {
                    loc_json.insert("country".to_string(), json!(country));
                }
                if let Some(tz) = &loc.timezone {
                    loc_json.insert("timezone".to_string(), json!(tz));
                }
                if !loc_json.is_empty() {
                    entry["user_location"] = Value::Object(loc_json);
                }
            }
            specs.push(entry);
        }
        if let Some(cfg) = &self.config.web_fetch {
            let mut entry = json!({
                "type": "web_fetch_20250910",
                "name": "web_fetch",
            });
            if let Some(max) = cfg.max_uses {
                entry["max_uses"] = json!(max);
            }
            if !cfg.allowed_domains.is_empty() {
                entry["allowed_domains"] = json!(cfg.allowed_domains);
            }
            if !cfg.blocked_domains.is_empty() {
                entry["blocked_domains"] = json!(cfg.blocked_domains);
            }
            if cfg.citations_enabled {
                entry["citations"] = json!({ "enabled": true });
            }
            if let Some(limit) = cfg.max_content_tokens {
                entry["max_content_tokens"] = json!(limit);
            }
            specs.push(entry);
        }
        specs
    }
}

// ── Message conversion ────────────────────────────────────────────────────────

/// Convert a [`ToolSpec`] to the Anthropic `tools` array entry.
///
/// Anthropic uses `input_schema` (not `parameters` like OpenAI/Ollama).
fn tool_spec_to_anthropic_json(tool: &ToolSpec) -> Value {
    let input_schema = tool.normalized_params_schema();

    json!({
        "name": tool.name,
        "description": tool.description,
        "input_schema": input_schema,
    })
}

/// Build Anthropic-format messages from history.
///
/// Returns `(system_prompt_or_empty, messages_vec)`.
/// The system prompt is passed separately at the top level; it is not a message.
/// The `tool_choice` object for `options`, or `None` to send the default.
fn anthropic_tool_choice(options: &ChatOptions) -> Option<Value> {
    let mut choice = match &options.tool_choice {
        ToolChoice::Auto if options.parallel_tool_calls.is_none() => return None,
        ToolChoice::Auto => json!({"type": "auto"}),
        ToolChoice::None => json!({"type": "none"}),
        ToolChoice::Required => json!({"type": "any"}),
        ToolChoice::Tool(name) => json!({"type": "tool", "name": name}),
    };
    if let Some(parallel) = options.parallel_tool_calls
        && options.tool_choice != ToolChoice::None
    {
        choice["disable_parallel_tool_use"] = json!(!parallel);
    }
    Some(choice)
}

fn build_anthropic_messages(history: &[ChatHistoryMessage]) -> (Vec<Value>, Vec<(String, String)>) {
    // Maps tool_use id → name, built as we process AssistantToolCalls.
    // Used by ToolResult to look up the tool_use_id.
    let mut pending_ids: Vec<(String, String)> = Vec::new(); // (name, id)
    let mut messages: Vec<Value> = Vec::new();

    for msg in history {
        match msg {
            ChatHistoryMessage::Text { role, content } => {
                let role_str = match role {
                    ChatRole::User | ChatRole::System | ChatRole::Tool => "user",
                    ChatRole::Assistant => "assistant",
                };
                messages.push(json!({ "role": role_str, "content": content }));
            }

            ChatHistoryMessage::MultimodalUser { content } => {
                let blocks: Vec<Value> = content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text(text) => Some(json!({"type": "text", "text": text})),
                        ContentBlock::Image { media_type, data } => Some(json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": media_type,
                                "data": data,
                            }
                        })),
                        ContentBlock::Document { media_type, data } => Some(json!({
                            "type": "document",
                            "source": {
                                "type": "base64",
                                "media_type": media_type,
                                "data": data,
                            }
                        })),
                        ContentBlock::Audio { .. } => None,
                    })
                    .collect();
                messages.push(json!({"role": "user", "content": blocks}));
            }

            ChatHistoryMessage::AssistantToolCalls { items: calls, .. } => {
                // Start a new batch; previous pending_ids from older rounds are consumed
                // by ToolResult messages, so only the current batch goes here.
                pending_ids.clear();
                let content_blocks: Vec<Value> = calls
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let id = c.id.clone().unwrap_or_else(|| format!("toolu_{i}"));
                        pending_ids.push((c.name.clone(), id.clone()));
                        json!({
                            "type": "tool_use",
                            "id": id,
                            "name": c.name,
                            "input": c.params,
                        })
                    })
                    .collect();
                messages.push(json!({
                    "role": "assistant",
                    "content": content_blocks,
                }));
            }

            ChatHistoryMessage::ToolResult { name, content } => {
                // Consume the first matching pending entry so that when the same tool
                // is called twice in one batch, each result gets a distinct id.
                let pos = pending_ids.iter().position(|(n, _)| n == name);
                let tool_use_id = if let Some(idx) = pos {
                    pending_ids.remove(idx).1
                } else {
                    format!("toolu_unknown_{name}")
                };

                let result_block = json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": content,
                });
                messages.push(json!({
                    "role": "user",
                    "content": [result_block],
                }));
            }
        }
    }

    (messages, pending_ids)
}

// ── LlmProvider impl ──────────────────────────────────────────────────────────

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: ToolSupport::Native,
            streaming: true,
            vision: true,
            hosted_tools: {
                let mut hosted = Vec::new();
                if self.config.web_search.is_some() {
                    hosted.push(HostedTool::WebSearch);
                }
                if self.config.web_fetch.is_some() {
                    hosted.push(HostedTool::WebFetch);
                }
                hosted
            },
            context_window_tokens: Some(200_000),
        }
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        self.chat_non_streaming(system_prompt, history, tools, &ChatOptions::default())
            .await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        self.chat_sse(
            system_prompt,
            history,
            tools,
            &ChatOptions::default(),
            chunk_sink,
        )
        .await
    }

    fn chat_options_support(&self) -> ChatOptionsSupport {
        ChatOptionsSupport {
            tool_choice: true,
            parallel_tool_calls: true,
            json_schema: false,
            audio_input: false,
        }
    }

    async fn chat_with(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        options: &ChatOptions,
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        match chunk_sink {
            Some(sink) => {
                self.chat_sse(system_prompt, history, tools, options, Some(sink))
                    .await
            }
            None => {
                self.chat_non_streaming(system_prompt, history, tools, options)
                    .await
            }
        }
    }

    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!(
            "Anthropic does not support text embeddings"
        ))
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn server_address(&self) -> &str {
        &self.config.base_url
    }
}

impl AnthropicProvider {
    fn request_body(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        options: &ChatOptions,
        stream: bool,
    ) -> Value {
        let (messages, _) = build_anthropic_messages(history);
        let mut request_tools: Vec<Value> = tools.iter().map(tool_spec_to_anthropic_json).collect();
        request_tools.extend(self.server_tool_specs());

        let mut body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": messages,
        });
        if stream {
            body["stream"] = json!(true);
        }
        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }
        if !request_tools.is_empty() {
            body["tools"] = json!(request_tools);
            if let Some(choice) = anthropic_tool_choice(options) {
                body["tool_choice"] = choice;
            }
        }
        body
    }

    // ── Non-streaming ─────────────────────────────────────────────────────────

    async fn chat_non_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        options: &ChatOptions,
    ) -> anyhow::Result<LlmResponse> {
        debug!(model = %self.config.model, tools = tools.len(), "Sending request to Anthropic");

        let body = self.request_body(system_prompt, history, tools, options, false);

        let url = format!("{}/v1/messages", self.config.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", self.config.api_key.expose())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Anthropic request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic returned {status}: {text}");
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Anthropic response: {e}"))?;

        debug!("Anthropic response received");

        let meta = extract_anthropic_meta(&json);
        parse_response_json(&json, meta)
    }

    // ── SSE streaming ─────────────────────────────────────────────────────────

    async fn chat_sse(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        options: &ChatOptions,
        token_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        debug!(
            model = %self.config.model,
            "Sending streaming request to Anthropic"
        );

        let body = self.request_body(system_prompt, history, tools, options, true);

        let url = format!("{}/v1/messages", self.config.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", self.config.api_key.expose())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Anthropic streaming request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic returned {status}: {text}");
        }

        // State accumulated while parsing SSE events.
        let mut text_buf = String::new();
        let mut thinking_buf = String::new();
        let mut tool_blocks: Vec<ToolBlock> = Vec::new();
        // Current block index and type.
        let mut current_block_idx: Option<usize> = None;
        let mut current_block_type = String::new();
        // Response metadata accumulated from message_start + message_delta events.
        let mut sse_meta = LlmResponseMeta::default();

        let mut byte_stream = resp.bytes_stream();
        let mut line_buf = String::new();
        let mut event_type = String::new();

        'outer: while let Some(chunk) = byte_stream.next().await {
            let chunk = chunk.map_err(|e| anyhow::anyhow!("SSE stream read error: {e}"))?;
            let text = String::from_utf8_lossy(&chunk);

            for ch in text.chars() {
                if ch == '\n' {
                    let line = std::mem::take(&mut line_buf);
                    let line = line.trim_end_matches('\r');

                    if line.is_empty() {
                        // Empty line = end of event; reset event type.
                        event_type.clear();
                        continue;
                    }

                    if let Some(rest) = line.strip_prefix("event: ") {
                        event_type = rest.to_string();
                        continue;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            break 'outer;
                        }
                        if let Ok(json) = serde_json::from_str::<Value>(data) {
                            process_sse_event(
                                &event_type,
                                &json,
                                &mut text_buf,
                                &mut thinking_buf,
                                &mut tool_blocks,
                                &mut current_block_idx,
                                &mut current_block_type,
                                &token_sink,
                                &mut sse_meta,
                            )
                            .await;
                        }
                        continue;
                    }
                } else {
                    line_buf.push(ch);
                }
            }
        }

        debug!("Anthropic SSE stream complete");

        // Priority: tool_use > thinking > text.
        if !tool_blocks.is_empty() {
            let items: Vec<ToolCallItem> = tool_blocks
                .into_iter()
                .filter_map(|block| {
                    if block.name.is_empty() {
                        return None;
                    }
                    let params = serde_json::from_str::<Value>(&block.partial_json)
                        .unwrap_or(Value::Object(serde_json::Map::new()));
                    Some(ToolCallItem {
                        name: block.name,
                        params,
                        id: Some(block.id),
                    })
                })
                .collect();
            if !items.is_empty() {
                debug!(count = items.len(), "Anthropic SSE: tool calls received");
                return Ok(LlmResponse::ToolCalls(ToolCallResponse {
                    items,
                    meta: sse_meta,
                    // Only carry thinking in batch if it wasn't already
                    // streamed token-by-token via the chunk sink.
                    thinking: if thinking_buf.is_empty() || token_sink.is_some() {
                        None
                    } else {
                        Some(thinking_buf.clone())
                    },
                    provider_state: None,
                }));
            }
        }

        if !thinking_buf.is_empty() {
            return Ok(LlmResponse::Thinking(thinking_buf, sse_meta));
        }

        Ok(LlmResponse::FinalAnswer(text_buf, sse_meta))
    }
}

// ── Tool-block accumulator ────────────────────────────────────────────────────

/// Accumulator for a single streaming tool-call block.
struct ToolBlock {
    id: String,
    name: String,
    partial_json: String,
    /// Number of decoded text characters already forwarded to the token sink.
    text_chars_sent: usize,
}

impl ToolBlock {
    fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            partial_json: String::new(),
            text_chars_sent: 0,
        }
    }

    fn empty() -> Self {
        Self::new(String::new(), String::new())
    }
}

/// Extract the current decoded value of the `"text"` field from a
/// partially-received JSON object (e.g. `{"text": "Hello wor`).
///
/// Handles JSON escape sequences.  Returns `None` if the key or opening
/// quote has not yet been received.
fn extract_partial_text(partial_json: &str) -> Option<String> {
    let value_start = partial_json
        .find("\"text\":\"")
        .map(|p| p + 8)
        .or_else(|| partial_json.find("\"text\": \"").map(|p| p + 9))?;

    let text_part = &partial_json[value_start..];
    let mut result = String::new();
    let mut chars = text_part.chars();
    let mut escape = false;

    loop {
        match chars.next() {
            None => break, // Partial input — text still arriving
            Some('\\') if !escape => {
                escape = true;
            }
            Some('"') if !escape => break, // End of string
            Some(ch) if escape => {
                match ch {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '/' => result.push('/'),
                    'b' => result.push('\x08'),
                    'f' => result.push('\x0C'),
                    _ => {
                        result.push('\\');
                        result.push(ch);
                    }
                }
                escape = false;
            }
            Some(ch) => {
                result.push(ch);
            }
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

// ── SSE event processor ───────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn process_sse_event(
    event_type: &str,
    json: &Value,
    text_buf: &mut String,
    thinking_buf: &mut String,
    tool_blocks: &mut Vec<ToolBlock>,
    current_block_idx: &mut Option<usize>,
    current_block_type: &mut String,
    token_sink: &Option<mpsc::Sender<StreamChunk>>,
    meta: &mut LlmResponseMeta,
) {
    match event_type {
        // `message_start` carries model, id, and input_tokens.
        "message_start" => {
            if let Some(msg) = json.get("message") {
                meta.model = msg.get("model").and_then(|v| v.as_str()).map(String::from);
                meta.response_id = msg.get("id").and_then(|v| v.as_str()).map(String::from);
                if let Some(usage) = msg.get("usage") {
                    apply_anthropic_usage(meta, usage);
                }
            }
        }

        "content_block_start" => {
            let idx = json.pointer("/index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let block_type = json
                .pointer("/content_block/type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            *current_block_idx = Some(idx);
            *current_block_type = block_type.clone();

            if block_type == "tool_use" {
                let id = json
                    .pointer("/content_block/id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = json
                    .pointer("/content_block/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                // Extend tool_blocks to accommodate this index.
                while tool_blocks.len() <= idx {
                    tool_blocks.push(ToolBlock::empty());
                }
                tool_blocks[idx] = ToolBlock::new(id, name);
            }
        }

        "content_block_delta" => {
            let delta_type = json
                .pointer("/delta/type")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match delta_type {
                "text_delta" => {
                    if let Some(text) = json.pointer("/delta/text").and_then(|v| v.as_str()) {
                        text_buf.push_str(text);
                        if let Some(sink) = token_sink {
                            let _ = sink.send(StreamChunk::Text(text.to_string())).await;
                        }
                    }
                }
                "thinking_delta" => {
                    if let Some(thinking) = json.pointer("/delta/thinking").and_then(|v| v.as_str())
                    {
                        thinking_buf.push_str(thinking);
                        if let Some(sink) = token_sink {
                            let _ = sink.send(StreamChunk::Thinking(thinking.to_string())).await;
                        }
                    }
                }
                "input_json_delta" => {
                    if let Some(idx) = *current_block_idx
                        && let Some(partial) =
                            json.pointer("/delta/partial_json").and_then(|v| v.as_str())
                        && idx < tool_blocks.len()
                    {
                        tool_blocks[idx].partial_json.push_str(partial);
                        // Stream the decoded text value of this tool block
                        // so the caller can update a live preview (e.g. the
                        // Slack placeholder message).
                        if let Some(sink) = token_sink
                            && let Some(text) = extract_partial_text(&tool_blocks[idx].partial_json)
                        {
                            let already = tool_blocks[idx].text_chars_sent;
                            if text.len() > already {
                                let new_text = text[already..].to_string();
                                tool_blocks[idx].text_chars_sent = text.len();
                                let _ = sink.send(StreamChunk::Text(new_text)).await;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // `message_delta` carries stop_reason and output_tokens.
        "message_delta" => {
            meta.finish_reason = json
                .pointer("/delta/stop_reason")
                .and_then(|v| v.as_str())
                .map(String::from);
            if let Some(out) = json
                .pointer("/usage/output_tokens")
                .and_then(|v| v.as_u64())
            {
                meta.output_tokens = Some(out);
            }
            if let Some(usage) = json.get("usage") {
                merge_anthropic_delta_usage(meta, usage);
            }
        }

        _ => {}
    }
}

// ── Response parser (non-streaming) ──────────────────────────────────────────

fn parse_response_json(json: &Value, meta: LlmResponseMeta) -> anyhow::Result<LlmResponse> {
    let content = json
        .get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Missing 'content' array in Anthropic response"))?;

    let mut tool_calls: Vec<ToolCallItem> = Vec::new();
    let mut text_parts: Vec<String> = Vec::new();
    let mut thinking_parts: Vec<String> = Vec::new();

    for block in content {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match block_type {
            "tool_use" => {
                let name = block
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                let id = block
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let params = block
                    .get("input")
                    .cloned()
                    .unwrap_or(Value::Object(serde_json::Map::new()));
                tool_calls.push(ToolCallItem { name, params, id });
            }
            "text" => {
                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                    text_parts.push(text.to_string());
                }
            }
            "thinking" => {
                if let Some(thinking) = block.get("thinking").and_then(|v| v.as_str()) {
                    thinking_parts.push(thinking.to_string());
                }
            }
            _ => {}
        }
    }

    // Priority: tool_use > thinking > text.
    if !tool_calls.is_empty() {
        debug!(
            count = tool_calls.len(),
            "Anthropic non-streaming: tool calls received"
        );
        return Ok(LlmResponse::ToolCalls(ToolCallResponse {
            items: tool_calls,
            meta,
            thinking: if thinking_parts.is_empty() {
                None
            } else {
                Some(thinking_parts.join(""))
            },
            provider_state: None,
        }));
    }
    if !thinking_parts.is_empty() {
        return Ok(LlmResponse::Thinking(thinking_parts.join(""), meta));
    }
    Ok(LlmResponse::FinalAnswer(text_parts.join(""), meta))
}

/// Extract [`LlmResponseMeta`] from an Anthropic non-streaming JSON response.
///
/// Top-level fields: `model`, `id`, `stop_reason`, `usage.input_tokens`,
/// `usage.output_tokens`.
fn extract_anthropic_meta(json: &Value) -> LlmResponseMeta {
    let mut meta = LlmResponseMeta {
        model: json.get("model").and_then(|v| v.as_str()).map(String::from),
        response_id: json.get("id").and_then(|v| v.as_str()).map(String::from),
        finish_reason: json
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(String::from),
        output_tokens: json
            .pointer("/usage/output_tokens")
            .and_then(|v| v.as_u64()),
        ..Default::default()
    };
    if let Some(usage) = json.get("usage") {
        apply_anthropic_usage(&mut meta, usage);
    }
    meta
}

/// Fold a `message_delta` usage object into `meta`. Its counts are cumulative
/// but may omit fields, so a count it leaves out keeps the value seen in
/// `message_start`.
fn merge_anthropic_delta_usage(meta: &mut LlmResponseMeta, usage: &Value) {
    let count = |key: &str| usage.get(key).and_then(Value::as_u64);
    let (delta_uncached, delta_cached, delta_write) = (
        count("input_tokens"),
        count("cache_read_input_tokens"),
        count("cache_creation_input_tokens"),
    );
    if delta_uncached.is_none() && delta_cached.is_none() && delta_write.is_none() {
        return;
    }
    let prior_cached = meta.cached_input_tokens.unwrap_or(0);
    let prior_write = meta.cache_write_input_tokens.unwrap_or(0);
    let uncached = delta_uncached.unwrap_or_else(|| {
        meta.input_tokens
            .unwrap_or(0)
            .saturating_sub(prior_cached + prior_write)
    });
    if delta_cached.is_some() {
        meta.cached_input_tokens = delta_cached;
    }
    if delta_write.is_some() {
        meta.cache_write_input_tokens = delta_write;
    }
    meta.input_tokens = Some(
        uncached
            + meta.cached_input_tokens.unwrap_or(0)
            + meta.cache_write_input_tokens.unwrap_or(0),
    );
}

/// Copy input-side token counts from an Anthropic `usage` object. Anthropic's
/// `input_tokens` excludes cache reads and writes; the GenAI conventions count
/// them as input, so `meta.input_tokens` is the sum.
fn apply_anthropic_usage(meta: &mut LlmResponseMeta, usage: &Value) {
    let count = |key: &str| usage.get(key).and_then(Value::as_u64);
    let uncached = count("input_tokens");
    meta.cached_input_tokens = count("cache_read_input_tokens");
    meta.cache_write_input_tokens = count("cache_creation_input_tokens");
    meta.input_tokens = match (
        uncached,
        meta.cached_input_tokens,
        meta.cache_write_input_tokens,
    ) {
        (None, None, None) => None,
        (a, b, c) => Some(a.unwrap_or(0) + b.unwrap_or(0) + c.unwrap_or(0)),
    };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use assistant_core::{ChatHistoryMessage, ChatRole, ContentBlock};

    use super::build_anthropic_messages;

    #[test]
    fn multimodal_user_produces_content_blocks_array() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("What is in this image?".to_string()),
                ContentBlock::Image {
                    media_type: "image/png".to_string(),
                    data: "iVBORw0KGgo=".to_string(),
                },
            ],
        }];
        let (messages, _) = build_anthropic_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");

        let content = messages[0]["content"].as_array().expect("content array");
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "What is in this image?");
        assert_eq!(content[1]["type"], "image");
        assert_eq!(content[1]["source"]["type"], "base64");
        assert_eq!(content[1]["source"]["media_type"], "image/png");
        assert_eq!(content[1]["source"]["data"], "iVBORw0KGgo=");
    }

    #[test]
    fn multimodal_user_text_only_still_emits_blocks() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![ContentBlock::Text("just text".to_string())],
        }];
        let (messages, _) = build_anthropic_messages(&history);
        let content = messages[0]["content"].as_array().expect("content array");
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "just text");
    }

    #[test]
    fn audio_blocks_are_dropped() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("[Voice message]: hi".into()),
                ContentBlock::Audio {
                    media_type: "audio/ogg".into(),
                    data: "T2dnUw==".into(),
                },
            ],
        }];
        let (messages, _) = build_anthropic_messages(&history);
        let content = messages[0]["content"].as_array().expect("content array");
        assert_eq!(content.len(), 1, "only the text: {content:?}");
        assert_eq!(content[0]["type"], "text");
    }

    #[test]
    fn multimodal_user_document_produces_document_block() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("Summarize this PDF".to_string()),
                ContentBlock::Document {
                    media_type: "application/pdf".to_string(),
                    data: "JVBERi0xLjQ=".to_string(),
                },
            ],
        }];
        let (messages, _) = build_anthropic_messages(&history);
        let content = messages[0]["content"].as_array().expect("content array");
        assert_eq!(content.len(), 2, "should have text and document blocks");
        assert_eq!(content[0]["type"], "text", "first block should be text");
        assert_eq!(
            content[1]["type"], "document",
            "second block should be document"
        );
        assert_eq!(
            content[1]["source"]["type"], "base64",
            "document source should be base64"
        );
        assert_eq!(
            content[1]["source"]["media_type"], "application/pdf",
            "document media type should be PDF"
        );
        assert_eq!(
            content[1]["source"]["data"], "JVBERi0xLjQ=",
            "document data should match encoded PDF"
        );
    }

    #[test]
    fn plain_text_message_uses_string_content() {
        let history = vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "hello".to_string(),
        }];
        let (messages, _) = build_anthropic_messages(&history);
        assert_eq!(messages[0]["content"], "hello");
    }

    // ── extract_partial_text ──────────────────────────────────────────────────

    use super::extract_partial_text;

    #[test]
    fn extract_partial_text_complete_json() {
        let json = r#"{"text":"Hello, world!"}"#;
        assert_eq!(extract_partial_text(json).as_deref(), Some("Hello, world!"));
    }

    #[test]
    fn extract_partial_text_space_after_colon() {
        let json = r#"{"text": "Hi there"}"#;
        assert_eq!(extract_partial_text(json).as_deref(), Some("Hi there"));
    }

    #[test]
    fn extract_partial_text_partial_json_no_closing_quote() {
        // The closing `"` hasn't arrived yet — should return what we have so far.
        let json = r#"{"text":"Hello wor"#;
        assert_eq!(extract_partial_text(json).as_deref(), Some("Hello wor"));
    }

    #[test]
    fn extract_partial_text_key_not_yet_arrived() {
        // Only the opening brace has been received — key missing.
        assert_eq!(extract_partial_text("{"), None);
        assert_eq!(extract_partial_text(r#"{"tex"#), None);
    }

    #[test]
    fn extract_partial_text_empty_string_value() {
        // Key present but value is the empty string → no text to stream yet.
        let json = r#"{"text":""}"#;
        assert_eq!(extract_partial_text(json), None);
    }

    #[test]
    fn extract_partial_text_escape_sequences() {
        // Embedded newline and quote, properly JSON-escaped.
        let json = r#"{"text":"line1\nline2"}"#;
        assert_eq!(extract_partial_text(json).as_deref(), Some("line1\nline2"));

        let json_quote = r#"{"text":"say \"hi\""}"#;
        assert_eq!(
            extract_partial_text(json_quote).as_deref(),
            Some(r#"say "hi""#)
        );
    }

    #[test]
    fn extract_partial_text_partial_escape_sequence() {
        // The backslash has arrived but not the escape char yet.
        let json = r#"{"text":"Hello \"#;
        // The `\` starts an escape we can't finish — the partially-decoded
        // result should be "Hello " (everything before the `\`).
        let result = extract_partial_text(json);
        assert_eq!(result.as_deref(), Some("Hello "));
    }

    #[test]
    fn extract_partial_text_missing_text_key() {
        // JSON with a different key should return None.
        let json = r#"{"content":"Hello"}"#;
        assert_eq!(extract_partial_text(json), None);
    }

    #[test]
    fn extract_partial_text_incremental_deltas() {
        // Simulate receiving JSON in fragments; each fragment is appended and
        // we call extract_partial_text on the accumulated buffer.
        let fragments = [
            r#"{"text":"H"#,
            r#"{"text":"He"#,
            r#"{"text":"Hel"#,
            r#"{"text":"Hell"#,
            r#"{"text":"Hello"}"#,
        ];
        let expected = ["H", "He", "Hel", "Hell", "Hello"];
        for (frag, exp) in fragments.iter().zip(expected.iter()) {
            assert_eq!(
                extract_partial_text(frag).as_deref(),
                Some(*exp),
                "fragment: {frag}"
            );
        }
    }

    #[test]
    fn extract_partial_text_only_sends_new_chars_when_tracked() {
        // Simulate the text_chars_sent tracking: each call returns the full
        // text so far, and the caller slices from `already_sent`.
        let full = r#"{"text":"Hello world"}"#;
        let text = extract_partial_text(full).unwrap();
        assert_eq!(&text[0..], "Hello world");
        assert_eq!(&text[5..], " world");
    }

    #[test]
    fn extract_partial_text_tab_and_backslash_escapes() {
        let json = r#"{"text":"a\tb\\c"}"#;
        assert_eq!(extract_partial_text(json).as_deref(), Some("a\tb\\c"));
    }

    #[test]
    fn multimodal_user_multiple_images() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("compare these".to_string()),
                ContentBlock::Image {
                    media_type: "image/png".to_string(),
                    data: "img1".to_string(),
                },
                ContentBlock::Image {
                    media_type: "image/jpeg".to_string(),
                    data: "img2".to_string(),
                },
            ],
        }];
        let (messages, _) = build_anthropic_messages(&history);
        let content = messages[0]["content"].as_array().expect("content array");
        assert_eq!(content.len(), 3, "text + 2 images");
        assert_eq!(content[1]["source"]["media_type"], "image/png");
        assert_eq!(content[2]["source"]["media_type"], "image/jpeg");
    }

    #[test]
    fn multimodal_user_interleaved_with_text_messages() {
        let history = vec![
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "first".to_string(),
            },
            ChatHistoryMessage::MultimodalUser {
                content: vec![
                    ContentBlock::Text("look at this".to_string()),
                    ContentBlock::Image {
                        media_type: "image/png".to_string(),
                        data: "abc".to_string(),
                    },
                ],
            },
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "I see an image".to_string(),
            },
        ];
        let (messages, _) = build_anthropic_messages(&history);
        assert_eq!(messages.len(), 3);
        // First: plain text
        assert_eq!(messages[0]["content"], "first");
        // Second: multimodal blocks
        assert!(messages[1]["content"].is_array());
        // Third: plain text assistant
        assert_eq!(messages[2]["content"], "I see an image");
        assert_eq!(messages[2]["role"], "assistant");
    }

    // ── tool_spec_to_anthropic_json ──────────────────────────────────────────

    use super::tool_spec_to_anthropic_json;
    use assistant_core::ToolSpec;

    #[test]
    fn tool_spec_serialises_input_schema_under_anthropic_key() {
        let spec = ToolSpec {
            name: "file-read".to_string(),
            description: "Read a file".to_string(),
            params_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
            }),
            is_mutating: false,
            requires_confirmation: false,
            requires_rich_client: false,
        };
        let v = tool_spec_to_anthropic_json(&spec);
        assert_eq!(v["name"], "file-read");
        assert_eq!(v["description"], "Read a file");
        assert!(v["input_schema"].is_object());
        // Anthropic uses `input_schema`, NOT `parameters`.
        assert!(v.get("parameters").is_none());
    }

    // ── AssistantToolCalls + ToolResult ──────────────────────────────────────

    use assistant_core::ToolCallItem;

    #[test]
    fn assistant_tool_calls_emit_tool_use_blocks_with_ids() {
        let history = vec![ChatHistoryMessage::AssistantToolCalls {
            items: vec![ToolCallItem {
                name: "shell".to_string(),
                params: serde_json::json!({"cmd": "ls"}),
                id: Some("toolu_123".to_string()),
            }],
            provider_state: None,
        }];
        let (messages, _) = build_anthropic_messages(&history);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "assistant");
        let blocks = messages[0]["content"].as_array().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "tool_use");
        assert_eq!(blocks[0]["id"], "toolu_123");
        assert_eq!(blocks[0]["name"], "shell");
        assert_eq!(blocks[0]["input"]["cmd"], "ls");
    }

    #[test]
    fn assistant_tool_calls_without_id_synthesise_one() {
        let history = vec![ChatHistoryMessage::AssistantToolCalls {
            items: vec![ToolCallItem {
                name: "shell".to_string(),
                params: serde_json::json!({}),
                id: None,
            }],
            provider_state: None,
        }];
        let (messages, _) = build_anthropic_messages(&history);
        let blocks = messages[0]["content"].as_array().unwrap();
        let id = blocks[0]["id"].as_str().unwrap();
        assert!(id.starts_with("toolu_"), "got: {id}");
    }

    #[test]
    fn tool_result_pairs_with_matching_pending_call() {
        let history = vec![
            ChatHistoryMessage::AssistantToolCalls {
                items: vec![ToolCallItem {
                    name: "shell".to_string(),
                    params: serde_json::json!({}),
                    id: Some("call_a".to_string()),
                }],
                provider_state: None,
            },
            ChatHistoryMessage::ToolResult {
                name: "shell".to_string(),
                content: "ok".to_string(),
            },
        ];
        let (messages, _) = build_anthropic_messages(&history);
        // assistant + user(tool_result)
        assert_eq!(messages.len(), 2);
        let result_block = &messages[1]["content"][0];
        assert_eq!(result_block["type"], "tool_result");
        assert_eq!(result_block["tool_use_id"], "call_a");
        assert_eq!(result_block["content"], "ok");
    }

    #[test]
    fn tool_result_without_matching_call_uses_unknown_sentinel() {
        let history = vec![ChatHistoryMessage::ToolResult {
            name: "ghost".to_string(),
            content: "unmatched".to_string(),
        }];
        let (messages, _) = build_anthropic_messages(&history);
        let result_block = &messages[0]["content"][0];
        let id = result_block["tool_use_id"].as_str().unwrap();
        assert!(id.contains("ghost"), "got: {id}");
    }

    // ── parse_response_json ──────────────────────────────────────────────────

    use super::parse_response_json;
    use assistant_core::{LlmResponse, LlmResponseMeta};

    fn meta() -> LlmResponseMeta {
        LlmResponseMeta::default()
    }

    #[test]
    fn parse_response_returns_text_for_plain_content() {
        let json = serde_json::json!({
            "content": [{"type": "text", "text": "Hello"}],
        });
        let resp = parse_response_json(&json, meta()).unwrap();
        match resp {
            LlmResponse::FinalAnswer(text, _) => assert_eq!(text, "Hello"),
            other => panic!("expected FinalAnswer, got {other:?}"),
        }
    }

    #[test]
    fn parse_response_concatenates_multiple_text_blocks() {
        let json = serde_json::json!({
            "content": [
                {"type": "text", "text": "Hello "},
                {"type": "text", "text": "world"},
            ],
        });
        let resp = parse_response_json(&json, meta()).unwrap();
        match resp {
            LlmResponse::FinalAnswer(text, _) => assert_eq!(text, "Hello world"),
            other => panic!("expected FinalAnswer, got {other:?}"),
        }
    }

    #[test]
    fn parse_response_returns_thinking_when_only_thinking_present() {
        let json = serde_json::json!({
            "content": [{"type": "thinking", "thinking": "reasoning..."}],
        });
        let resp = parse_response_json(&json, meta()).unwrap();
        match resp {
            LlmResponse::Thinking(text, _) => assert_eq!(text, "reasoning..."),
            other => panic!("expected Thinking, got {other:?}"),
        }
    }

    #[test]
    fn parse_response_prioritises_tool_use_over_text_and_thinking() {
        let json = serde_json::json!({
            "content": [
                {"type": "text", "text": "preamble"},
                {"type": "thinking", "thinking": "ponder"},
                {"type": "tool_use", "id": "toolu_1", "name": "shell", "input": {"cmd": "ls"}},
            ],
        });
        let resp = parse_response_json(&json, meta()).unwrap();
        match resp {
            LlmResponse::ToolCalls(tc) => {
                assert_eq!(tc.items.len(), 1);
                assert_eq!(tc.items[0].name, "shell");
                assert_eq!(tc.items[0].id.as_deref(), Some("toolu_1"));
                // Thinking propagates alongside tool calls.
                assert_eq!(tc.thinking.as_deref(), Some("ponder"));
            }
            other => panic!("expected ToolCalls, got {other:?}"),
        }
    }

    #[test]
    fn parse_response_skips_tool_use_with_empty_name() {
        let json = serde_json::json!({
            "content": [
                {"type": "tool_use", "id": "toolu_1", "name": "", "input": {}},
                {"type": "text", "text": "ok"},
            ],
        });
        // The empty-name tool_use is skipped; falls through to FinalAnswer.
        let resp = parse_response_json(&json, meta()).unwrap();
        assert!(matches!(resp, LlmResponse::FinalAnswer(_, _)));
    }

    #[test]
    fn parse_response_errors_when_content_missing() {
        let json = serde_json::json!({"id": "msg_1"});
        let err = parse_response_json(&json, meta()).unwrap_err();
        assert!(err.to_string().contains("content"));
    }

    // ── extract_anthropic_meta ───────────────────────────────────────────────

    use super::extract_anthropic_meta;

    #[test]
    fn extract_meta_pulls_top_level_fields() {
        let json = serde_json::json!({
            "model": "claude-3-5-sonnet",
            "id": "msg_abc",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 42, "output_tokens": 17},
        });
        let m = extract_anthropic_meta(&json);
        assert_eq!(m.model.as_deref(), Some("claude-3-5-sonnet"));
        assert_eq!(m.response_id.as_deref(), Some("msg_abc"));
        assert_eq!(m.finish_reason.as_deref(), Some("end_turn"));
        assert_eq!(m.input_tokens, Some(42));
        assert_eq!(m.output_tokens, Some(17));
    }

    #[test]
    fn extract_meta_counts_cache_tokens_as_input() {
        let json = serde_json::json!({
            "usage": {
                "input_tokens": 100,
                "cache_read_input_tokens": 900,
                "cache_creation_input_tokens": 50,
                "output_tokens": 10
            }
        });
        let m = extract_anthropic_meta(&json);
        assert_eq!(
            m.input_tokens,
            Some(1050),
            "Anthropic excludes cache tokens from input_tokens; semconv wants the total"
        );
        assert_eq!(m.cached_input_tokens, Some(900));
        assert_eq!(m.cache_write_input_tokens, Some(50));
        assert_eq!(m.reasoning_tokens, None);
        assert_eq!(m.cost_usd, None);
    }

    #[test]
    fn extract_meta_handles_missing_fields_gracefully() {
        let json = serde_json::json!({});
        let m = extract_anthropic_meta(&json);
        assert!(m.model.is_none());
        assert!(m.response_id.is_none());
        assert!(m.finish_reason.is_none());
        assert!(m.input_tokens.is_none());
        assert!(m.output_tokens.is_none());
    }

    // ── process_sse_event ────────────────────────────────────────────────────

    use super::{ToolBlock, process_sse_event};
    use assistant_core::StreamChunk;
    use tokio::sync::mpsc;

    fn empty_state() -> (
        String,
        String,
        Vec<ToolBlock>,
        Option<usize>,
        String,
        LlmResponseMeta,
    ) {
        (
            String::new(),
            String::new(),
            Vec::new(),
            None,
            String::new(),
            LlmResponseMeta::default(),
        )
    }

    #[tokio::test]
    async fn process_sse_event_message_start_populates_meta() {
        let (mut text, mut thinking, mut blocks, mut idx, mut block_type, mut meta) = empty_state();
        let evt = serde_json::json!({
            "message": {
                "model": "claude-3-5-sonnet",
                "id": "msg_xyz",
                "usage": {"input_tokens": 100}
            }
        });
        process_sse_event(
            "message_start",
            &evt,
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &None,
            &mut meta,
        )
        .await;
        assert_eq!(meta.model.as_deref(), Some("claude-3-5-sonnet"));
        assert_eq!(meta.response_id.as_deref(), Some("msg_xyz"));
        assert_eq!(meta.input_tokens, Some(100));
    }

    #[tokio::test]
    async fn process_sse_event_message_start_counts_cache_tokens() {
        let (mut text, mut thinking, mut blocks, mut idx, mut block_type, mut meta) = empty_state();
        let evt = serde_json::json!({
            "message": {
                "usage": {
                    "input_tokens": 5,
                    "cache_read_input_tokens": 2000,
                    "cache_creation_input_tokens": 300
                }
            }
        });
        process_sse_event(
            "message_start",
            &evt,
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &None,
            &mut meta,
        )
        .await;
        assert_eq!(meta.input_tokens, Some(2305));
        assert_eq!(meta.cached_input_tokens, Some(2000));
        assert_eq!(meta.cache_write_input_tokens, Some(300));
    }

    async fn feed(meta: &mut LlmResponseMeta, event: &str, json: serde_json::Value) {
        let (mut text, mut thinking, mut blocks, mut idx, mut block_type, _) = empty_state();
        process_sse_event(
            event,
            &json,
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &None,
            meta,
        )
        .await;
    }

    #[tokio::test]
    async fn message_delta_updates_cumulative_cache_counts() {
        let mut meta = LlmResponseMeta::default();
        feed(
            &mut meta,
            "message_start",
            serde_json::json!({"message": {"usage": {
                "input_tokens": 5,
                "cache_read_input_tokens": 2000,
                "cache_creation_input_tokens": 300
            }}}),
        )
        .await;
        feed(
            &mut meta,
            "message_delta",
            serde_json::json!({
                "delta": {"stop_reason": "end_turn"},
                "usage": {"output_tokens": 50, "input_tokens": 7, "cache_read_input_tokens": 2100}
            }),
        )
        .await;
        assert_eq!(meta.output_tokens, Some(50));
        assert_eq!(
            meta.cached_input_tokens,
            Some(2100),
            "the delta's cumulative count wins"
        );
        assert_eq!(
            meta.cache_write_input_tokens,
            Some(300),
            "a count absent from the delta keeps its message_start value"
        );
        assert_eq!(meta.input_tokens, Some(7 + 2100 + 300));
    }

    #[tokio::test]
    async fn message_delta_with_only_output_tokens_keeps_input_counts() {
        let mut meta = LlmResponseMeta::default();
        feed(
            &mut meta,
            "message_start",
            serde_json::json!({"message": {"usage": {"input_tokens": 5, "cache_read_input_tokens": 2000}}}),
        )
        .await;
        feed(
            &mut meta,
            "message_delta",
            serde_json::json!({"delta": {}, "usage": {"output_tokens": 9}}),
        )
        .await;
        assert_eq!(meta.input_tokens, Some(2005));
        assert_eq!(meta.cached_input_tokens, Some(2000));
        assert_eq!(meta.output_tokens, Some(9));
    }

    #[tokio::test]
    async fn process_sse_event_text_delta_appends_and_streams_chunk() {
        let (mut text, mut thinking, mut blocks, mut idx, mut block_type, mut meta) = empty_state();
        let (tx, mut rx) = mpsc::channel::<StreamChunk>(8);
        let evt = serde_json::json!({"delta": {"type": "text_delta", "text": "Hello"}});
        process_sse_event(
            "content_block_delta",
            &evt,
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &Some(tx),
            &mut meta,
        )
        .await;
        assert_eq!(text, "Hello");
        let chunk = rx.recv().await.unwrap();
        match chunk {
            StreamChunk::Text(t) => assert_eq!(t, "Hello"),
            _ => panic!("expected Text chunk"),
        }
    }

    #[tokio::test]
    async fn process_sse_event_thinking_delta_appends_and_streams_chunk() {
        let (mut text, mut thinking, mut blocks, mut idx, mut block_type, mut meta) = empty_state();
        let (tx, mut rx) = mpsc::channel::<StreamChunk>(8);
        let evt = serde_json::json!({"delta": {"type": "thinking_delta", "thinking": "ponder"}});
        process_sse_event(
            "content_block_delta",
            &evt,
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &Some(tx),
            &mut meta,
        )
        .await;
        assert_eq!(thinking, "ponder");
        let chunk = rx.recv().await.unwrap();
        assert!(matches!(chunk, StreamChunk::Thinking(s) if s == "ponder"));
    }

    #[tokio::test]
    async fn process_sse_event_content_block_start_tool_use_registers_tool() {
        let (mut text, mut thinking, mut blocks, mut idx, mut block_type, mut meta) = empty_state();
        let evt = serde_json::json!({
            "index": 0,
            "content_block": {"type": "tool_use", "id": "toolu_1", "name": "shell"}
        });
        process_sse_event(
            "content_block_start",
            &evt,
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &None,
            &mut meta,
        )
        .await;
        assert_eq!(idx, Some(0));
        assert_eq!(block_type, "tool_use");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id, "toolu_1");
        assert_eq!(blocks[0].name, "shell");
    }

    #[tokio::test]
    async fn process_sse_event_input_json_delta_appends_partial_json() {
        let (mut text, mut thinking, mut blocks, mut idx, mut block_type, mut meta) = empty_state();
        // First register a tool_use block at index 0.
        process_sse_event(
            "content_block_start",
            &serde_json::json!({
                "index": 0,
                "content_block": {"type": "tool_use", "id": "toolu_1", "name": "shell"}
            }),
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &None,
            &mut meta,
        )
        .await;
        // Then send an input_json_delta.
        process_sse_event(
            "content_block_delta",
            &serde_json::json!({"delta": {"type": "input_json_delta", "partial_json": "{\"cmd\":\"ls\"}"}}),
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &None,
            &mut meta,
        )
        .await;
        assert_eq!(blocks[0].partial_json, r#"{"cmd":"ls"}"#);
    }

    #[tokio::test]
    async fn process_sse_event_message_delta_populates_finish_and_tokens() {
        let (mut text, mut thinking, mut blocks, mut idx, mut block_type, mut meta) = empty_state();
        let evt = serde_json::json!({
            "delta": {"stop_reason": "end_turn"},
            "usage": {"output_tokens": 50},
        });
        process_sse_event(
            "message_delta",
            &evt,
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &None,
            &mut meta,
        )
        .await;
        assert_eq!(meta.finish_reason.as_deref(), Some("end_turn"));
        assert_eq!(meta.output_tokens, Some(50));
    }

    #[tokio::test]
    async fn process_sse_event_unknown_event_type_is_a_noop() {
        let (mut text, mut thinking, mut blocks, mut idx, mut block_type, mut meta) = empty_state();
        process_sse_event(
            "ping",
            &serde_json::json!({}),
            &mut text,
            &mut thinking,
            &mut blocks,
            &mut idx,
            &mut block_type,
            &None,
            &mut meta,
        )
        .await;
        assert!(text.is_empty());
        assert!(thinking.is_empty());
        assert!(blocks.is_empty());
    }

    mod chat_options {
        use serde_json::json;
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use assistant_core::{
            ChatHistoryMessage, ChatOptions, ChatRole, LlmProvider, ToolChoice, ToolSpec,
        };

        use super::super::{AnthropicConfig, AnthropicProvider};

        fn provider(base_url: &str) -> AnthropicProvider {
            AnthropicProvider::new(AnthropicConfig {
                api_key: assistant_core::Secret::new("test"),
                base_url: base_url.to_string(),
                ..AnthropicConfig::default()
            })
            .unwrap()
        }

        fn tool() -> ToolSpec {
            ToolSpec {
                name: "submit".into(),
                description: "Submit".into(),
                params_schema: json!({"type": "object"}),
                is_mutating: false,
                requires_confirmation: false,
                requires_rich_client: false,
            }
        }

        fn history() -> Vec<ChatHistoryMessage> {
            vec![ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "hi".into(),
            }]
        }

        fn body(options: ChatOptions) -> serde_json::Value {
            provider("http://unused").request_body("sys", &history(), &[tool()], &options, false)
        }

        fn choose(tool_choice: ToolChoice) -> ChatOptions {
            ChatOptions {
                tool_choice,
                ..ChatOptions::default()
            }
        }

        #[test]
        fn default_options_send_no_tool_choice() {
            assert!(body(ChatOptions::default()).get("tool_choice").is_none());
        }

        #[test]
        fn tool_choices_map_to_anthropic_types() {
            let cases = [
                (ToolChoice::None, json!({"type": "none"})),
                (ToolChoice::Required, json!({"type": "any"})),
                (
                    ToolChoice::Tool("submit".into()),
                    json!({"type": "tool", "name": "submit"}),
                ),
            ];
            for (choice, expected) in cases {
                assert_eq!(
                    body(choose(choice.clone()))["tool_choice"],
                    expected,
                    "{choice:?}"
                );
            }
        }

        #[test]
        fn parallel_tool_calls_false_disables_parallel_use() {
            let options = ChatOptions {
                parallel_tool_calls: Some(false),
                ..ChatOptions::default()
            };
            assert_eq!(
                body(options)["tool_choice"],
                json!({"type": "auto", "disable_parallel_tool_use": true})
            );
            let forced = ChatOptions {
                tool_choice: ToolChoice::Required,
                parallel_tool_calls: Some(true),
                ..ChatOptions::default()
            };
            assert_eq!(
                body(forced)["tool_choice"],
                json!({"type": "any", "disable_parallel_tool_use": false})
            );
            let none = ChatOptions {
                tool_choice: ToolChoice::None,
                parallel_tool_calls: Some(false),
                ..ChatOptions::default()
            };
            assert_eq!(
                body(none)["tool_choice"],
                json!({"type": "none"}),
                "the none choice takes no parallel flag"
            );
        }

        #[test]
        fn no_tool_choice_without_tools() {
            let body = provider("http://unused").request_body(
                "sys",
                &history(),
                &[],
                &choose(ToolChoice::Required),
                false,
            );
            assert!(
                body.get("tool_choice").is_none(),
                "tool_choice needs tools: {body}"
            );
        }

        #[tokio::test]
        async fn chat_structured_forces_the_respond_tool() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .and(body_partial_json(json!({
                    "tools": [{"name": "respond", "input_schema": {"required": ["verdict"]}}],
                    "tool_choice": {"type": "tool", "name": "respond"},
                })))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "id": "msg_1",
                    "model": "claude",
                    "stop_reason": "tool_use",
                    "content": [{
                        "type": "tool_use",
                        "id": "t1",
                        "name": "respond",
                        "input": {"verdict": "keep"},
                    }],
                })))
                .expect(1)
                .mount(&server)
                .await;

            let schema = json!({
                "type": "object",
                "properties": {"verdict": {"type": "string"}},
                "required": ["verdict"],
            });
            let value = assistant_core::chat_structured(
                &provider(&server.uri()),
                "sys",
                &history(),
                "decision",
                &schema,
            )
            .await
            .unwrap();
            assert_eq!(value, json!({"verdict": "keep"}));
        }

        #[tokio::test]
        async fn chat_with_sends_tool_choice() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .and(body_partial_json(
                    json!({"tool_choice": {"type": "tool", "name": "submit"}}),
                ))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "id": "msg_1",
                    "model": "claude",
                    "stop_reason": "tool_use",
                    "content": [{"type": "tool_use", "id": "t1", "name": "submit", "input": {}}],
                })))
                .expect(1)
                .mount(&server)
                .await;

            let response = provider(&server.uri())
                .chat_with(
                    "sys",
                    &history(),
                    &[tool()],
                    &choose(ToolChoice::Tool("submit".into())),
                    None,
                )
                .await
                .unwrap();
            assert!(
                matches!(response, assistant_core::LlmResponse::ToolCalls(_)),
                "{response:?}"
            );
        }
    }
}
