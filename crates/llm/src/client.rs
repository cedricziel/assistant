use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use assistant_core::{types::ToolCallMode, SkillDef};
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage, MessageRole},
    Ollama,
};
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::{
    prompts::build_system_prompt,
    react::{ReActParser, ReActStep},
};

// ── Public types ──────────────────────────────────────────────────────────────

/// A single message in the chat history as tracked by the caller.
///
/// This is the crate's own message type so callers are not required to depend
/// directly on `ollama_rs` internals.
#[derive(Debug, Clone)]
pub struct ChatHistoryMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Chat participant role (mirrors `ollama_rs::MessageRole` without leaking it).
#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// The outcome of a single `LlmClient::chat` invocation.
#[derive(Debug, Clone)]
pub enum LlmResponse {
    /// The model wants to call a skill.
    ToolCall {
        name: String,
        params: serde_json::Value,
    },
    /// The model has a definitive answer for the user.
    FinalAnswer(String),
    /// The model emitted only a reasoning step (no action yet).
    Thinking(String),
}

/// Configuration for the LLM client.
#[derive(Debug, Clone)]
pub struct LlmClientConfig {
    pub model: String,
    pub base_url: String,
    pub tool_call_mode: ToolCallMode,
    pub timeout_secs: u64,
}

impl Default for LlmClientConfig {
    fn default() -> Self {
        Self {
            model: "qwen2.5:7b".to_string(),
            base_url: "http://localhost:11434".to_string(),
            tool_call_mode: ToolCallMode::Auto,
            timeout_secs: 120,
        }
    }
}

impl From<&assistant_core::LlmConfig> for LlmClientConfig {
    fn from(cfg: &assistant_core::LlmConfig) -> Self {
        Self {
            model: cfg.model.clone(),
            base_url: cfg.base_url.clone(),
            tool_call_mode: cfg.tool_call_mode.clone(),
            timeout_secs: cfg.timeout_secs,
        }
    }
}

// ── LlmClient ────────────────────────────────────────────────────────────────

/// High-level LLM client with automatic tool-calling strategy selection.
///
/// Supports two invocation paths:
///
/// 1. **Native Ollama tool-calling** (preferred when the model supports it):
///    Sends the request with a `tools` array (built from skill definitions) to
///    the Ollama `/api/chat` endpoint via `reqwest`, and parses `tool_calls`
///    from the JSON response.
///
/// 2. **ReAct text fallback**: Injects skill descriptions into the system
///    prompt, sends a plain chat request via `ollama_rs`, and parses the
///    `THOUGHT:`/`ACTION:`/`ANSWER:` output with [`ReActParser`].
///
/// When `tool_call_mode` is [`ToolCallMode::Auto`], the client tries native
/// tool-calling first. If the response contains no `tool_calls` but the text
/// contains ReAct markers, it records that the model does not support native
/// tool-calling and switches to ReAct for all subsequent calls.
pub struct LlmClient {
    config: LlmClientConfig,
    ollama: Ollama,
    /// Shared reqwest client used for the native-tool-call path.
    http: reqwest::Client,
    /// `None` = not yet determined, `Some(true)` = native OK, `Some(false)` = use ReAct.
    tool_call_capable: Arc<Mutex<Option<bool>>>,
}

impl LlmClient {
    /// Create a new client from the given configuration.
    pub fn new(config: LlmClientConfig) -> anyhow::Result<Self> {
        let base_url = config.base_url.trim_end_matches('/');

        // Derive host + port for the ollama_rs client.
        let parsed = format!("{}/", base_url)
            .parse::<url::Url>()
            .with_context(|| format!("invalid Ollama base_url: {}", config.base_url))?;

        let scheme = parsed.scheme();
        let host = parsed.host_str().unwrap_or("localhost");
        let port = parsed.port().unwrap_or(11434);
        let host_with_scheme = format!("{scheme}://{host}");

        let ollama = Ollama::new(host_with_scheme, port);

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .context("failed to build reqwest client")?;

        Ok(Self {
            config,
            ollama,
            http,
            tool_call_capable: Arc::new(Mutex::new(None)),
        })
    }

