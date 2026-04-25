//! `OpenRouterProvider` — [`LlmProvider`] implementation for the OpenRouter
//! unified API gateway.
//!
//! OpenRouter provides access to 300+ models from multiple providers through
//! a single OpenAI-compatible Chat Completions endpoint.  This provider
//! delegates all Chat Completions logic to [`ChatCompletionsProvider`] and
//! injects the required `HTTP-Referer` and `X-Title` headers.

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::{
    Capabilities, ChatHistoryMessage, LlmConfig, LlmProvider, LlmResponse, StreamChunk, ToolSpec,
    ToolSupport,
};

use crate::chat_completions::{ChatCompletionsConfig, ChatCompletionsProvider};

// ── Defaults ────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";
const DEFAULT_MAX_TOKENS: u32 = 8192;

// ── OpenRouterProvider ──────────────────────────────────────────────────────

/// [`LlmProvider`] backed by the OpenRouter Chat Completions API.
///
/// Delegates all chat / streaming to [`ChatCompletionsProvider`].
/// Injects `HTTP-Referer` and `X-Title` headers per OpenRouter requirements.
pub struct OpenRouterProvider {
    /// Shared Chat Completions client (handles standard chat + streaming).
    inner: ChatCompletionsProvider,
}

impl OpenRouterProvider {
    /// Create from the unified [`LlmConfig`].
    ///
    /// Resolves the API key from `config.api_key` or `OPENROUTER_API_KEY` env
    /// var.  Falls back to `DEFAULT_BASE_URL` when the base URL is the Ollama
    /// default (i.e. not explicitly configured).
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        let api_key = cfg
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "OpenRouter API key not found. Set api_key in [llm] config \
                     or OPENROUTER_API_KEY environment variable."
                )
            })?;

        // If the base URL is the Ollama default, the user hasn't explicitly set
        // it, so use the OpenRouter endpoint instead.
        let base_url = if cfg.base_url == "http://localhost:11434" {
            DEFAULT_BASE_URL.to_string()
        } else {
            cfg.base_url.clone()
        };

        let mut extra_headers = Vec::new();
        if let Some(ref referer) = cfg.openrouter.referer {
            extra_headers.push(("HTTP-Referer".to_string(), referer.clone()));
        }
        if let Some(ref title) = cfg.openrouter.title {
            extra_headers.push(("X-Title".to_string(), title.clone()));
        }

        let max_tokens = cfg.openrouter.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

        let config = ChatCompletionsConfig {
            model: cfg.model.clone(),
            base_url,
            api_key,
            timeout_secs: cfg.timeout_secs,
            max_tokens,
            extra_headers,
        };

        debug!(
            model = %config.model,
            base_url = %config.base_url,
            "Creating OpenRouter provider"
        );

        Ok(Self {
            inner: ChatCompletionsProvider::new(config)?,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: ToolSupport::Native,
            streaming: true,
            // Model-dependent, but safe to declare — the Chat Completions layer
            // handles multimodal content blocks regardless.
            vision: true,
            hosted_tools: vec![],
            context_window_tokens: None, // varies by model
        }
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        self.inner
            .chat_non_streaming(system_prompt, history, tools)
            .await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        self.inner
            .chat_sse(system_prompt, history, tools, chunk_sink)
            .await
    }

    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!(
            "OpenRouter does not support embeddings directly. \
             Configure a dedicated embedding provider under [llm.embeddings]."
        ))
    }

    fn provider_name(&self) -> &str {
        "openrouter"
    }

    fn model_name(&self) -> &str {
        &self.inner.config.model
    }

    fn server_address(&self) -> &str {
        &self.inner.config.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::OpenRouterOptions;
    use assistant_core::types::LlmProviderKind;
    use serde_json::{Value, json};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ── Config / construction tests ─────────────────────────────────────

    #[test]
    fn provider_name_is_openrouter() {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();
        assert_eq!(provider.provider_name(), "openrouter");
    }

    #[test]
    fn default_base_url_when_not_configured() {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();
        assert_eq!(provider.server_address(), DEFAULT_BASE_URL);
    }

    #[test]
    fn custom_base_url_preserved() {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some("test-key".to_string()),
            base_url: "https://custom.openrouter.example/v1".to_string(),
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();
        assert_eq!(
            provider.server_address(),
            "https://custom.openrouter.example/v1"
        );
    }

    #[test]
    fn missing_api_key_produces_clear_error() {
        // Clear env to ensure no fallback key
        // SAFETY: this test runs serially and no other thread reads this var concurrently.
        unsafe { std::env::remove_var("OPENROUTER_API_KEY") };
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: None,
            ..LlmConfig::default()
        };
        let result = OpenRouterProvider::from_llm_config(&cfg);
        assert!(result.is_err(), "should fail without API key");
        let err = result.err().unwrap();
        assert!(
            err.to_string().contains("API key"),
            "error should mention API key: {err}"
        );
    }

    #[test]
    fn capabilities_report_tools_and_streaming() {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();
        let caps = provider.capabilities();
        assert_eq!(caps.tools, ToolSupport::Native);
        assert!(caps.streaming);
        assert!(caps.vision);
    }

    #[test]
    fn openrouter_options_applied_to_provider() {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            api_key: Some("sk-or-test".to_string()),
            openrouter: OpenRouterOptions {
                referer: Some("https://example.com".to_string()),
                title: Some("Test App".to_string()),
                max_tokens: Some(4096),
            },
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();
        assert_eq!(provider.model_name(), "anthropic/claude-sonnet-4-20250514");
        assert_eq!(provider.inner.config.max_tokens, 4096);
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    fn chat_response(content: &str) -> Value {
        json!({
            "id": "gen-abc123",
            "object": "chat.completion",
            "created": 1700000000_u64,
            "model": "anthropic/claude-sonnet-4-20250514",
            "choices": [{"index": 0, "finish_reason": "stop", "message": {
                "role": "assistant",
                "content": content
            }}],
            "usage": {"prompt_tokens": 20, "completion_tokens": 10, "total_tokens": 30}
        })
    }

    fn tool_call_response(tool_name: &str, args: &Value) -> Value {
        json!({
            "id": "gen-def456",
            "object": "chat.completion",
            "created": 1700000000_u64,
            "model": "anthropic/claude-sonnet-4-20250514",
            "choices": [{"index": 0, "finish_reason": "tool_calls", "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [{"id": "call_001", "type": "function", "function": {
                    "name": tool_name,
                    "arguments": serde_json::to_string(args).unwrap()
                }}]
            }}],
            "usage": {"prompt_tokens": 30, "completion_tokens": 15, "total_tokens": 45}
        })
    }

    fn make_provider(server_uri: &str) -> OpenRouterProvider {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some("test-key".to_string()),
            base_url: server_uri.to_string(),
            openrouter: OpenRouterOptions {
                referer: Some("https://my-app.example.com".to_string()),
                title: Some("My Test App".to_string()),
                max_tokens: Some(4096),
            },
            ..LlmConfig::default()
        };
        OpenRouterProvider::from_llm_config(&cfg).unwrap()
    }

    // ── Wiremock: non-streaming chat ────────────────────────────────────

    #[tokio::test]
    async fn non_streaming_chat_round_trip() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(chat_response("Hello from OpenRouter!")),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider = make_provider(&server.uri());

        match provider.chat("You are helpful.", &[], &[]).await.unwrap() {
            LlmResponse::FinalAnswer(text, meta) => {
                assert_eq!(text, "Hello from OpenRouter!");
                assert_eq!(
                    meta.model.as_deref(),
                    Some("anthropic/claude-sonnet-4-20250514")
                );
                assert_eq!(meta.input_tokens, Some(20));
                assert_eq!(meta.output_tokens, Some(10));
            }
            other => panic!("expected FinalAnswer, got {other:?}"),
        }
    }

    // ── Wiremock: custom headers ────────────────────────────────────────

    #[tokio::test]
    async fn custom_headers_sent_with_request() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("http-referer", "https://my-app.example.com"))
            .and(header("x-title", "My Test App"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(chat_response("OK")))
            .expect(1)
            .mount(&server)
            .await;

        let provider = make_provider(&server.uri());
        let result = provider.chat("sys", &[], &[]).await;
        assert!(result.is_ok(), "request with custom headers should succeed");
    }

    // ── Wiremock: tool calling ──────────────────────────────────────────

    #[tokio::test]
    async fn tool_call_request_and_response() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response(
                "file-read",
                &json!({"path": "/tmp/test"}),
            )))
            .expect(1)
            .mount(&server)
            .await;

        let provider = make_provider(&server.uri());

        let spec = ToolSpec {
            name: "file-read".into(),
            description: "Read a file".into(),
            params_schema: json!({"type": "object", "properties": {"path": {"type": "string"}}}),
            is_mutating: false,
            requires_confirmation: false,
        };

        match provider.chat("sys", &[], &[spec]).await.unwrap() {
            LlmResponse::ToolCalls(resp) => {
                assert_eq!(resp.items.len(), 1);
                assert_eq!(resp.items[0].name, "file-read");
                assert_eq!(resp.items[0].params["path"], "/tmp/test");
                assert_eq!(resp.items[0].id.as_deref(), Some("call_001"));
            }
            other => panic!("expected ToolCalls, got {other:?}"),
        }
    }

    // ── Wiremock: streaming SSE ─────────────────────────────────────────

    #[tokio::test]
    async fn streaming_chat_round_trip() {
        let server = MockServer::start().await;

        // SSE stream: two data chunks then [DONE]
        let sse_body = [
            "data: {\"id\":\"gen-stream\",\"object\":\"chat.completion.chunk\",\"created\":1700000000,\"model\":\"meta-llama/llama-3-8b\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"Hello \"},\"finish_reason\":null}],\"usage\":null}\n\n",
            "data: {\"id\":\"gen-stream\",\"object\":\"chat.completion.chunk\",\"created\":1700000000,\"model\":\"meta-llama/llama-3-8b\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"world!\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n\n",
            "data: [DONE]\n\n",
        ]
        .join("");

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
            .expect(1)
            .mount(&server)
            .await;

        let provider = make_provider(&server.uri());

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let result = provider
            .chat_streaming("sys", &[], &[], Some(tx))
            .await
            .unwrap();

        match result {
            LlmResponse::FinalAnswer(text, meta) => {
                assert_eq!(text, "Hello world!");
                assert_eq!(meta.model.as_deref(), Some("meta-llama/llama-3-8b"));
            }
            other => panic!("expected FinalAnswer, got {other:?}"),
        }

        // Verify stream chunks were delivered
        let mut chunks = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            chunks.push(chunk);
        }
        assert!(
            chunks.len() >= 2,
            "expected at least 2 stream chunks, got {}",
            chunks.len()
        );
    }

    // ── Wiremock: streaming tool calls ──────────────────────────────────

    #[tokio::test]
    async fn streaming_tool_call() {
        let server = MockServer::start().await;

        let sse_body = [
            "data: {\"id\":\"gen-tc\",\"object\":\"chat.completion.chunk\",\"created\":1700000000,\"model\":\"openai/gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"tool_calls\":[{\"index\":0,\"id\":\"call_s1\",\"type\":\"function\",\"function\":{\"name\":\"web-fetch\",\"arguments\":\"\"}}]},\"finish_reason\":null}],\"usage\":null}\n\n",
            "data: {\"id\":\"gen-tc\",\"object\":\"chat.completion.chunk\",\"created\":1700000000,\"model\":\"openai/gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"url\\\":\\\"https://example.com\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":15,\"completion_tokens\":8,\"total_tokens\":23}}\n\n",
            "data: [DONE]\n\n",
        ].join("");

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
            .expect(1)
            .mount(&server)
            .await;

        let provider = make_provider(&server.uri());

        let spec = ToolSpec {
            name: "web-fetch".into(),
            description: "Fetch a URL".into(),
            params_schema: json!({"type": "object", "properties": {"url": {"type": "string"}}}),
            is_mutating: false,
            requires_confirmation: false,
        };

        match provider
            .chat_streaming("sys", &[], &[spec], None)
            .await
            .unwrap()
        {
            LlmResponse::ToolCalls(resp) => {
                assert_eq!(resp.items.len(), 1);
                assert_eq!(resp.items[0].name, "web-fetch");
                assert_eq!(resp.items[0].params["url"], "https://example.com");
            }
            other => panic!("expected ToolCalls, got {other:?}"),
        }
    }

    // ── Wiremock: error handling ─────────────────────────────────────────

    #[tokio::test]
    async fn api_error_propagated() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": {
                    "message": "Invalid API key",
                    "type": "authentication_error"
                }
            })))
            .expect(1..)
            .mount(&server)
            .await;

        let provider = make_provider(&server.uri());
        let result = provider.chat("sys", &[], &[]).await;
        assert!(result.is_err(), "should propagate API error");
    }

    // ── Live integration test ───────────────────────────────────────────

    #[tokio::test]
    #[ignore = "requires OPENROUTER_API_KEY"]
    async fn live_openrouter_chat() {
        let api_key = std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY must be set");
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            model: "meta-llama/llama-3.3-8b-instruct:free".to_string(),
            api_key: Some(api_key),
            openrouter: OpenRouterOptions {
                referer: Some("https://assistant.test".to_string()),
                title: Some("Assistant Test Suite".to_string()),
                ..Default::default()
            },
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();

        match provider
            .chat("Reply with exactly: PONG", &[], &[])
            .await
            .unwrap()
        {
            LlmResponse::FinalAnswer(text, meta) => {
                assert!(!text.is_empty(), "response should not be empty");
                assert!(meta.model.is_some(), "model should be reported");
            }
            other => panic!("expected FinalAnswer, got {other:?}"),
        }
    }
}
