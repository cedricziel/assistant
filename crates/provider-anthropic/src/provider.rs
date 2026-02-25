//! `AnthropicProvider` — [`LlmProvider`] implementation backed by the Anthropic Messages API.

use async_trait::async_trait;
use futures::StreamExt as _;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::types::{
    AnthropicUserLocation, AnthropicWebFetchOptions, AnthropicWebSearchOptions,
};
use assistant_core::LlmConfig;
use assistant_llm::{
    Capabilities, ChatHistoryMessage, ChatRole, HostedTool, LlmProvider, LlmResponse,
    LlmResponseMeta, ToolCallItem, ToolSpec, ToolSupport,
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
    http: reqwest::Client,
}

impl AnthropicProvider {
    /// Create from explicit config.
    pub fn new(config: AnthropicConfig) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))?;
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
    let schema = &tool.params_schema;

    let input_schema = if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
        schema.clone()
    } else if schema.as_object().is_some() {
        json!({"type": "object", "properties": schema})
    } else {
        json!({"type": "object", "properties": {}, "required": []})
    };

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
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        self.chat_sse(system_prompt, history, tools, token_sink)
            .await
    }

    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!(
            "Anthropic does not support text embeddings"
        ))
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
        token_sink: Option<mpsc::Sender<String>>,
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
        // tool blocks: index → (id, name, partial_json)
        let mut tool_blocks: Vec<(String, String, String)> = Vec::new();
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
                .filter_map(|(id, name, partial_json)| {
                    if name.is_empty() {
                        return None;
                    }
                    let params = serde_json::from_str::<Value>(&partial_json)
                        .unwrap_or(Value::Object(serde_json::Map::new()));
                    Some(ToolCallItem {
                        name,
                        params,
                        id: Some(id),
                    })
                })
                .collect();
            if !items.is_empty() {
                debug!(count = items.len(), "Anthropic SSE: tool calls received");
                return Ok(LlmResponse::ToolCalls(items, sse_meta));
            }
        }

        if !thinking_buf.is_empty() {
            return Ok(LlmResponse::Thinking(thinking_buf, sse_meta));
        }

        Ok(LlmResponse::FinalAnswer(text_buf, sse_meta))
    }
}

// ── SSE event processor ───────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn process_sse_event(
    event_type: &str,
    json: &Value,
    text_buf: &mut String,
    thinking_buf: &mut String,
    tool_blocks: &mut Vec<(String, String, String)>,
    current_block_idx: &mut Option<usize>,
    current_block_type: &mut String,
    token_sink: &Option<mpsc::Sender<String>>,
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
                    tool_blocks.push((String::new(), String::new(), String::new()));
                }
                tool_blocks[idx] = (id, name, String::new());
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
                            let _ = sink.send(text.to_string()).await;
                        }
                    }
                }
                "thinking_delta" => {
                    if let Some(thinking) = json.pointer("/delta/thinking").and_then(|v| v.as_str())
                    {
                        thinking_buf.push_str(thinking);
                    }
                }
                "input_json_delta" => {
                    if let Some(idx) = *current_block_idx {
                        if let Some(partial) =
                            json.pointer("/delta/partial_json").and_then(|v| v.as_str())
                        {
                            if idx < tool_blocks.len() {
                                tool_blocks[idx].2.push_str(partial);
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
        return Ok(LlmResponse::ToolCalls(tool_calls, meta));
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
