use anyhow::Context as _;
use async_trait::async_trait;
use futures::StreamExt as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::debug;

use crate::provider::{Capabilities, LlmProvider, ToolSupport};
use crate::tool_spec::ToolSpec;

// ── Public types ──────────────────────────────────────────────────────────────

/// A single content block in a multimodal message.
///
/// Used by [`ChatHistoryMessage::MultimodalUser`] to carry a mix of text and
/// inline images.
#[derive(Debug, Clone)]
pub enum ContentBlock {
    /// A text segment.
    Text(String),
    /// A base64-encoded image.
    Image {
        /// MIME type, e.g. `"image/png"`, `"image/jpeg"`.
        media_type: String,
        /// Base64-encoded image data (no data-URI prefix).
        data: String,
    },
}

/// A single message in the chat history as tracked by the caller.
///
/// The enum reflects the structurally distinct message kinds in the
/// Ollama (and OpenAI-compatible) multi-turn tool-calling format:
///
/// * `Text` — a plain user, assistant, or system message.
/// * `MultimodalUser` — a user message with mixed text and image content.
/// * `AssistantToolCalls` — the assistant's decision to invoke one or more
///   tools.  Serialises to `{"role":"assistant","content":"","tool_calls":[…]}`.
/// * `ToolResult` — the result returned for a single tool invocation.
///   Serialises to `{"role":"tool","name":"…","content":"…"}`.
#[derive(Debug, Clone)]
pub enum ChatHistoryMessage {
    /// A plain text message (user / assistant / system).
    Text { role: ChatRole, content: String },
    /// A user message with mixed text and image content blocks.
    ///
    /// Providers that do not support vision should extract only the
    /// [`ContentBlock::Text`] parts and discard images.
    MultimodalUser { content: Vec<ContentBlock> },
    /// The assistant requested one or more tool calls in a single turn.
    AssistantToolCalls(Vec<ToolCallItem>),
    /// The result of a single tool invocation.
    ToolResult { name: String, content: String },
}

/// Chat participant role.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallItem {
    pub name: String,
    pub params: serde_json::Value,
    /// Provider-assigned call ID (e.g. Anthropic `tool_use_id`).
    /// `None` for providers that do not use IDs (Ollama).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Metadata returned alongside the model's response.
///
/// Fields are best-effort: providers populate what they can, leaving the rest
/// as `None` / `0`.  The struct is intentionally cheap to clone.
#[derive(Debug, Clone, Default)]
pub struct LlmResponseMeta {
    /// Model identifier echoed by the provider (e.g. `"qwen2.5:7b"`).
    pub model: Option<String>,
    /// Number of tokens in the prompt (input).
    pub input_tokens: Option<u64>,
    /// Number of tokens in the completion (output).
    pub output_tokens: Option<u64>,
    /// Provider-specific finish/stop reason (e.g. `"stop"`, `"tool_calls"`).
    pub finish_reason: Option<String>,
    /// Provider-assigned response/message ID.
    pub response_id: Option<String>,
}

/// The outcome of a single `LlmClient::chat` invocation.
#[derive(Debug, Clone)]
pub enum LlmResponse {
    /// The model wants to call one or more tools.
    ToolCalls(Vec<ToolCallItem>, LlmResponseMeta),
    /// The model has a definitive answer for the user.
    FinalAnswer(String, LlmResponseMeta),
    /// The model emitted only a reasoning step (no action yet).
    Thinking(String, LlmResponseMeta),
}

impl LlmResponse {
    /// Access the response metadata regardless of variant.
    pub fn meta(&self) -> &LlmResponseMeta {
        match self {
            LlmResponse::ToolCalls(_, m) => m,
            LlmResponse::FinalAnswer(_, m) => m,
            LlmResponse::Thinking(_, m) => m,
        }
    }
}

/// Configuration for the LLM client.
#[derive(Debug, Clone)]
pub struct LlmClientConfig {
    pub model: String,
    pub base_url: String,
    pub timeout_secs: u64,
}

impl Default for LlmClientConfig {
    fn default() -> Self {
        Self {
            model: "qwen2.5:7b".to_string(),
            base_url: "http://localhost:11434".to_string(),
            timeout_secs: 120,
        }
    }
}

