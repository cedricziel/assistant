//! `MoonshotProvider` — thin facade over [`OpenAIProvider`] for the Moonshot AI
//! (Kimi) chat completions API.
//!
//! Moonshot exposes an OpenAI-compatible `/v1/chat/completions` endpoint, so we
//! delegate all heavy lifting to the existing OpenAI provider and only override
//! construction defaults and OTel metadata.
//!
//! When the `$web_search` builtin is enabled the provider handles the
//! Moonshot-specific echo-back loop internally: it injects a
//! `builtin_function` tool spec, intercepts `$web_search` tool calls, echoes
//! the arguments back to the API, and returns the final answer to the caller.

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use assistant_core::LlmConfig;
use assistant_llm::{
    Capabilities, ChatHistoryMessage, ChatRole, ContentBlock, HostedTool, LlmProvider, LlmResponse,
    LlmResponseMeta, ToolCallItem, ToolSpec,
};
use assistant_provider_openai::{OpenAIProvider, OpenAIProviderConfig};

// ── Defaults ──────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL: &str = "https://api.moonshot.ai/v1";
const DEFAULT_MAX_TOKENS: u32 = 8192;

/// Upper bound on the number of `$web_search` echo-back rounds to prevent
/// infinite loops if the API never returns `finish_reason: "stop"`.
const MAX_WEB_SEARCH_ROUNDS: usize = 5;

// ── MoonshotProvider ──────────────────────────────────────────────────────────

/// [`LlmProvider`] backed by the Moonshot AI (Kimi) chat completions API.
///
/// Internally delegates to [`OpenAIProvider`] since the wire protocol is
/// identical.  Provides Moonshot-specific defaults for base URL, model, and
/// API-key resolution (`MOONSHOT_API_KEY` env var).
///
/// When `web_search` is enabled, chat requests are handled via raw HTTP so the
/// provider can inject the `builtin_function` tool type and perform the
/// echo-back loop that Moonshot's `$web_search` requires.
pub struct MoonshotProvider {
    inner: OpenAIProvider,
    /// Kept separately so `server_address()` returns the Moonshot URL, not
    /// whatever the inner provider normalised.
    base_url: String,
    model: String,
    /// API key — needed for the raw-HTTP path when web search is enabled.
    api_key: String,
    max_tokens: u32,
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
        let openai_cfg = OpenAIProviderConfig {
            model: model.clone(),
            base_url: base_url.clone(),
            timeout_secs,
            max_tokens,
            embedding_model: String::new(), // Moonshot has no embedding endpoint
            web_search: None,               // handled by our own raw-HTTP path
        };

        let inner = OpenAIProvider::new(openai_cfg, api_key)?;

        let http_client = assistant_llm::build_http_client(timeout_secs)?;

        debug!(
            model = %model,
            base_url = %base_url,
            web_search = web_search_enabled,
            "Moonshot provider initialised (delegating to OpenAI provider)"
        );

        Ok(Self {
            inner,
            base_url,
            model,
            api_key: api_key.to_string(),
            max_tokens,
            web_search_enabled,
            http_client,
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
        let mut messages = build_moonshot_messages(system_prompt, history);

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
                    // Separate $web_search calls from regular tool calls.
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

                    // If there are regular (non-web-search) tool calls, return
                    // them to the orchestrator immediately.
                    if !regular_calls.is_empty() {
                        debug!(
                            count = regular_calls.len(),
                            "Moonshot: regular tool calls received alongside web search"
                        );
                        return Ok(LlmResponse::ToolCalls(regular_calls, meta));
                    }

                    // Echo $web_search calls back and continue the loop.
                    if !web_search_calls.is_empty() {
                        // Append the assistant message (with tool_calls).
                        messages.push(message.clone());

                        // Echo each $web_search call's arguments as tool result.
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

                        continue; // next round
                    }
                }
            }

