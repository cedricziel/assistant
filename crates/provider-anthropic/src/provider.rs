//! `AnthropicProvider` — [`LlmProvider`] implementation backed by the Anthropic Messages API.

use async_trait::async_trait;
use futures::StreamExt as _;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::LlmConfig;
use assistant_core::types::{
    AnthropicUserLocation, AnthropicWebFetchOptions, AnthropicWebSearchOptions,
};
use assistant_llm::{
    Capabilities, ChatHistoryMessage, ChatRole, ContentBlock, HostedTool, LlmProvider, LlmResponse,
    LlmResponseMeta, StreamChunk, ToolCallItem, ToolCallResponse, ToolSpec, ToolSupport,
};

// ── AnthropicConfig ───────────────────────────────────────────────────────────

/// Configuration for the Anthropic backend.
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// Model ID, e.g. `"claude-opus-4-6"`.
    pub model: String,
    /// Anthropic API key.  Falls back to `ANTHROPIC_API_KEY` env var at construction time.
    pub api_key: String,
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
            api_key: String::new(),
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
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
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
        let http = assistant_llm::build_http_client(
            config.timeout_secs,
            &assistant_llm::RetryConfig::default(),
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
                    .map(|block| match block {
                        ContentBlock::Text(text) => json!({"type": "text", "text": text}),
                        ContentBlock::Image { media_type, data } => json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": media_type,
                                "data": data,
                            }
                        }),
                        ContentBlock::Document { media_type, data } => json!({
                            "type": "document",
                            "source": {
                                "type": "base64",
                                "media_type": media_type,
                                "data": data,
                            }
                        }),
                    })
                    .collect();
                messages.push(json!({"role": "user", "content": blocks}));
            }

            ChatHistoryMessage::AssistantToolCalls(calls) => {
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
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        self.chat_sse(system_prompt, history, tools, chunk_sink)
            .await
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
    // ── Non-streaming ─────────────────────────────────────────────────────────

    async fn chat_non_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        debug!(model = %self.config.model, tools = tools.len(), "Sending request to Anthropic");

        let (messages, _) = build_anthropic_messages(history);
        let mut request_tools: Vec<Value> = tools.iter().map(tool_spec_to_anthropic_json).collect();
        request_tools.extend(self.server_tool_specs());

        let mut body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": messages,
        });

        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }
        if !request_tools.is_empty() {
            body["tools"] = json!(request_tools);
        }

        let url = format!("{}/v1/messages", self.config.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.config.api_key)
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
        token_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        debug!(
            model = %self.config.model,
            "Sending streaming request to Anthropic"
        );

        let (messages, _) = build_anthropic_messages(history);
        let mut request_tools: Vec<Value> = tools.iter().map(tool_spec_to_anthropic_json).collect();
        request_tools.extend(self.server_tool_specs());

        let mut body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": messages,
            "stream": true,
        });

        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }
        if !request_tools.is_empty() {
            body["tools"] = json!(request_tools);
        }

        let url = format!("{}/v1/messages", self.config.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.config.api_key)
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
                meta.input_tokens = msg.pointer("/usage/input_tokens").and_then(|v| v.as_u64());
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
        }));
    }
    if !thinking_parts.is_empty() {
        return Ok(LlmResponse::Thinking(thinking_parts.join(""), meta));
    }
    Ok(LlmResponse::FinalAnswer(text_parts.join(""), meta))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use assistant_llm::{ChatHistoryMessage, ChatRole, ContentBlock};

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
}

/// Extract [`LlmResponseMeta`] from an Anthropic non-streaming JSON response.
///
/// Top-level fields: `model`, `id`, `stop_reason`, `usage.input_tokens`,
/// `usage.output_tokens`.
fn extract_anthropic_meta(json: &Value) -> LlmResponseMeta {
    LlmResponseMeta {
        model: json.get("model").and_then(|v| v.as_str()).map(String::from),
        response_id: json.get("id").and_then(|v| v.as_str()).map(String::from),
        finish_reason: json
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(String::from),
        input_tokens: json.pointer("/usage/input_tokens").and_then(|v| v.as_u64()),
        output_tokens: json
            .pointer("/usage/output_tokens")
            .and_then(|v| v.as_u64()),
    }
}