impl From<&assistant_core::LlmConfig> for LlmClientConfig {
    fn from(cfg: &assistant_core::LlmConfig) -> Self {
        Self {
            model: cfg.model.clone(),
            base_url: cfg.base_url.clone(),
            timeout_secs: cfg.timeout_secs,
        }
    }
}

// ── LlmClient ────────────────────────────────────────────────────────────────

/// High-level LLM client using Ollama native tool-calling.
///
/// Sends requests with a `tools` array (built from ToolSpec definitions) to the
/// Ollama `/api/chat` endpoint and parses `tool_calls` from the JSON response.
pub struct LlmClient {
    config: LlmClientConfig,
    /// Shared reqwest client (with tracing middleware) for all requests.
    http: reqwest_middleware::ClientWithMiddleware,
}

impl LlmClient {
    /// Create a new client from the given configuration.
    pub fn new(config: LlmClientConfig) -> anyhow::Result<Self> {
        let http = crate::http::build_http_client(
            config.timeout_secs,
            &crate::retry::RetryConfig::default(),
        )?;

        Ok(Self { config, http })
    }

    /// Create a client directly from a `LlmConfig` (convenience wrapper).
    pub fn from_llm_config(cfg: &assistant_core::LlmConfig) -> anyhow::Result<Self> {
        Self::new(LlmClientConfig::from(cfg))
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Send a chat turn and return the model's response.
    pub async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        self.chat_native(system_prompt, history, tools).await
    }