            // Final answer (finish_reason == "stop" or anything else).
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
        let mut caps = self.inner.capabilities();
        if self.web_search_enabled {
            caps.hosted_tools.push(HostedTool::WebSearch);
        }
        caps
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
            self.inner.chat(system_prompt, history, tools).await
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
            // fall back to non-streaming.  The final answer tokens are still
            // forwarded through the sink if provided.
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
            self.inner
                .chat_streaming(system_prompt, history, tools, token_sink)
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

// ── Message conversion helpers ────────────────────────────────────────────────

/// Build a JSON messages array for the Moonshot API from our conversation
/// history.  Uses the same structure as OpenAI Chat Completions.
fn build_moonshot_messages(system_prompt: &str, history: &[ChatHistoryMessage]) -> Vec<Value> {
    let mut messages: Vec<Value> = Vec::with_capacity(history.len() + 1);
    let mut pending_ids: Vec<(String, String)> = Vec::new();

    if !system_prompt.is_empty() {
        messages.push(json!({
            "role": "system",
            "content": system_prompt,
        }));
    }

    for entry in history {
        match entry {
            ChatHistoryMessage::Text { role, content } => {
                let role_str = match role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "user", // fallback
                };
                messages.push(json!({
                    "role": role_str,
                    "content": content,
                }));
            }

            ChatHistoryMessage::MultimodalUser { content } => {
                let parts: Vec<Value> = content
                    .iter()
                    .map(|block| match block {
                        ContentBlock::Text(text) => {
                            json!({"type": "text", "text": text})
                        }
                        ContentBlock::Image { media_type, data } => {
                            let data_uri = format!("data:{media_type};base64,{data}");
                            json!({
                                "type": "image_url",
                                "image_url": { "url": data_uri }
                            })
                        }
                    })
                    .collect();
                messages.push(json!({
                    "role": "user",
                    "content": parts,
                }));
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
                messages.push(json!({
                    "role": "assistant",
                    "content": null,
                    "tool_calls": tool_calls,
                }));
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

    // ── Message builder tests ─────────────────────────────────────────────

    #[test]
    fn build_moonshot_messages_system_prompt() {
        let msgs = build_moonshot_messages("You are helpful.", &[]);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[0]["content"], "You are helpful.");
    }

    #[test]
    fn build_moonshot_messages_text_exchange() {
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
        let msgs = build_moonshot_messages("sys", &history);
        // system + user + assistant
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[2]["role"], "assistant");
    }

    #[test]
    fn build_moonshot_messages_tool_calls_and_results() {
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
        let msgs = build_moonshot_messages("", &history);
        // assistant (tool_calls) + tool (result)  (no system because empty prompt)
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "assistant");
        assert!(msgs[0]["tool_calls"].is_array());
        assert_eq!(msgs[1]["role"], "tool");
        assert_eq!(msgs[1]["tool_call_id"], "call_123");
    }

    // ── Capabilities tests ────────────────────────────────────────────────

