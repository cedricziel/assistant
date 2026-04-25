//! `MoonshotProvider` — [`LlmProvider`] implementation for the Moonshot AI
//! (Kimi) chat completions API.
//!
//! Delegates standard Chat Completions logic to [`ChatCompletionsProvider`].
//! The `$web_search` builtin is handled via raw HTTP because it uses the
//! non-standard `"type": "builtin_function"` tool spec and requires a
//! multi-round echo-back loop.

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use assistant_core::{
    Capabilities, ChatHistoryMessage, HostedTool, LlmConfig, LlmProvider, LlmResponse,
    LlmResponseMeta, StreamChunk, ToolCallItem, ToolCallResponse, ToolSpec, ToolSupport,
};

use crate::chat_completions::{ChatCompletionsConfig, ChatCompletionsProvider, build_raw_messages};

// ── Defaults ─────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL: &str = "https://api.moonshot.ai/v1";
const DEFAULT_MAX_TOKENS: u32 = 8192;

/// Upper bound on the number of `$web_search` echo-back rounds to prevent
/// infinite loops if the API never returns `finish_reason: "stop"`.
const MAX_WEB_SEARCH_ROUNDS: usize = 5;

// ── MoonshotProvider ─────────────────────────────────────────────────────────

/// [`LlmProvider`] backed by the Moonshot AI (Kimi) chat completions API.
///
/// Delegates standard chat / streaming to [`ChatCompletionsProvider`].
/// When `web_search` is enabled, requests go through raw HTTP instead,
/// because the `$web_search` tool uses the non-standard
/// `"type": "builtin_function"` and requires a multi-round echo-back loop.
pub struct MoonshotProvider {
    /// Shared Chat Completions client (handles standard chat + streaming).
    inner: ChatCompletionsProvider,
    web_search_enabled: bool,
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
        let config = ChatCompletionsConfig {
            model,
            base_url,
            api_key: api_key.to_string(),
            timeout_secs,
            max_tokens,
            extra_headers: vec![],
        };

        debug!(
            model = %config.model,
            base_url = %config.base_url,
            web_search = web_search_enabled,
            "Moonshot provider initialised"
        );

        Ok(Self {
            inner: ChatCompletionsProvider::new(config)?,
            web_search_enabled,
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

    // ── Raw-HTTP chat (web search path) ──────────────────────────────────

    /// Send a chat request via raw HTTP, injecting the `$web_search` builtin
    /// and handling the echo-back loop internally.
    async fn chat_with_web_search(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        let url = format!(
            "{}/chat/completions",
            self.inner.config.base_url.trim_end_matches('/')
        );

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
                "model": self.inner.config.model,
                "messages": messages,
                "tools": request_tools,
                "max_tokens": self.inner.config.max_tokens,
                "thinking": { "type": "disabled" },
            });

            let resp_body = self.inner.raw_http_post(&url, &body).await?;

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
            if finish_reason == "tool_calls"
                && let Some(tool_calls) = message["tool_calls"].as_array()
            {
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
                    return Ok(LlmResponse::ToolCalls(ToolCallResponse {
                        items: regular_calls,
                        meta,
                        thinking: None,
                    }));
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

            let content = message["content"].as_str().unwrap_or("").to_string();
            debug!("Moonshot web-search: final answer received");
            return Ok(LlmResponse::FinalAnswer(content, meta));
        }

        warn!("Moonshot web-search: max rounds ({MAX_WEB_SEARCH_ROUNDS}) exceeded");
        anyhow::bail!("Moonshot $web_search echo-back loop exceeded {MAX_WEB_SEARCH_ROUNDS} rounds")
    }
}

// ── LlmProvider ──────────────────────────────────────────────────────────────

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
            self.inner
                .chat_non_streaming(system_prompt, history, tools)
                .await
        }
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        if self.web_search_enabled {
            // $web_search echo-back loop is not compatible with SSE streaming;
            // fall back to non-streaming.
            let result = self
                .chat_with_web_search(system_prompt, history, tools)
                .await?;
            if let LlmResponse::FinalAnswer(ref text, _) = result
                && let Some(sink) = chunk_sink
            {
                let _ = sink.send(StreamChunk::Text(text.clone())).await;
            }
            Ok(result)
        } else {
            self.inner
                .chat_sse(system_prompt, history, tools, chunk_sink)
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
        &self.inner.config.model
    }

    fn server_address(&self) -> &str {
        &self.inner.config.base_url
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_core::ChatRole;

    #[test]
    fn default_constants_are_sensible() {
        assert_eq!(DEFAULT_BASE_URL, "https://api.moonshot.ai/v1");
        assert!(DEFAULT_MAX_TOKENS > 0);
    }

    // ── Capabilities ─────────────────────────────────────────────────────

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
        assert!(
            p.capabilities()
                .hosted_tools
                .contains(&HostedTool::WebSearch)
        );
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
        assert!(
            !p.capabilities()
                .hosted_tools
                .contains(&HostedTool::WebSearch)
        );
    }

    // ── Web-search echo-back tests (wiremock) ────────────────────────────

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
            LlmResponse::ToolCalls(resp) => {
                assert_eq!(resp.items.len(), 1);
                assert_eq!(resp.items[0].name, "file-read");
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