    /// Like [`chat`] but streams final-answer tokens through `token_sink`.
    pub async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        self.chat_native_streaming(system_prompt, history, tools, token_sink)
            .await
    }

    // ── Native tool-calling (via reqwest) ────────────────────────────────────

    async fn chat_native(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        debug!(
            model = %self.config.model,
            tools = tools.len(),
            "Sending native tool-call request to Ollama"
        );

        // Ollama vision support depends on the model; default to false to avoid
        // sending images to non-vision models which would cause a rejection.
        let messages = build_json_messages(system_prompt, history, false);
        let tools_json: Vec<Value> = tools.iter().map(tool_spec_to_ollama_json).collect();

        let role_summary: Vec<&str> = messages
            .iter()
            .filter_map(|m| m.get("role").and_then(|r| r.as_str()))
            .collect();
        debug!(
            messages = messages.len(),
            roles = ?role_summary,
            "Message history sent to Ollama"
        );

        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "tools": tools_json,
            "stream": false,
        });

        let url = format!("{}/api/chat", self.config.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("HTTP request to Ollama /api/chat failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama returned {status}: {text}");
        }

        let json: Value = resp
            .json()
            .await
            .context("failed to parse Ollama JSON response")?;

        debug!("Native tool-call response received");

        // Extract response metadata from the Ollama JSON envelope.
        let meta = extract_ollama_meta(&json);

        if let Some(tool_calls) = json
            .pointer("/message/tool_calls")
            .and_then(|v| v.as_array())
        {
            let items: Vec<ToolCallItem> = tool_calls
                .iter()
                .filter_map(|tc| {
                    let name = tc
                        .pointer("/function/name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if name.is_empty() {
                        return None;
                    }
                    let params = tc
                        .pointer("/function/arguments")
                        .cloned()
                        .unwrap_or(Value::Object(serde_json::Map::new()));
                    Some(ToolCallItem {
                        name,
                        params,
                        id: None,
                    })
                })
                .collect();
            if !items.is_empty() {
                debug!(count = items.len(), "Native tool calls received");
                return Ok(LlmResponse::ToolCalls(items, meta));
            }
        }

        let content = json
            .pointer("/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Thinking models (e.g. qwen3) emit reasoning in a separate `/message/thinking`
        // field and may leave `/message/content` empty when no visible text follows the
        // think block.  Returning an empty FinalAnswer causes downstream callers (e.g.
        // the Slack auto-post path) to send an empty message.  Instead, if content is
        // empty but thinking is present, surface it as a Thinking step so the
        // orchestrator adds it to history and re-prompts the model for a visible reply.
        if content.trim().is_empty() {
            if let Some(thinking) = json
                .pointer("/message/thinking")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
            {
                debug!("Model returned empty content with non-empty thinking; surfacing as Thinking step");
                return Ok(LlmResponse::Thinking(thinking.to_string(), meta));
            }
        }

        debug!("Native request returned no tool_calls; treating as final answer");
        Ok(LlmResponse::FinalAnswer(content, meta))
    }

    async fn chat_native_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        debug!(
            model = %self.config.model,
            "Sending native streaming request to Ollama"
        );

        let messages = build_json_messages(system_prompt, history, false);
        let tools_json: Vec<Value> = tools.iter().map(tool_spec_to_ollama_json).collect();

        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "tools": tools_json,
            "stream": true,
        });

        let url = format!("{}/api/chat", self.config.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("HTTP streaming request to Ollama /api/chat failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama returned {status}: {text}");
        }

        let mut content = String::new();
        let mut tool_calls_json: Option<Value> = None;
        let mut final_json: Option<Value> = None;

        let mut byte_stream = resp.bytes_stream();
        let mut line_buf = String::new();

        while let Some(chunk) = byte_stream.next().await {
            let chunk = chunk.context("Stream read error (native)")?;
            let text = String::from_utf8_lossy(&chunk);

            for ch in text.chars() {
                if ch == '\n' {
                    let line = std::mem::take(&mut line_buf);
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if let Ok(json) = serde_json::from_str::<Value>(line) {
                        if let Some(token) = json
                            .pointer("/message/content")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                        {
                            content.push_str(token);
                            if let Some(ref sink) = token_sink {
                                let _ = sink.send(token.to_string()).await;
                            }
                        }

                        if let Some(tc) = json.pointer("/message/tool_calls") {
                            if tc.as_array().is_some_and(|a| !a.is_empty()) {
                                tool_calls_json = Some(tc.clone());
                            }
                        }

                        // The final chunk carries `done: true` and the metadata.
                        if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                            final_json = Some(json);
                        }
                    }
                } else {
                    line_buf.push(ch);
                }
            }
        }

        if !line_buf.is_empty() {
            if let Ok(json) = serde_json::from_str::<Value>(&line_buf) {
                if let Some(token) = json
                    .pointer("/message/content")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    content.push_str(token);
                    if let Some(ref sink) = token_sink {
                        let _ = sink.send(token.to_string()).await;
                    }
                }
                if let Some(tc) = json.pointer("/message/tool_calls") {
                    if tc.as_array().is_some_and(|a| !a.is_empty()) {
                        tool_calls_json = Some(tc.clone());
                    }
                }
                if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                    final_json = Some(json);
                }
            }
        }

        debug!("Native streaming response complete");

        let meta = final_json
            .as_ref()
            .map(extract_ollama_meta)
            .unwrap_or_default();

        if let Some(tc) = tool_calls_json {
            if let Some(arr) = tc.as_array() {
                let items: Vec<ToolCallItem> = arr
                    .iter()
                    .filter_map(|entry| {
                        let name = entry
                            .pointer("/function/name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if name.is_empty() {
                            return None;
                        }
                        let params = entry
                            .pointer("/function/arguments")
                            .cloned()
                            .unwrap_or(Value::Object(serde_json::Map::new()));
                        Some(ToolCallItem {
                            name,
                            params,
                            id: None,
                        })
                    })
                    .collect();
                if !items.is_empty() {
                    debug!(count = items.len(), "Native streaming: tool calls received");
                    return Ok(LlmResponse::ToolCalls(items, meta));
                }
            }
        }

        Ok(LlmResponse::FinalAnswer(content, meta))
    }
}

// ── LlmProvider impl ─────────────────────────────────────────────────────────

#[async_trait]
impl LlmProvider for LlmClient {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: ToolSupport::Native,
            streaming: true,
            vision: false,
            hosted_tools: Vec::new(),
        }
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        self.chat_native(system_prompt, history, tools).await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        self.chat_native_streaming(system_prompt, history, tools, token_sink)
            .await
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn server_address(&self) -> &str {
        &self.config.base_url
    }
}

// ── Free helpers ──────────────────────────────────────────────────────────────