    #[test]
    fn capabilities_with_web_search_enabled() {
        let provider = MoonshotProvider::new(
            "kimi-k2.5".to_string(),
            DEFAULT_BASE_URL.to_string(),
            "test-key",
            120,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .expect("should build");
        let caps = provider.capabilities();
        assert!(
            caps.hosted_tools.contains(&HostedTool::WebSearch),
            "should report WebSearch"
        );
    }

    #[test]
    fn capabilities_without_web_search() {
        let provider = MoonshotProvider::new(
            "kimi-k2.5".to_string(),
            DEFAULT_BASE_URL.to_string(),
            "test-key",
            120,
            DEFAULT_MAX_TOKENS,
            false,
        )
        .expect("should build");
        let caps = provider.capabilities();
        assert!(
            !caps.hosted_tools.contains(&HostedTool::WebSearch),
            "should not report WebSearch"
        );
    }

    // ── Web-search echo-back loop tests (wiremock) ────────────────────────

    /// Helper: build a Moonshot API response with a `$web_search` tool call.
    fn web_search_tool_call_response(call_id: &str, query: &str) -> Value {
        json!({
            "id": "resp_001",
            "model": "kimi-k2.5",
            "choices": [{
                "index": 0,
                "finish_reason": "tool_calls",
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{
                        "id": call_id,
                        "type": "function",
                        "function": {
                            "name": "$web_search",
                            "arguments": json!({"query": query}).to_string()
                        }
                    }]
                }
            }],
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 10,
            }
        })
    }

    /// Helper: build a Moonshot API final-answer response.
    fn final_answer_response(content: &str) -> Value {
        json!({
            "id": "resp_002",
            "model": "kimi-k2.5",
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": content,
                }
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 30,
            }
        })
    }

    #[tokio::test]
    async fn web_search_echo_back_loop() {
        let mock_server = MockServer::start().await;

        // Round 1: model requests $web_search.
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(web_search_tool_call_response("call_ws_1", "latest AI news")),
            )
            .up_to_n_times(1)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Round 2: after echo-back, model returns final answer.
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(final_answer_response("Here are the latest AI news...")),
            )
            .up_to_n_times(1)
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = MoonshotProvider::new(
            "kimi-k2.5".to_string(),
            mock_server.uri(),
            "test-key",
            30,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .expect("should build");

        let result = provider
            .chat("You are helpful.", &[], &[])
            .await
            .expect("chat should succeed");

        match result {
            LlmResponse::FinalAnswer(text, meta) => {
                assert_eq!(text, "Here are the latest AI news...");
                assert_eq!(meta.model.as_deref(), Some("kimi-k2.5"));
            }
            other => panic!("expected FinalAnswer, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn web_search_regular_tool_calls_returned_to_caller() {
        let mock_server = MockServer::start().await;

        // Model returns a regular (non-$web_search) tool call.
        let response = json!({
            "id": "resp_003",
            "model": "kimi-k2.5",
            "choices": [{
                "index": 0,
                "finish_reason": "tool_calls",
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{
                        "id": "call_regular",
                        "type": "function",
                        "function": {
                            "name": "file-read",
                            "arguments": "{\"path\": \"/tmp/foo.txt\"}"
                        }
                    }]
                }
            }],
            "usage": { "prompt_tokens": 50, "completion_tokens": 10 }
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = MoonshotProvider::new(
            "kimi-k2.5".to_string(),
            mock_server.uri(),
            "test-key",
            30,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .expect("should build");

        let tool_spec = ToolSpec {
            name: "file-read".to_string(),
            description: "Read a file".to_string(),
            params_schema: json!({"type": "object", "properties": {"path": {"type": "string"}}}),
            is_mutating: false,
            requires_confirmation: false,
        };

        let result = provider
            .chat("You are helpful.", &[], &[tool_spec])
            .await
            .expect("chat should succeed");

        match result {
            LlmResponse::ToolCalls(calls, _) => {
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].name, "file-read");
                assert_eq!(calls[0].params["path"], "/tmp/foo.txt");
            }
            other => panic!("expected ToolCalls, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn web_search_direct_answer_no_search() {
        let mock_server = MockServer::start().await;

        // Model answers directly without searching.
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(final_answer_response("2+2 is 4")),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = MoonshotProvider::new(
            "kimi-k2.5".to_string(),
            mock_server.uri(),
            "test-key",
            30,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .expect("should build");

        let result = provider
            .chat("You are helpful.", &[], &[])
            .await
            .expect("chat should succeed");

        match result {
            LlmResponse::FinalAnswer(text, _) => {
                assert_eq!(text, "2+2 is 4");
            }
            other => panic!("expected FinalAnswer, got {other:?}"),
        }
    }

    // ── Live integration tests (require MOONSHOT_API_KEY) ─────────────────

    #[tokio::test]
    #[ignore = "requires MOONSHOT_API_KEY"]
    async fn live_web_search() {
        let api_key = std::env::var("MOONSHOT_API_KEY").expect("MOONSHOT_API_KEY must be set");

        let provider = MoonshotProvider::new(
            "kimi-k2.5".to_string(),
            DEFAULT_BASE_URL.to_string(),
            &api_key,
            60,
            DEFAULT_MAX_TOKENS,
            true,
        )
        .expect("should build");

        let history = vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "What is today's date? Use web search to confirm.".to_string(),
        }];

        let result = provider
            .chat("You are a helpful assistant. Be concise.", &history, &[])
            .await
            .expect("live chat should succeed");

        match result {
            LlmResponse::FinalAnswer(text, meta) => {
                eprintln!("--- Live $web_search response ---");
                eprintln!("Model: {:?}", meta.model);
                eprintln!(
                    "Tokens: in={:?} out={:?}",
                    meta.input_tokens, meta.output_tokens
                );
                eprintln!("Answer: {text}");
                eprintln!("---");
                assert!(!text.is_empty(), "answer should not be empty");
            }
            other => {
                panic!("expected FinalAnswer but got {other:?}");
            }
        }
    }
}