    /// Create a client directly from a `LlmConfig` (convenience wrapper).
    pub fn from_llm_config(cfg: &assistant_core::LlmConfig) -> anyhow::Result<Self> {
        Self::new(LlmClientConfig::from(cfg))
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Send a chat turn and return the model's response.
    ///
    /// # Parameters
    /// * `system_prompt` – base system instructions (extended in ReAct mode)
    /// * `history` – previous messages in the conversation
    /// * `skills` – skills available for this turn
    pub async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
    ) -> anyhow::Result<LlmResponse> {
        match self.effective_mode() {
            ToolCallMode::Native => self.chat_native(system_prompt, history, skills).await,
            ToolCallMode::React => self.chat_react(system_prompt, history, skills).await,
            ToolCallMode::Auto => self.chat_auto(system_prompt, history, skills).await,
        }
    }

    /// Send a chat turn, streaming content tokens to `token_tx` as they arrive.
    ///
    /// For native tool-call mode: content tokens are sent as each chunk arrives
    /// from the Ollama streaming endpoint. Tool-call responses typically have
    /// empty content, so no tokens are forwarded for those turns.
    ///
    /// For ReAct mode: the full response is accumulated, then parsed. If the
    /// result is a `FinalAnswer`, the answer text is sent as a single token.
    ///
    /// Returns the parsed [`LlmResponse`] once the model is done.
    pub async fn chat_stream(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
        token_tx: &tokio::sync::mpsc::UnboundedSender<String>,
    ) -> anyhow::Result<LlmResponse> {
        match self.effective_mode() {
            ToolCallMode::Native => {
                self.chat_native_stream(system_prompt, history, skills, token_tx)
                    .await
            }
            ToolCallMode::React => {
                self.chat_react_stream(system_prompt, history, skills, token_tx)
                    .await
            }
            ToolCallMode::Auto => {
                self.chat_auto_stream(system_prompt, history, skills, token_tx)
                    .await
            }
        }
    }

    // ── Mode resolution ───────────────────────────────────────────────────────

    fn effective_mode(&self) -> ToolCallMode {
        match self.config.tool_call_mode {
            ToolCallMode::Auto => match *self.tool_call_capable.lock().unwrap() {
                Some(true) => ToolCallMode::Native,
                Some(false) => ToolCallMode::React,
                None => ToolCallMode::Auto,
            },
            ref m => m.clone(),
        }
    }

    fn set_tool_call_capable(&self, capable: bool) {
        *self.tool_call_capable.lock().unwrap() = Some(capable);
    }

    // ── Auto mode ─────────────────────────────────────────────────────────────

    async fn chat_auto(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
    ) -> anyhow::Result<LlmResponse> {
        debug!("Auto mode: trying native tool-calling first");

        match self.chat_native(system_prompt, history, skills).await {
            Ok(response @ LlmResponse::ToolCall { .. }) => {
                self.set_tool_call_capable(true);
                debug!("Auto mode: model supports native tool-calling");
                Ok(response)
            }
            Ok(LlmResponse::FinalAnswer(text)) if ReActParser::looks_like_react(&text) => {
                warn!(
                    "Auto mode: native call returned no tool_calls but response looks like ReAct \
                     — switching to ReAct for this session"
                );
                self.set_tool_call_capable(false);
                // Re-parse the text we already have via the ReAct parser.
                Ok(react_step_to_response(ReActParser::parse(&text)))
            }
            Ok(response) => {
                // Clean final answer; assume native mode is fine.
                self.set_tool_call_capable(true);
                Ok(response)
            }
            Err(err) => {
                warn!(%err, "Auto mode: native tool-calling failed, falling back to ReAct");
                self.set_tool_call_capable(false);
                self.chat_react(system_prompt, history, skills).await
            }
        }
    }

    // ── Native tool-calling (via reqwest) ────────────────────────────────────

