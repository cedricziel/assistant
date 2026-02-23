//! `AnthropicProvider` — [`LlmProvider`] implementation backed by the Anthropic Messages API.

use async_trait::async_trait;
use futures::StreamExt as _;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::{LlmConfig, SkillDef};
use assistant_llm::{
    Capabilities, ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse, ToolCallItem, ToolSupport,
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
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            model: "claude-opus-4-6".to_string(),
            api_key: String::new(),
            base_url: "https://api.anthropic.com".to_string(),
            timeout_secs: 120,
            max_tokens: 8192,
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
        })
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
}

// ── Message conversion ────────────────────────────────────────────────────────

/// Convert a [`SkillDef`] to the Anthropic `tools` array entry.
///
/// Anthropic uses `input_schema` (not `parameters` like OpenAI/Ollama).
fn skill_to_anthropic_tool(skill: &SkillDef) -> Value {
    let raw = skill.params_schema();

    let input_schema = match raw {
        None => json!({"type": "object", "properties": {}, "required": []}),
        Some(schema) if schema.get("type").and_then(|t| t.as_str()) == Some("object") => schema,
        Some(flat) => json!({"type": "object", "properties": flat}),
    };

    json!({
        "name": skill.name,
        "description": skill.description,
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
                // Look up the tool_use_id by matching the tool name from pending_ids.
                let tool_use_id = pending_ids
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, id)| id.clone())
                    .unwrap_or_else(|| format!("toolu_unknown_{name}"));

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
        }
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
    ) -> anyhow::Result<LlmResponse> {
        self.chat_non_streaming(system_prompt, history, skills)
            .await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        self.chat_sse(system_prompt, history, skills, token_sink)
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
        skills: &[&SkillDef],
    ) -> anyhow::Result<LlmResponse> {
        debug!(model = %self.config.model, skills = skills.len(), "Sending request to Anthropic");

        let (messages, _) = build_anthropic_messages(history);
        let tools: Vec<Value> = skills.iter().map(|s| skill_to_anthropic_tool(s)).collect();

        let mut body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": messages,
        });

        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }
        if !tools.is_empty() {
            body["tools"] = json!(tools);
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
        parse_response_json(&json)
    }

    // ── SSE streaming ─────────────────────────────────────────────────────────

    async fn chat_sse(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        debug!(
            model = %self.config.model,
            "Sending streaming request to Anthropic"
        );

        let (messages, _) = build_anthropic_messages(history);
        let tools: Vec<Value> = skills.iter().map(|s| skill_to_anthropic_tool(s)).collect();

        let mut body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": messages,
            "stream": true,
        });

        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }
        if !tools.is_empty() {
            body["tools"] = json!(tools);
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

        let mut byte_stream = resp.bytes_stream();
        let mut line_buf = String::new();
        let mut event_type = String::new();

        while let Some(chunk) = byte_stream.next().await {
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
                            break;
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
                return Ok(LlmResponse::ToolCalls(items));
            }
        }

        if !thinking_buf.is_empty() {
            return Ok(LlmResponse::Thinking(thinking_buf));
        }

        Ok(LlmResponse::FinalAnswer(text_buf))
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
) {
    match event_type {
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

        // message_delta and message_stop are informational; no state needed.
        _ => {}
    }
}

// ── Response parser (non-streaming) ──────────────────────────────────────────

fn parse_response_json(json: &Value) -> anyhow::Result<LlmResponse> {
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
        return Ok(LlmResponse::ToolCalls(tool_calls));
    }
    if !thinking_parts.is_empty() {
        return Ok(LlmResponse::Thinking(thinking_parts.join("")));
    }
    Ok(LlmResponse::FinalAnswer(text_parts.join("")))
}