/// Extract [`LlmResponseMeta`] from an Ollama JSON response object.
///
/// Ollama puts metadata at the top level:
///   `model`, `prompt_eval_count` (input tokens), `eval_count` (output tokens),
///   `done_reason` (finish reason).
fn extract_ollama_meta(json: &Value) -> LlmResponseMeta {
    LlmResponseMeta {
        model: json.get("model").and_then(|v| v.as_str()).map(String::from),
        input_tokens: json.get("prompt_eval_count").and_then(|v| v.as_u64()),
        output_tokens: json.get("eval_count").and_then(|v| v.as_u64()),
        finish_reason: json
            .get("done_reason")
            .and_then(|v| v.as_str())
            .map(String::from),
        response_id: None, // Ollama does not emit a response ID.
    }
}

/// Convert a [`ToolSpec`] to the JSON structure expected by the Ollama
/// `tools` array in the `/api/chat` request body.
pub fn tool_spec_to_ollama_json(tool: &ToolSpec) -> Value {
    let schema = &tool.params_schema;

    // Normalise to a proper JSON Schema object.
    let parameters = if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
        schema.clone()
    } else if schema.as_object().is_some() {
        json!({"type": "object", "properties": schema})
    } else {
        json!({"type": "object", "properties": {}, "required": []})
    };

    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": parameters,
        }
    })
}

/// Build the JSON messages array for the native (reqwest) path.
///
/// When `vision` is `false`, image content blocks inside
/// [`ChatHistoryMessage::MultimodalUser`] are discarded so that non-vision
/// Ollama models do not reject the request.
fn build_json_messages(
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    vision: bool,
) -> Vec<Value> {
    let mut messages = Vec::with_capacity(history.len() + 1);

    if !system_prompt.is_empty() {
        messages.push(json!({ "role": "system", "content": system_prompt }));
    }

    for msg in history {
        match msg {
            ChatHistoryMessage::Text { role, content } => {
                let role_str = match role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                };
                messages.push(json!({ "role": role_str, "content": content }));
            }
            ChatHistoryMessage::AssistantToolCalls(calls) => {
                let tool_calls: Vec<Value> = calls
                    .iter()
                    .map(|c| {
                        json!({
                            "function": {
                                "name": c.name,
                                "arguments": c.params,
                            }
                        })
                    })
                    .collect();
                messages.push(json!({
                    "role": "assistant",
                    "content": "",
                    "tool_calls": tool_calls,
                }));
            }
            ChatHistoryMessage::MultimodalUser { content } => {
                // Ollama supports images via an `images` field alongside `content`.
                // When vision is disabled, discard image blocks so that non-vision
                // models do not reject the request.
                let mut texts = Vec::new();
                let mut images = Vec::new();
                for block in content {
                    match block {
                        ContentBlock::Text(t) => texts.push(t.as_str()),
                        ContentBlock::Image { data, .. } if vision => {
                            images.push(json!(data));
                        }
                        ContentBlock::Image { .. } => { /* vision disabled — skip */ }
                    }
                }
                let combined_text = texts.join("\n");
                if images.is_empty() {
                    messages.push(json!({ "role": "user", "content": combined_text }));
                } else {
                    messages.push(
                        json!({ "role": "user", "content": combined_text, "images": images }),
                    );
                }
            }
            ChatHistoryMessage::ToolResult { name, content } => {
                messages.push(json!({
                    "role": "tool",
                    "name": name,
                    "content": content,
                }));
            }
        }
    }

    messages
}