    /// Send a native Ollama tool-call request by constructing the JSON body
    /// directly with `reqwest`.
    ///
    /// `ollama_rs::ToolInfo` uses private fields and type-level generics that
    /// make it impractical for dynamically-discovered skills; we therefore
    /// bypass the ollama-rs abstractions here and speak to the REST API
    /// directly.
    async fn chat_native(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
    ) -> anyhow::Result<LlmResponse> {
        debug!(
            model = %self.config.model,
            skills = skills.len(),
            "Sending native tool-call request to Ollama"
        );

        let messages = build_json_messages(system_prompt, history);
        let tools: Vec<Value> = skills.iter().map(|s| skill_to_tool_json(s)).collect();

        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "tools": tools,
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

        // Check for tool_calls in the response message.
        if let Some(tool_calls) = json
            .pointer("/message/tool_calls")
            .and_then(|v| v.as_array())
        {
            if let Some(first) = tool_calls.first() {
                let name = first
                    .pointer("/function/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let params = first
                    .pointer("/function/arguments")
                    .cloned()
                    .unwrap_or(Value::Object(serde_json::Map::new()));

                if !name.is_empty() {
                    debug!(skill = %name, "Native tool call received");
                    return Ok(LlmResponse::ToolCall { name, params });
                }
            }
        }

        // No tool calls — treat the content as a final answer.
        let content = json
            .pointer("/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        debug!("Native request returned no tool_calls; treating as final answer");
        Ok(LlmResponse::FinalAnswer(content))
    }

    // ── ReAct fallback (via ollama-rs) ────────────────────────────────────────

    async fn chat_react(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
    ) -> anyhow::Result<LlmResponse> {
        debug!(
            model = %self.config.model,
            "Sending ReAct text request to Ollama"
        );

        // Build a combined system prompt that includes the ReAct skill listing.
        let react_system = if system_prompt.is_empty() {
            build_system_prompt(skills)
        } else {
            format!("{}\n\n{}", system_prompt, build_system_prompt(skills))
        };

        let messages = build_ollama_messages(&react_system, history);
        let request = ChatMessageRequest::new(self.config.model.clone(), messages);

        let response = self
            .ollama
            .send_chat_messages(request)
            .await
            .map_err(|e| anyhow::anyhow!("Ollama chat request failed: {e}"))?;

        let raw_text = response.message.content;
        let step = ReActParser::parse(&raw_text);
        Ok(react_step_to_response(step))
    }

    // ── Streaming native (via reqwest) ───────────────────────────────────────

    /// Streaming variant of [`chat_native`].  Content tokens are forwarded to
    /// `token_tx` as each NDJSON chunk arrives.  Tool-call responses typically
    /// carry empty content, so nothing will be forwarded during tool turns.
    async fn chat_native_stream(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
        token_tx: &tokio::sync::mpsc::UnboundedSender<String>,
    ) -> anyhow::Result<LlmResponse> {
        debug!(
            model = %self.config.model,
            skills = skills.len(),
            "Sending native streaming request to Ollama"
        );

        let messages = build_json_messages(system_prompt, history);
        let tools: Vec<Value> = skills.iter().map(|s| skill_to_tool_json(s)).collect();

        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "tools": tools,
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

        let mut content_buf = String::new();
        let mut tool_calls_buf: Vec<Value> = Vec::new();
        let mut line_buf = String::new();
        let mut resp = resp;

        while let Some(chunk) = resp.chunk().await? {
            let chunk_str = String::from_utf8_lossy(&chunk);
            line_buf.push_str(&chunk_str);

            // Process all complete newline-delimited JSON lines.
            while let Some(pos) = line_buf.find('\n') {
                let line = line_buf[..pos].trim().to_string();
                line_buf = line_buf[pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Ok(json) = serde_json::from_str::<Value>(&line) {
                    // Forward content tokens immediately.
                    if let Some(content) = json.pointer("/message/content").and_then(|v| v.as_str())
                    {
                        if !content.is_empty() {
                            content_buf.push_str(content);
                            let _ = token_tx.send(content.to_string());
                        }
                    }
                    // Accumulate tool calls (appear near end of stream).
                    if let Some(tcs) = json
                        .pointer("/message/tool_calls")
                        .and_then(|v| v.as_array())
                    {
                        tool_calls_buf.extend(tcs.iter().cloned());
                    }
                }
            }
        }

        // Determine final response type from accumulated data.
        if let Some(first) = tool_calls_buf.first() {
            let name = first
                .pointer("/function/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let params = first
                .pointer("/function/arguments")
                .cloned()
                .unwrap_or(Value::Object(serde_json::Map::new()));

            if !name.is_empty() {
                debug!(skill = %name, "Native streaming: tool call received");
                return Ok(LlmResponse::ToolCall { name, params });
            }
        }

        debug!("Native streaming: final answer received");
        Ok(LlmResponse::FinalAnswer(content_buf))
    }

    // ── Streaming ReAct (via ollama-rs) ──────────────────────────────────────

    /// Streaming variant of [`chat_react`].  The full text is accumulated from
    /// the stream before parsing.  If the result is a `FinalAnswer`, the answer
    /// text is forwarded to `token_tx` as a single chunk.  Raw ReAct-format
    /// content (THOUGHT/ACTION markers) is not forwarded to avoid confusing UI.
    async fn chat_react_stream(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
        token_tx: &tokio::sync::mpsc::UnboundedSender<String>,
    ) -> anyhow::Result<LlmResponse> {
        use tokio_stream::StreamExt as _;

        debug!(
            model = %self.config.model,
            "Sending ReAct streaming request to Ollama"
        );

        let react_system = if system_prompt.is_empty() {
            build_system_prompt(skills)
        } else {
            format!("{}\n\n{}", system_prompt, build_system_prompt(skills))
        };

        let messages = build_ollama_messages(&react_system, history);
        let request = ChatMessageRequest::new(self.config.model.clone(), messages);

        let mut stream = self
            .ollama
            .send_chat_messages_stream(request)
            .await
            .map_err(|e| anyhow::anyhow!("Ollama streaming chat request failed: {e}"))?;

        let mut raw_text = String::new();
        while let Some(Ok(res)) = stream.next().await {
            raw_text.push_str(&res.message.content);
        }

        let step = ReActParser::parse(&raw_text);

        // Only forward the clean answer text, not raw ReAct markers.
        if let ReActStep::Answer(ref text) = step {
            let _ = token_tx.send(text.clone());
        }

        Ok(react_step_to_response(step))
    }

    // ── Streaming auto mode ───────────────────────────────────────────────────

    async fn chat_auto_stream(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
        token_tx: &tokio::sync::mpsc::UnboundedSender<String>,
    ) -> anyhow::Result<LlmResponse> {
        debug!("Auto streaming: trying native tool-calling first");

        match self
            .chat_native_stream(system_prompt, history, skills, token_tx)
            .await
        {
            Ok(response @ LlmResponse::ToolCall { .. }) => {
                self.set_tool_call_capable(true);
                debug!("Auto streaming: model supports native tool-calling");
                Ok(response)
            }
            Ok(LlmResponse::FinalAnswer(text)) if ReActParser::looks_like_react(&text) => {
                warn!(
                    "Auto streaming: native response looks like ReAct \
                     — switching to ReAct for this session"
                );
                self.set_tool_call_capable(false);
                // Re-parse without a second LLM call; tokens already forwarded.
                Ok(react_step_to_response(ReActParser::parse(&text)))
            }
            Ok(response) => {
                self.set_tool_call_capable(true);
                Ok(response)
            }
            Err(err) => {
                warn!(%err, "Auto streaming: native failed, falling back to ReAct");
                self.set_tool_call_capable(false);
                self.chat_react_stream(system_prompt, history, skills, token_tx)
                    .await
            }
        }
    }
}

// ── Free helpers ──────────────────────────────────────────────────────────────

/// Convert a [`SkillDef`] to the JSON structure expected by the Ollama
/// `tools` array in the `/api/chat` request body.
fn skill_to_tool_json(skill: &SkillDef) -> Value {
    let parameters = skill.params_schema().unwrap_or_else(|| {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    });

    json!({
        "type": "function",
        "function": {
            "name": skill.name,
            "description": skill.description,
            "parameters": parameters,
        }
    })
}

/// Build the JSON messages array for the native (reqwest) path.
fn build_json_messages(system_prompt: &str, history: &[ChatHistoryMessage]) -> Vec<Value> {
    let mut messages = Vec::with_capacity(history.len() + 1);

    if !system_prompt.is_empty() {
        messages.push(json!({ "role": "system", "content": system_prompt }));
    }

    for msg in history {
        let role = match msg.role {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::Tool => "tool",
        };
        messages.push(json!({ "role": role, "content": msg.content }));
    }

    messages
}

/// Build the `ollama_rs` `ChatMessage` list for the ReAct (ollama-rs) path.
fn build_ollama_messages(system_prompt: &str, history: &[ChatHistoryMessage]) -> Vec<ChatMessage> {
    let mut messages = Vec::with_capacity(history.len() + 1);

    if !system_prompt.is_empty() {
        messages.push(ChatMessage::system(system_prompt.to_string()));
    }

    for msg in history {
        let role = match msg.role {
            ChatRole::System => MessageRole::System,
            ChatRole::User => MessageRole::User,
            ChatRole::Assistant => MessageRole::Assistant,
            ChatRole::Tool => MessageRole::Tool,
        };
        messages.push(ChatMessage::new(role, msg.content.clone()));
    }

    messages
}

/// Convert a parsed [`ReActStep`] into an [`LlmResponse`].
fn react_step_to_response(step: ReActStep) -> LlmResponse {
    match step {
        ReActStep::ToolCall { name, params } => LlmResponse::ToolCall { name, params },
        ReActStep::Answer(text) => LlmResponse::FinalAnswer(text),
        ReActStep::Thought(text) => LlmResponse::Thinking(text),
    }
}