#[cfg(test)]
mod tests {
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    fn make_client(base_url: &str) -> LlmClient {
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: base_url.to_string(),
            timeout_secs: 5,
        })
        .unwrap()
    }

    fn tool_calls_body(calls: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "model": "test",
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": calls
            },
            "done": true
        })
    }

    fn answer_body(text: &str) -> serde_json::Value {
        serde_json::json!({
            "model": "test",
            "message": { "role": "assistant", "content": text },
            "done": true
        })
    }

    #[tokio::test]
    async fn parses_single_tool_call() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tool_calls_body(
                serde_json::json!([
                    { "function": { "name": "my-tool", "arguments": { "key": "val" } } }
                ]),
            )))
            .mount(&server)
            .await;

        let client = make_client(&server.uri());
        let resp = client.chat("sys", &[], &[]).await.unwrap();

        let LlmResponse::ToolCalls(items, _meta) = resp else {
            panic!("expected ToolCalls, got {resp:?}");
        };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "my-tool");
        assert_eq!(items[0].params["key"], "val");
    }

    #[tokio::test]
    async fn parses_multiple_tool_calls() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tool_calls_body(
                serde_json::json!([
                    { "function": { "name": "tool-a", "arguments": { "x": 1 } } },
                    { "function": { "name": "tool-b", "arguments": { "y": 2 } } }
                ]),
            )))
            .mount(&server)
            .await;

        let client = make_client(&server.uri());
        let resp = client.chat("sys", &[], &[]).await.unwrap();

        let LlmResponse::ToolCalls(items, _meta) = resp else {
            panic!("expected ToolCalls, got {resp:?}");
        };
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "tool-a");
        assert_eq!(items[0].params["x"], 1);
        assert_eq!(items[1].name, "tool-b");
        assert_eq!(items[1].params["y"], 2);
    }

    #[tokio::test]
    async fn empty_tool_calls_falls_back_to_final_answer() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(answer_body("hello!")))
            .mount(&server)
            .await;

        let client = make_client(&server.uri());
        let resp = client.chat("sys", &[], &[]).await.unwrap();

        let LlmResponse::FinalAnswer(text, _meta) = resp else {
            panic!("expected FinalAnswer, got {resp:?}");
        };
        assert_eq!(text, "hello!");
    }

    #[tokio::test]
    async fn skips_tool_call_entries_with_empty_name() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tool_calls_body(
                serde_json::json!([
                    { "function": { "name": "", "arguments": {} } },
                    { "function": { "name": "good-tool", "arguments": {} } }
                ]),
            )))
            .mount(&server)
            .await;

        let client = make_client(&server.uri());
        let resp = client.chat("sys", &[], &[]).await.unwrap();

        let LlmResponse::ToolCalls(items, _meta) = resp else {
            panic!("expected ToolCalls, got {resp:?}");
        };
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "good-tool");
    }

    // ── build_json_messages tests (MultimodalUser) ───────────────────────────

    #[test]
    fn multimodal_user_text_only_produces_plain_user_message() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![ContentBlock::Text("hello".to_string())],
        }];
        let msgs = build_json_messages("", &history, true);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], "hello");
        assert!(msgs[0].get("images").is_none(), "no images field expected");
    }

    #[test]
    fn multimodal_user_with_image_emits_images_field() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("describe this".to_string()),
                ContentBlock::Image {
                    media_type: "image/png".to_string(),
                    data: "iVBORw0KGgo=".to_string(),
                },
            ],
        }];
        let msgs = build_json_messages("", &history, true);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], "describe this");
        let images = msgs[0]["images"].as_array().expect("images field");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0], "iVBORw0KGgo=");
    }

    #[test]
    fn multimodal_user_multiple_images() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("compare".to_string()),
                ContentBlock::Image {
                    media_type: "image/png".to_string(),
                    data: "img1data".to_string(),
                },
                ContentBlock::Image {
                    media_type: "image/jpeg".to_string(),
                    data: "img2data".to_string(),
                },
            ],
        }];
        let msgs = build_json_messages("", &history, true);
        let images = msgs[0]["images"].as_array().expect("images field");
        assert_eq!(images.len(), 2);
        assert_eq!(images[0], "img1data");
        assert_eq!(images[1], "img2data");
    }

    #[test]
    fn multimodal_user_image_only_has_empty_content() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![ContentBlock::Image {
                media_type: "image/png".to_string(),
                data: "abc123".to_string(),
            }],
        }];
        let msgs = build_json_messages("", &history, true);
        assert_eq!(msgs[0]["content"], "");
        let images = msgs[0]["images"].as_array().expect("images field");
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn multimodal_user_vision_false_strips_images() {
        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("describe this".to_string()),
                ContentBlock::Image {
                    media_type: "image/png".to_string(),
                    data: "iVBORw0KGgo=".to_string(),
                },
            ],
        }];
        let msgs = build_json_messages("", &history, false);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], "describe this");
        assert!(
            msgs[0].get("images").is_none(),
            "images must be stripped when vision is false"
        );
    }
}
