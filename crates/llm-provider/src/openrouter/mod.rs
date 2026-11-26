//! `OpenRouterProvider` — [`LlmProvider`] implementation for the OpenRouter
//! unified API gateway.
//!
//! OpenRouter provides access to 300+ models from multiple providers through
//! a single OpenAI-compatible Chat Completions endpoint. The wire client in
//! this module keeps fields a generic client drops, such as `usage.cost`,
//! and sends the configured attribution headers, routing preferences and
//! fallback models.

mod catalog;
mod citations;
mod client;
mod embedder;
pub(crate) mod errors;
mod reasoning;
mod request;
mod stream;
mod wire;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::types::llm::{LlmConfig, OPENROUTER_API_KEY_ENV, resolve_openrouter_key};
use assistant_core::{
    Capabilities, ChatHistoryMessage, ChatOptions, ChatOptionsSupport, Clock, HostedTool,
    LlmProvider, LlmResponse, ResponseFormat, StreamChunk, SystemClock, ToolChoice, ToolSpec,
    ToolSupport, chat_with_fallback,
};

use std::sync::Arc;

use self::catalog::OpenRouterCatalog;
use self::client::OpenRouterClient;
pub use self::embedder::OpenRouterEmbedder;
use self::request::{ModelInputs, RequestSettings, apply_options, build_chat_request};

// ── Defaults ────────────────────────────────────────────────────────────────

/// OpenRouter API base URL used when none is configured.
pub const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";
const DEFAULT_MAX_TOKENS: u32 = 8192;
/// Embedding model used when `[llm] embedding_model` is unset.
pub const DEFAULT_EMBEDDING_MODEL: &str = "openai/text-embedding-3-small";

// ── OpenRouterProvider ──────────────────────────────────────────────────────

/// [`LlmProvider`] backed by the OpenRouter Chat Completions API.
///
pub struct OpenRouterProvider {
    client: OpenRouterClient,
    embedding_model: String,
    catalog: Arc<OpenRouterCatalog>,
    settings: RequestSettings,
    base_url: String,
}

impl OpenRouterProvider {
    async fn send(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        options: &ChatOptions,
        stream: bool,
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        self.catalog.wait_ready().await;
        self.catalog.check_model()?;
        let info = self.catalog.info();
        let inputs = ModelInputs {
            vision: info.as_deref().is_some_and(|i| i.vision),
            file_input: info.as_deref().is_some_and(|i| i.file_input),
        };
        let mut request = build_chat_request(&self.settings, system_prompt, history, tools, inputs);
        request.stream = stream;
        apply_options(&mut request, options);
        if stream {
            self.client.chat_stream(&request, chunk_sink).await
        } else {
            self.client.chat(&request).await
        }
    }

    /// Create from the unified [`LlmConfig`].
    ///
    /// Resolves the API key from `config.api_key` or `OPENROUTER_API_KEY` env
    /// var.  Falls back to `DEFAULT_BASE_URL` when the base URL is the Ollama
    /// default (i.e. not explicitly configured).
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        Self::with_clock(cfg, Arc::new(SystemClock))
    }

    /// Like [`from_llm_config`](Self::from_llm_config), timing catalog
    /// retries with `clock`.
    fn with_clock(cfg: &LlmConfig, clock: Arc<dyn Clock>) -> anyhow::Result<Self> {
        let api_key = resolve_openrouter_key(None, cfg, OPENROUTER_API_KEY_ENV, |name| {
            std::env::var(name).ok()
        })
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
            extra_headers.push(("X-OpenRouter-Title".to_string(), title.clone()));
        }
        if !cfg.openrouter.categories.is_empty() {
            extra_headers.push((
                "X-OpenRouter-Categories".to_string(),
                cfg.openrouter.categories.join(","),
            ));
        }

        let max_tokens = cfg.openrouter.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

        debug!(model = %cfg.model, base_url = %base_url, "Creating OpenRouter provider");

        Ok(Self {
            catalog: OpenRouterCatalog::new(
                base_url.clone(),
                api_key.clone(),
                cfg.timeout_secs,
                cfg.model.clone(),
                cfg.openrouter.provider.clone(),
                cfg.openrouter.reasoning.as_ref().and_then(|r| r.effort),
                clock,
            )?,
            client: OpenRouterClient::new(
                base_url.clone(),
                api_key,
                cfg.timeout_secs,
                extra_headers,
            )?,
            embedding_model: cfg
                .embedding_model
                .clone()
                .unwrap_or_else(|| DEFAULT_EMBEDDING_MODEL.to_string()),
            settings: RequestSettings {
                model: cfg.model.clone(),
                max_tokens,
                fallback_models: cfg.openrouter.fallback_models.clone(),
                routing: cfg.openrouter.provider.clone(),
                reasoning: cfg.openrouter.reasoning.clone(),
                cache: cfg.openrouter.cache.clone(),
                web_search: cfg.openrouter.web_search.clone(),
                pdf_engine: cfg.openrouter.pdf_engine,
            },
            base_url,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    /// From the model catalog once it has loaded. Until then, and when it
    /// can't be reached: native tools, no vision, unknown context window.
    fn capabilities(&self) -> Capabilities {
        let info = self.catalog.info();
        Capabilities {
            tools: match info.as_deref() {
                Some(info) if !info.tools => ToolSupport::None,
                _ => ToolSupport::Native,
            },
            streaming: true,
            vision: info.as_deref().is_some_and(|i| i.vision),
            hosted_tools: if self.settings.web_search.enabled {
                vec![HostedTool::WebSearch]
            } else {
                vec![]
            },
            context_window_tokens: info.and_then(|i| i.context_window_tokens),
        }
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        let options = ChatOptions::default();
        self.send(system_prompt, history, tools, &options, false, None)
            .await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        let options = ChatOptions::default();
        self.send(system_prompt, history, tools, &options, true, chunk_sink)
            .await
    }

    async fn ready(&self) {
        self.catalog.wait_ready().await;
    }

    /// From the model's catalog entry; nothing until the catalog loads.
    fn chat_options_support(&self) -> ChatOptionsSupport {
        self.catalog
            .info()
            .map_or_else(ChatOptionsSupport::default, |info| ChatOptionsSupport {
                tool_choice: info.tool_choice,
                parallel_tool_calls: info.parallel_tool_calls,
                json_schema: info.json_schema,
            })
    }

    /// Sends the options the model supports natively. A `tool_choice` the
    /// model doesn't support goes through [`chat_with_fallback`]; other
    /// unsupported options are dropped.
    async fn chat_with(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        options: &ChatOptions,
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        self.catalog.wait_ready().await;
        let support = self.chat_options_support();
        if options.tool_choice != ToolChoice::Auto && !support.tool_choice {
            return chat_with_fallback(self, system_prompt, history, tools, options, chunk_sink)
                .await;
        }
        let native = ChatOptions {
            tool_choice: options.tool_choice.clone(),
            parallel_tool_calls: options
                .parallel_tool_calls
                .filter(|_| support.parallel_tool_calls),
            response_format: if support.json_schema {
                options.response_format.clone()
            } else {
                ResponseFormat::Text
            },
        };
        let stream = chunk_sink.is_some();
        self.send(system_prompt, history, tools, &native, stream, chunk_sink)
            .await
    }

    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        self.client
            .embed(&self.embedding_model, &[text], None)
            .await?
            .pop()
            .ok_or_else(|| anyhow::anyhow!("OpenRouter returned no embedding"))
    }

    async fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        self.client.embed(&self.embedding_model, texts, None).await
    }

    fn provider_name(&self) -> &str {
        "openrouter"
    }

    fn model_name(&self) -> &str {
        &self.settings.model
    }

    fn server_address(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::llm::LlmProviderKind;
    use assistant_core::types::llm::OpenRouterOptions;
    use assistant_core::{
        ChatOptions, ChatOptionsSupport, HostedTool, ResponseFormat, Secret, ToolChoice,
    };
    use serde_json::{Value, json};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ── Config / construction tests ─────────────────────────────────────

    #[test]
    fn provider_name_is_openrouter() {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some(Secret::new("test-key")),
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();
        assert_eq!(provider.provider_name(), "openrouter");
    }

    #[test]
    fn default_base_url_when_not_configured() {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some(Secret::new("test-key")),
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();
        assert_eq!(provider.server_address(), DEFAULT_BASE_URL);
    }

    #[test]
    fn custom_base_url_preserved() {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some(Secret::new("test-key")),
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
            api_key: Some(Secret::new("test-key")),
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();
        let caps = provider.capabilities();
        assert_eq!(caps.tools, ToolSupport::Native);
        assert!(caps.streaming);
        assert!(!caps.vision, "conservative until the catalog loads");
        assert_eq!(caps.context_window_tokens, None);
    }

    #[test]
    fn openrouter_options_applied_to_provider() {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            api_key: Some(Secret::new("sk-or-test")),
            openrouter: OpenRouterOptions {
                referer: Some("https://example.com".to_string()),
                title: Some("Test App".to_string()),
                max_tokens: Some(4096),
                ..OpenRouterOptions::default()
            },
            ..LlmConfig::default()
        };
        let provider = OpenRouterProvider::from_llm_config(&cfg).unwrap();
        assert_eq!(provider.model_name(), "anthropic/claude-sonnet-4-20250514");
        assert_eq!(provider.settings.max_tokens, 4096);
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
            api_key: Some(Secret::new("test-key")),
            base_url: server_uri.to_string(),
            openrouter: OpenRouterOptions {
                referer: Some("https://my-app.example.com".to_string()),
                title: Some("My Test App".to_string()),
                max_tokens: Some(4096),
                ..OpenRouterOptions::default()
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
            .and(header("x-openrouter-title", "My Test App"))
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
            requires_rich_client: false,
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

    // ── Wiremock: OpenRouter wire format ────────────────────────────────

    async fn mount_once(server: &MockServer, body: Value) {
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(server)
            .await;
    }

    /// The chat requests the server received, ignoring catalog fetches.
    async fn chat_requests(server: &MockServer) -> Vec<wiremock::Request> {
        server
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|r| r.url.path() == "/chat/completions")
            .collect()
    }

    async fn sent_body(server: &MockServer) -> Value {
        let requests = chat_requests(server).await;
        serde_json::from_slice(&requests[0].body).unwrap()
    }

    fn read_tool() -> ToolSpec {
        ToolSpec {
            name: "file-read".into(),
            description: "Read a file".into(),
            params_schema: json!({"type": "object"}),
            is_mutating: false,
            requires_confirmation: false,
            requires_rich_client: false,
        }
    }

    #[tokio::test]
    async fn request_carries_messages_tools_and_max_completion_tokens() {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let history = [ChatHistoryMessage::Text {
            role: assistant_core::ChatRole::User,
            content: "hi".into(),
        }];

        make_provider(&server.uri())
            .chat("sys", &history, &[read_tool()])
            .await
            .unwrap();

        let body = sent_body(&server).await;
        assert_eq!(body["model"], LlmConfig::default().model);
        assert_eq!(body["max_completion_tokens"], 4096);
        assert_eq!(
            body["messages"],
            json!([
                {"role": "system", "content": "sys"},
                {"role": "user", "content": "hi"},
            ])
        );
        assert_eq!(
            body["tools"],
            json!([{"type": "function", "function": {
                "name": "file-read",
                "description": "Read a file",
                "parameters": {"type": "object"},
            }}])
        );
        for absent in [
            "stream",
            "tool_choice",
            "parallel_tool_calls",
            "response_format",
        ] {
            assert!(
                body.get(absent).is_none(),
                "{absent} must not be sent: {body}"
            );
        }
    }

    #[tokio::test]
    async fn request_without_tools_omits_the_tools_field() {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;

        make_provider(&server.uri())
            .chat("", &[], &[])
            .await
            .unwrap();

        let body = sent_body(&server).await;
        assert!(body.get("tools").is_none(), "{body}");
        assert_eq!(body["messages"], json!([]), "no system message when empty");
    }

    #[tokio::test]
    async fn response_reports_served_model_and_full_usage() {
        let server = MockServer::start().await;
        mount_once(
            &server,
            json!({
                "id": "gen-1",
                "model": "anthropic/claude-sonnet-4.5",
                "choices": [{"finish_reason": "stop", "message": {
                    "role": "assistant", "content": "Hello"
                }}],
                "usage": {
                    "prompt_tokens": 100,
                    "completion_tokens": 20,
                    "total_tokens": 120,
                    "cost": 0.00123,
                    "prompt_tokens_details": {"cached_tokens": 40, "cache_write_tokens": 10},
                    "completion_tokens_details": {"reasoning_tokens": 5},
                },
            }),
        )
        .await;

        let response = make_provider(&server.uri())
            .chat("sys", &[], &[])
            .await
            .unwrap();
        let LlmResponse::FinalAnswer(text, meta) = response else {
            panic!("expected a final answer, got {response:?}");
        };
        assert_eq!(text, "Hello");
        assert_eq!(meta.model.as_deref(), Some("anthropic/claude-sonnet-4.5"));
        assert_eq!(meta.response_id.as_deref(), Some("gen-1"));
        assert_eq!(meta.finish_reason.as_deref(), Some("stop"));
        assert_eq!(meta.input_tokens, Some(100));
        assert_eq!(meta.output_tokens, Some(20));
        assert_eq!(meta.cached_input_tokens, Some(40));
        assert_eq!(meta.cache_write_input_tokens, Some(10));
        assert_eq!(meta.reasoning_tokens, Some(5));
        assert_eq!(meta.cost_usd, Some(0.00123));
    }

    #[tokio::test]
    async fn several_tool_calls_keep_order_ids_and_arguments() {
        let server = MockServer::start().await;
        mount_once(
            &server,
            json!({
                "id": "gen-2",
                "model": "openai/gpt-4o",
                "choices": [{"finish_reason": "tool_calls", "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {"id": "call_a", "type": "function", "function": {
                            "name": "file-read", "arguments": "{\"path\":\"/a\"}"}},
                        {"id": "call_b", "type": "function", "function": {
                            "name": "file-read", "arguments": "not json"}},
                    ],
                }}],
            }),
        )
        .await;

        let response = make_provider(&server.uri())
            .chat("sys", &[], &[read_tool()])
            .await
            .unwrap();
        let LlmResponse::ToolCalls(calls) = response else {
            panic!("expected tool calls, got {response:?}");
        };
        let ids: Vec<_> = calls.items.iter().map(|c| c.id.as_deref()).collect();
        assert_eq!(ids, [Some("call_a"), Some("call_b")]);
        assert_eq!(calls.items[0].params, json!({"path": "/a"}));
        assert_eq!(
            calls.items[1].params,
            json!({}),
            "bad arguments become an empty object"
        );
        assert_eq!(calls.meta.finish_reason.as_deref(), Some("tool_calls"));
        assert_eq!(calls.meta.input_tokens, None, "no usage reported");
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
            requires_rich_client: false,
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

    // ── Wiremock: attribution, routing and identity ──────────────────────

    fn provider_with(server_uri: &str, openrouter: OpenRouterOptions) -> OpenRouterProvider {
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            model: "anthropic/claude-sonnet-4.5".to_string(),
            api_key: Some(Secret::new("test-key")),
            base_url: server_uri.to_string(),
            openrouter,
            ..LlmConfig::default()
        };
        OpenRouterProvider::from_llm_config(&cfg).unwrap()
    }

    #[tokio::test]
    async fn categories_are_sent_comma_separated() {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let options = OpenRouterOptions {
            categories: vec!["cli-agent".into(), "chat".into()],
            ..OpenRouterOptions::default()
        };

        provider_with(&server.uri(), options)
            .chat("sys", &[], &[])
            .await
            .unwrap();

        let requests = chat_requests(&server).await;
        let value = requests[0].headers.get("x-openrouter-categories").unwrap();
        assert_eq!(value.to_str().unwrap(), "cli-agent,chat");
    }

    #[tokio::test]
    async fn routing_and_fallback_models_are_sent() {
        use assistant_core::types::llm::{OpenRouterDataCollection, OpenRouterRouting};

        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let options = OpenRouterOptions {
            fallback_models: vec!["openai/gpt-4o".into()],
            provider: Some(OpenRouterRouting {
                data_collection: Some(OpenRouterDataCollection::Deny),
                zdr: Some(true),
                ..OpenRouterRouting::default()
            }),
            ..OpenRouterOptions::default()
        };

        provider_with(&server.uri(), options)
            .chat("sys", &[], &[])
            .await
            .unwrap();

        let body = sent_body(&server).await;
        assert_eq!(
            body["provider"],
            json!({"data_collection": "deny", "zdr": true})
        );
        assert_eq!(
            body["models"],
            json!(["anthropic/claude-sonnet-4.5", "openai/gpt-4o"]),
            "primary model first"
        );
    }

    #[tokio::test]
    async fn no_routing_fields_without_config_or_context() {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;

        provider_with(&server.uri(), OpenRouterOptions::default())
            .chat("sys", &[], &[])
            .await
            .unwrap();

        let body = sent_body(&server).await;
        for absent in ["provider", "models", "session_id", "user"] {
            assert!(
                body.get(absent).is_none(),
                "{absent} must not be sent: {body}"
            );
        }
    }

    #[tokio::test]
    async fn turn_context_sets_session_id_and_user() {
        use assistant_core::llm::call_context::LlmCallContext;

        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let conversation_id = Default::default();
        let context = LlmCallContext {
            conversation_id,
            user_hash: Some("abc123".into()),
        };

        let provider = provider_with(&server.uri(), OpenRouterOptions::default());
        context.scope(provider.chat("sys", &[], &[])).await.unwrap();

        let body = sent_body(&server).await;
        assert_eq!(body["session_id"], conversation_id.to_string());
        assert_eq!(body["user"], "abc123");
    }

    // ── Wiremock: model catalog ─────────────────────────────────────────

    const MODEL: &str = "anthropic/claude-sonnet-4.5";

    /// Serves `model` from `/models` and `endpoints` from its endpoints route.
    async fn mount_catalog(server: &MockServer, model: Value, endpoints: Value) {
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [
                {"id": "other/model", "context_length": 1000},
                model,
            ]})))
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/models/{MODEL}/endpoints")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data": {"id": MODEL, "endpoints": endpoints}})),
            )
            .mount(server)
            .await;
    }

    fn catalog_model(input: &[&str], params: &[&str]) -> Value {
        json!({
            "id": MODEL,
            "context_length": 200_000,
            "architecture": {"input_modalities": input, "output_modalities": ["text"]},
            "supported_parameters": params,
        })
    }

    fn endpoint(provider: &str, tag: &str, context_length: u64) -> Value {
        json!({"provider_name": provider, "tag": tag, "context_length": context_length})
    }

    /// Capabilities after the first chat call, which waits for the catalog.
    async fn caps_after_first_call(
        server: &MockServer,
        options: OpenRouterOptions,
    ) -> Capabilities {
        mount_once(server, chat_response("OK")).await;
        let provider = provider_with(&server.uri(), options);
        provider.chat("sys", &[], &[]).await.unwrap();
        provider.capabilities()
    }

    #[tokio::test]
    async fn catalog_sets_context_window_vision_and_tools() {
        let server = MockServer::start().await;
        mount_catalog(
            &server,
            catalog_model(&["text", "image"], &["tools", "tool_choice"]),
            json!([
                endpoint("Anthropic", "anthropic", 200_000),
                endpoint("Google", "google-vertex", 128_000)
            ]),
        )
        .await;

        let caps = caps_after_first_call(&server, OpenRouterOptions::default()).await;
        assert_eq!(
            caps.context_window_tokens,
            Some(128_000),
            "smallest selectable endpoint"
        );
        assert!(caps.vision);
        assert_eq!(caps.tools, ToolSupport::Native);
    }

    #[tokio::test]
    async fn text_only_model_without_tools() {
        let server = MockServer::start().await;
        mount_catalog(
            &server,
            catalog_model(&["text"], &["temperature"]),
            json!([endpoint("Anthropic", "anthropic", 200_000)]),
        )
        .await;

        let caps = caps_after_first_call(&server, OpenRouterOptions::default()).await;
        assert!(!caps.vision);
        assert_eq!(caps.tools, ToolSupport::None);
    }

    fn routing(routing: assistant_core::types::llm::OpenRouterRouting) -> OpenRouterOptions {
        OpenRouterOptions {
            provider: Some(routing),
            ..OpenRouterOptions::default()
        }
    }

    async fn two_endpoint_catalog(server: &MockServer) {
        mount_catalog(
            server,
            catalog_model(&["text"], &["tools"]),
            json!([
                endpoint("Anthropic", "anthropic", 200_000),
                endpoint("Google", "google-vertex", 128_000)
            ]),
        )
        .await;
    }

    #[tokio::test]
    async fn only_restricts_the_selectable_endpoints() {
        use assistant_core::types::llm::OpenRouterRouting;
        let server = MockServer::start().await;
        two_endpoint_catalog(&server).await;

        let options = routing(OpenRouterRouting {
            only: Some(vec!["anthropic".into()]),
            ..OpenRouterRouting::default()
        });
        let caps = caps_after_first_call(&server, options).await;
        assert_eq!(caps.context_window_tokens, Some(200_000));
    }

    #[tokio::test]
    async fn order_restricts_only_without_fallbacks() {
        use assistant_core::types::llm::OpenRouterRouting;
        let server = MockServer::start().await;
        two_endpoint_catalog(&server).await;
        let order = Some(vec!["anthropic".into()]);

        let with_fallbacks = routing(OpenRouterRouting {
            order: order.clone(),
            ..OpenRouterRouting::default()
        });
        let caps = caps_after_first_call(&server, with_fallbacks).await;
        assert_eq!(
            caps.context_window_tokens,
            Some(128_000),
            "fallbacks may reach google"
        );

        let server = MockServer::start().await;
        two_endpoint_catalog(&server).await;
        let pinned = routing(OpenRouterRouting {
            order,
            allow_fallbacks: Some(false),
            ..OpenRouterRouting::default()
        });
        let caps = caps_after_first_call(&server, pinned).await;
        assert_eq!(caps.context_window_tokens, Some(200_000));
    }

    #[tokio::test]
    async fn ignored_endpoints_do_not_count() {
        use assistant_core::types::llm::OpenRouterRouting;
        let server = MockServer::start().await;
        two_endpoint_catalog(&server).await;

        let options = routing(OpenRouterRouting {
            ignore: Some(vec!["google-vertex".into()]),
            ..OpenRouterRouting::default()
        });
        let caps = caps_after_first_call(&server, options).await;
        assert_eq!(caps.context_window_tokens, Some(200_000));
    }

    #[tokio::test]
    async fn first_call_waits_for_a_slow_catalog() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_millis(300))
                    .set_body_json(
                        json!({"data": [catalog_model(&["text", "image"], &["tools"])]}),
                    ),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/models/{MODEL}/endpoints")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"data": {"endpoints": [
                    endpoint("Anthropic", "anthropic", 64_000)
                ]}})),
            )
            .mount(&server)
            .await;

        let caps = caps_after_first_call(&server, OpenRouterOptions::default()).await;
        assert_eq!(caps.context_window_tokens, Some(64_000));
    }

    #[tokio::test]
    async fn catalog_failure_keeps_conservative_capabilities() {
        let server = MockServer::start().await;
        // No catalog mocks: /models answers 404.

        let caps = caps_after_first_call(&server, OpenRouterOptions::default()).await;
        assert_eq!(caps.tools, ToolSupport::Native);
        assert!(!caps.vision);
        assert_eq!(caps.context_window_tokens, None);
    }

    async fn catalog_fetches(server: &MockServer) -> usize {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == "/models")
            .count()
    }

    #[tokio::test]
    async fn unknown_model_fails_every_call_without_an_upstream_request() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [
                {"id": "other/model"}
            ]})))
            .mount(&server)
            .await;
        let provider = provider_with(&server.uri(), OpenRouterOptions::default());

        for _ in 0..2 {
            let err = provider
                .chat("sys", &[], &[])
                .await
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("no model `anthropic/claude-sonnet-4.5`"),
                "{err}"
            );
        }
        let err = provider
            .chat_streaming("sys", &[], &[], None)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("no model"), "{err}");
        assert_eq!(request_count(&server).await, 0, "nothing is sent upstream");
        assert_eq!(
            catalog_fetches(&server).await,
            1,
            "an unknown model is not refetched"
        );
    }

    #[tokio::test]
    async fn catalog_outage_is_retried_after_ten_minutes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        mount_catalog(
            &server,
            catalog_model(&["text", "image"], &["tools"]),
            json!([endpoint("Anthropic", "anthropic", 64_000)]),
        )
        .await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(chat_response("OK")))
            .mount(&server)
            .await;
        let clock = assistant_core::FakeClock::new_at_seed();
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            model: MODEL.to_string(),
            api_key: Some(Secret::new("test-key")),
            base_url: server.uri(),
            ..LlmConfig::default()
        };
        let provider =
            OpenRouterProvider::with_clock(&cfg, std::sync::Arc::new(clock.clone())).unwrap();

        provider.chat("sys", &[], &[]).await.unwrap();
        assert!(
            !provider.capabilities().vision,
            "conservative during the outage"
        );
        assert_eq!(catalog_fetches(&server).await, 1);

        clock.advance(std::time::Duration::from_secs(9 * 60));
        provider.chat("sys", &[], &[]).await.unwrap();
        assert_eq!(
            catalog_fetches(&server).await,
            1,
            "no retry before ten minutes"
        );

        clock.advance(std::time::Duration::from_secs(60));
        provider.chat("sys", &[], &[]).await.unwrap();
        assert_eq!(catalog_fetches(&server).await, 2);
        let caps = provider.capabilities();
        assert!(caps.vision, "capabilities update once a fetch succeeds");
        assert_eq!(caps.context_window_tokens, Some(64_000));

        clock.advance(std::time::Duration::from_secs(60 * 60));
        provider.chat("sys", &[], &[]).await.unwrap();
        assert_eq!(
            catalog_fetches(&server).await,
            2,
            "a loaded catalog is not refetched"
        );
    }

    // ── Wiremock: native chat options ───────────────────────────────────

    /// The system prompt text, whether sent as a string or as text parts.
    fn system_text(body: &Value) -> String {
        let content = &body["messages"][0]["content"];
        match content.as_array() {
            Some(parts) => parts.iter().filter_map(|p| p["text"].as_str()).collect(),
            None => content.as_str().unwrap_or_default().to_owned(),
        }
    }

    fn submit_tool() -> ToolSpec {
        ToolSpec {
            name: "submit".into(),
            description: "Submit".into(),
            params_schema: json!({"type": "object"}),
            is_mutating: false,
            requires_confirmation: false,
            requires_rich_client: false,
        }
    }

    fn submit_call() -> Value {
        json!({
            "id": "gen-t",
            "model": MODEL,
            "choices": [{"finish_reason": "tool_calls", "message": {"role": "assistant", "tool_calls": [
                {"id": "call_1", "type": "function", "function": {"name": "submit", "arguments": "{}"}}
            ]}}],
        })
    }

    async fn provider_for(
        server: &MockServer,
        params: &[&str],
        options: OpenRouterOptions,
    ) -> OpenRouterProvider {
        mount_catalog(
            server,
            catalog_model(&["text"], params),
            json!([endpoint("Anthropic", "anthropic", 200_000)]),
        )
        .await;
        provider_with(&server.uri(), options)
    }

    fn forced(name: &str) -> ChatOptions {
        ChatOptions {
            tool_choice: ToolChoice::Tool(name.into()),
            ..ChatOptions::default()
        }
    }

    #[tokio::test]
    async fn option_support_comes_from_the_catalog() {
        let cases: [(&[&str], ChatOptionsSupport); 3] = [
            (
                &[
                    "tools",
                    "tool_choice",
                    "parallel_tool_calls",
                    "structured_outputs",
                ],
                ChatOptionsSupport {
                    tool_choice: true,
                    parallel_tool_calls: true,
                    json_schema: true,
                },
            ),
            (
                &["tools", "response_format"],
                ChatOptionsSupport {
                    json_schema: true,
                    ..ChatOptionsSupport::default()
                },
            ),
            (&["tools"], ChatOptionsSupport::default()),
        ];
        for (params, expected) in cases {
            let server = MockServer::start().await;
            mount_once(&server, chat_response("OK")).await;
            let provider = provider_for(&server, params, OpenRouterOptions::default()).await;
            provider.chat("sys", &[], &[]).await.unwrap();
            assert_eq!(provider.chat_options_support(), expected, "{params:?}");
        }
    }

    #[tokio::test]
    async fn forced_tool_is_sent_natively_and_requires_parameters() {
        use assistant_core::types::llm::{OpenRouterDataCollection, OpenRouterRouting};

        let server = MockServer::start().await;
        mount_once(&server, submit_call()).await;
        let options = OpenRouterOptions {
            provider: Some(OpenRouterRouting {
                data_collection: Some(OpenRouterDataCollection::Deny),
                ..OpenRouterRouting::default()
            }),
            ..OpenRouterOptions::default()
        };
        let provider = provider_for(&server, &["tools", "tool_choice"], options).await;

        provider
            .chat_with("sys", &[], &[submit_tool()], &forced("submit"), None)
            .await
            .unwrap();

        let body = sent_body(&server).await;
        assert_eq!(
            body["tool_choice"],
            json!({"type": "function", "function": {"name": "submit"}})
        );
        assert_eq!(
            body["provider"],
            json!({"data_collection": "deny", "require_parameters": true}),
            "configured routing is kept"
        );
        assert_eq!(system_text(&body), "sys", "no prompt instruction");
    }

    #[tokio::test]
    async fn parallel_tool_calls_is_sent_only_with_tools() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(chat_response("OK")))
            .mount(&server)
            .await;
        let provider = provider_for(
            &server,
            &["tools", "parallel_tool_calls"],
            OpenRouterOptions::default(),
        )
        .await;
        let sequential = ChatOptions {
            parallel_tool_calls: Some(false),
            ..ChatOptions::default()
        };

        provider
            .chat_with("sys", &[], &[submit_tool()], &sequential, None)
            .await
            .unwrap();
        provider
            .chat_with("sys", &[], &[], &sequential, None)
            .await
            .unwrap();

        let requests = chat_requests(&server).await;
        let with_tools: Value = serde_json::from_slice(&requests[0].body).unwrap();
        let without: Value = serde_json::from_slice(&requests[1].body).unwrap();
        assert_eq!(with_tools["parallel_tool_calls"], false);
        assert!(
            with_tools.get("provider").is_none(),
            "parallel alone needs no require_parameters"
        );
        assert!(without.get("parallel_tool_calls").is_none(), "{without}");
    }

    #[tokio::test]
    async fn json_schema_is_native_with_response_healing_when_not_streaming() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(chat_response(r#"{"ok": true}"#)),
            )
            .mount(&server)
            .await;
        let provider = provider_for(
            &server,
            &["structured_outputs"],
            OpenRouterOptions::default(),
        )
        .await;
        let schema = json!({
            "type": "object",
            "properties": {"ok": {"type": "boolean"}},
            "required": ["ok"],
            "additionalProperties": false,
        });

        let value = assistant_core::chat_structured(&provider, "sys", &[], "verdict", &schema)
            .await
            .unwrap();
        assert_eq!(value, json!({"ok": true}));

        let body = sent_body(&server).await;
        assert_eq!(
            body["response_format"],
            json!({"type": "json_schema", "json_schema": {"name": "verdict", "strict": true, "schema": schema}})
        );
        assert_eq!(body["plugins"], json!([{"id": "response-healing"}]));
        assert_eq!(body["provider"], json!({"require_parameters": true}));
    }

    #[tokio::test]
    async fn streaming_json_schema_skips_response_healing() {
        let server = MockServer::start().await;
        mount_sse(
            &server,
            &[
                r#"data: {"choices":[{"delta":{"content":"{}"},"finish_reason":"stop"}]}"#,
                "data: [DONE]",
            ],
        )
        .await;
        let provider = provider_for(
            &server,
            &["structured_outputs"],
            OpenRouterOptions::default(),
        )
        .await;
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        let options = ChatOptions {
            response_format: ResponseFormat::JsonSchema {
                name: "verdict".into(),
                schema: json!({"type": "object"}),
                strict: true,
            },
            ..ChatOptions::default()
        };

        provider
            .chat_with("sys", &[], &[], &options, Some(tx))
            .await
            .unwrap();

        let body = sent_body(&server).await;
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert!(body.get("plugins").is_none(), "{body}");
    }

    #[tokio::test]
    async fn unsupported_tool_choice_uses_the_prompt_fallback() {
        let server = MockServer::start().await;
        mount_once(&server, submit_call()).await;
        let provider = provider_for(&server, &["tools"], OpenRouterOptions::default()).await;

        provider
            .chat_with("sys", &[], &[submit_tool()], &forced("submit"), None)
            .await
            .unwrap();

        let body = sent_body(&server).await;
        assert!(body.get("tool_choice").is_none(), "{body}");
        assert!(body.get("provider").is_none(), "{body}");
        let system = system_text(&body);
        assert!(system.contains("`submit`"), "{system}");
    }

    fn photo_message() -> Vec<ChatHistoryMessage> {
        vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                assistant_core::ContentBlock::Text("what is this?".into()),
                assistant_core::ContentBlock::Image {
                    media_type: "image/png".into(),
                    data: "aGk=".into(),
                },
            ],
        }]
    }

    #[tokio::test]
    async fn text_only_model_gets_a_note_instead_of_the_image() {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let provider = provider_for(&server, &["tools"], OpenRouterOptions::default()).await;

        provider.chat("sys", &photo_message(), &[]).await.unwrap();

        let body = sent_body(&server).await;
        let parts = body["messages"][1]["content"].as_array().unwrap();
        assert!(
            parts.iter().all(|p| p["type"] == "text"),
            "no image parts: {parts:?}"
        );
        assert!(
            parts
                .iter()
                .any(|p| p["text"].as_str().unwrap().contains("image")),
            "the model is told an image was left out: {parts:?}"
        );
    }

    #[tokio::test]
    async fn vision_model_gets_the_image() {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        mount_catalog(
            &server,
            catalog_model(&["text", "image"], &["tools"]),
            json!([endpoint("Anthropic", "anthropic", 200_000)]),
        )
        .await;
        let provider = provider_with(&server.uri(), OpenRouterOptions::default());

        provider.chat("sys", &photo_message(), &[]).await.unwrap();

        let body = sent_body(&server).await;
        let parts = body["messages"][1]["content"].as_array().unwrap();
        assert!(parts.iter().any(|p| p["type"] == "image_url"), "{parts:?}");
    }

    // ── Wiremock: reasoning ─────────────────────────────────────────────

    fn reasoning_options(
        effort: assistant_core::types::llm::OpenRouterEffort,
    ) -> OpenRouterOptions {
        OpenRouterOptions {
            reasoning: Some(assistant_core::types::llm::OpenRouterReasoning {
                effort: Some(effort),
                ..Default::default()
            }),
            ..OpenRouterOptions::default()
        }
    }

    async fn reasoning_provider(
        server: &MockServer,
        reasoning: Option<Value>,
        options: OpenRouterOptions,
    ) -> OpenRouterProvider {
        let mut model = catalog_model(&["text"], &["tools", "reasoning"]);
        if let Some(reasoning) = reasoning {
            model["reasoning"] = reasoning;
        }
        mount_catalog(
            server,
            model,
            json!([endpoint("Anthropic", "anthropic", 200_000)]),
        )
        .await;
        provider_with(&server.uri(), options)
    }

    #[tokio::test]
    async fn configured_reasoning_is_sent() {
        use assistant_core::types::llm::OpenRouterEffort;
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let provider = reasoning_provider(
            &server,
            Some(json!({"supported_efforts": ["high", "medium", "low"]})),
            reasoning_options(OpenRouterEffort::High),
        )
        .await;

        provider.chat("sys", &[], &[]).await.unwrap();

        assert_eq!(
            sent_body(&server).await["reasoning"],
            json!({"effort": "high"})
        );
    }

    #[tokio::test]
    async fn unsupported_effort_fails_every_call_with_the_supported_list() {
        use assistant_core::types::llm::OpenRouterEffort;
        let server = MockServer::start().await;
        let provider = reasoning_provider(
            &server,
            Some(json!({"supported_efforts": ["high", "medium"]})),
            reasoning_options(OpenRouterEffort::Xhigh),
        )
        .await;

        for _ in 0..2 {
            let err = provider
                .chat("sys", &[], &[])
                .await
                .unwrap_err()
                .to_string();
            assert!(err.contains("xhigh"), "{err}");
            assert!(err.contains("high, medium"), "{err}");
        }
        assert_eq!(request_count(&server).await, 0, "nothing is sent upstream");
    }

    #[tokio::test]
    async fn null_supported_efforts_accepts_any_effort() {
        use assistant_core::types::llm::OpenRouterEffort;
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let provider = reasoning_provider(
            &server,
            Some(json!({"supported_efforts": null})),
            reasoning_options(OpenRouterEffort::Max),
        )
        .await;

        provider.chat("sys", &[], &[]).await.unwrap();
    }

    #[tokio::test]
    async fn effort_on_a_model_without_effort_selection_fails() {
        use assistant_core::types::llm::OpenRouterEffort;
        let server = MockServer::start().await;
        let provider =
            reasoning_provider(&server, None, reasoning_options(OpenRouterEffort::Low)).await;

        let err = provider
            .chat("sys", &[], &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not support reasoning effort"), "{err}");
    }

    #[tokio::test]
    async fn effort_none_fails_when_reasoning_is_mandatory() {
        use assistant_core::types::llm::OpenRouterEffort;
        let server = MockServer::start().await;
        let provider = reasoning_provider(
            &server,
            Some(json!({"supported_efforts": null, "mandatory": true})),
            reasoning_options(OpenRouterEffort::None),
        )
        .await;

        let err = provider
            .chat("sys", &[], &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("mandatory"), "{err}");
    }

    // ── Wiremock: reasoning output ──────────────────────────────────────

    fn reasoning_tool_call() -> Value {
        json!({
            "id": "gen-r",
            "model": MODEL,
            "choices": [{"finish_reason": "tool_calls", "message": {
                "role": "assistant",
                "content": null,
                "reasoning_details": [
                    {"type": "reasoning.text", "text": "Checking the file first", "index": 0},
                    {"type": "reasoning.summary", "summary": "Read it, then answer", "index": 1},
                    {"type": "reasoning.encrypted", "data": "c2VjcmV0", "index": 2},
                ],
                "tool_calls": [{"id": "call_1", "type": "function", "function": {
                    "name": "file-read", "arguments": "{}"}}],
            }}],
        })
    }

    fn excluding() -> OpenRouterOptions {
        OpenRouterOptions {
            reasoning: Some(assistant_core::types::llm::OpenRouterReasoning {
                exclude: Some(true),
                ..Default::default()
            }),
            ..OpenRouterOptions::default()
        }
    }

    #[tokio::test]
    async fn tool_call_reasoning_is_returned_as_thinking_without_encrypted_entries() {
        let server = MockServer::start().await;
        mount_once(&server, reasoning_tool_call()).await;

        let response = make_provider(&server.uri())
            .chat("sys", &[], &[read_tool()])
            .await
            .unwrap();

        let LlmResponse::ToolCalls(calls) = response else {
            panic!("expected tool calls, got {response:?}");
        };
        let thinking = calls.thinking.unwrap();
        assert!(thinking.contains("Checking the file first"), "{thinking}");
        assert!(thinking.contains("Read it, then answer"), "{thinking}");
        assert!(
            !thinking.contains("c2VjcmV0"),
            "encrypted entries are hidden: {thinking}"
        );
    }

    #[tokio::test]
    async fn exclude_emits_no_thinking() {
        let server = MockServer::start().await;
        mount_once(&server, reasoning_tool_call()).await;

        let response = provider_with(&server.uri(), excluding())
            .chat("sys", &[], &[read_tool()])
            .await
            .unwrap();

        let LlmResponse::ToolCalls(calls) = response else {
            panic!("expected tool calls, got {response:?}");
        };
        assert_eq!(calls.thinking, None);
    }

    fn thinking_and_text(rx: &mut tokio::sync::mpsc::Receiver<StreamChunk>) -> Vec<String> {
        let mut seen = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            match chunk {
                StreamChunk::Thinking(t) => seen.push(format!("thinking:{t}")),
                StreamChunk::Text(t) => seen.push(format!("text:{t}")),
            }
        }
        seen
    }

    const REASONING_STREAM: [&str; 4] = [
        r#"data: {"choices":[{"delta":{"reasoning_details":[{"type":"reasoning.text","text":"Checking the file first","index":0}]}}]}"#,
        r#"data: {"choices":[{"delta":{"reasoning_details":[{"type":"reasoning.encrypted","data":"c2VjcmV0","index":1}]}}]}"#,
        r#"data: {"choices":[{"delta":{"content":"Done"},"finish_reason":"stop"}]}"#,
        "data: [DONE]",
    ];

    #[tokio::test]
    async fn streamed_reasoning_comes_before_the_answer() {
        let server = MockServer::start().await;
        mount_sse(&server, &REASONING_STREAM).await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        let response = make_provider(&server.uri())
            .chat_streaming("sys", &[], &[], Some(tx))
            .await
            .unwrap();

        assert!(matches!(response, LlmResponse::FinalAnswer(ref t, _) if t == "Done"));
        assert_eq!(
            thinking_and_text(&mut rx),
            ["thinking:Checking the file first", "text:Done"],
            "encrypted entries produce no chunk"
        );
    }

    #[tokio::test]
    async fn exclude_streams_no_thinking() {
        let server = MockServer::start().await;
        mount_sse(&server, &REASONING_STREAM).await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        provider_with(&server.uri(), excluding())
            .chat_streaming("sys", &[], &[], Some(tx))
            .await
            .unwrap();

        assert_eq!(thinking_and_text(&mut rx), ["text:Done"]);
    }

    #[tokio::test]
    async fn budget_used_up_by_reasoning_is_an_error() {
        let server = MockServer::start().await;
        mount_once(
            &server,
            json!({
                "id": "gen-l",
                "model": MODEL,
                "choices": [{"finish_reason": "length", "message": {
                    "role": "assistant",
                    "content": "",
                    "reasoning": "Thinking about it at great length",
                }}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 4096,
                          "completion_tokens_details": {"reasoning_tokens": 4096}},
            }),
        )
        .await;

        let err = make_provider(&server.uri())
            .chat("sys", &[], &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("token budget") && err.contains("reasoning"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn streamed_budget_exhaustion_is_an_error() {
        let server = MockServer::start().await;
        mount_sse(
            &server,
            &[
                r#"data: {"choices":[{"delta":{"reasoning":"thinking..."}}]}"#,
                r#"data: {"choices":[{"delta":{"content":""},"finish_reason":"length"}]}"#,
                "data: [DONE]",
            ],
        )
        .await;

        let err = make_provider(&server.uri())
            .chat_streaming("sys", &[], &[], None)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("token budget"), "{err}");
    }

    // ── Wiremock: reasoning continuity ──────────────────────────────────

    fn stored_details() -> Value {
        json!([
            {"type": "reasoning.text", "text": "Checking the file first", "signature": "sig-a", "index": 0},
            {"type": "reasoning.encrypted", "data": "c2VjcmV0", "index": 1},
        ])
    }

    fn state(provider: &str, model: &str) -> assistant_core::ProviderState {
        assistant_core::ProviderState {
            provider: provider.into(),
            model: model.into(),
            data: json!({"reasoning_details": stored_details()}),
        }
    }

    #[tokio::test]
    async fn tool_call_reasoning_details_become_provider_state() {
        let server = MockServer::start().await;
        let mut body = reasoning_tool_call();
        body["choices"][0]["message"]["reasoning_details"] = stored_details();
        mount_once(&server, body).await;

        let response = provider_with(&server.uri(), OpenRouterOptions::default())
            .chat("sys", &[], &[read_tool()])
            .await
            .unwrap();

        let LlmResponse::ToolCalls(calls) = response else {
            panic!("expected tool calls, got {response:?}");
        };
        assert_eq!(calls.provider_state, Some(state("openrouter", MODEL)));
    }

    #[tokio::test]
    async fn streamed_reasoning_details_are_merged_by_index() {
        let server = MockServer::start().await;
        mount_sse(
            &server,
            &[
                r#"data: {"choices":[{"delta":{"reasoning_details":[{"type":"reasoning.text","text":"Checking the ","index":0}]}}]}"#,
                r#"data: {"choices":[{"delta":{"reasoning_details":[{"type":"reasoning.text","text":"file first","signature":"sig-a","index":0}]}}]}"#,
                r#"data: {"choices":[{"delta":{"reasoning_details":[{"type":"reasoning.encrypted","data":"c2VjcmV0","index":1}]}}]}"#,
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"file-read","arguments":"{}"}}]},"finish_reason":"tool_calls"}]}"#,
                "data: [DONE]",
            ],
        )
        .await;

        let response = provider_with(&server.uri(), OpenRouterOptions::default())
            .chat_streaming("sys", &[], &[read_tool()], None)
            .await
            .unwrap();

        let LlmResponse::ToolCalls(calls) = response else {
            panic!("expected tool calls, got {response:?}");
        };
        assert_eq!(calls.provider_state, Some(state("openrouter", MODEL)));
    }

    fn tool_loop_history(
        provider_state: Option<assistant_core::ProviderState>,
    ) -> Vec<ChatHistoryMessage> {
        vec![
            ChatHistoryMessage::Text {
                role: assistant_core::ChatRole::User,
                content: "read it".into(),
            },
            ChatHistoryMessage::AssistantToolCalls {
                items: vec![assistant_core::ToolCallItem {
                    name: "file-read".into(),
                    params: json!({}),
                    id: Some("call_1".into()),
                }],
                provider_state,
            },
            ChatHistoryMessage::ToolResult {
                name: "file-read".into(),
                content: "contents".into(),
            },
        ]
    }

    async fn sent_assistant_message(state: Option<assistant_core::ProviderState>) -> Value {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        provider_with(&server.uri(), OpenRouterOptions::default())
            .chat("sys", &tool_loop_history(state), &[read_tool()])
            .await
            .unwrap();
        let body = sent_body(&server).await;
        body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["role"] == "assistant")
            .cloned()
            .unwrap()
    }

    #[tokio::test]
    async fn stored_reasoning_details_are_sent_back_unchanged_and_in_order() {
        let message = sent_assistant_message(Some(state("openrouter", MODEL))).await;
        assert_eq!(message["reasoning_details"], stored_details());
        assert_eq!(message["tool_calls"][0]["id"], "call_1");
    }

    #[tokio::test]
    async fn reasoning_from_another_provider_or_model_is_not_sent() {
        for foreign in [
            state("anthropic", MODEL),
            state("openrouter", "openai/gpt-4o"),
        ] {
            let message = sent_assistant_message(Some(foreign.clone())).await;
            assert!(
                message.get("reasoning_details").is_none(),
                "{foreign:?}: {message}"
            );
        }
    }

    #[tokio::test]
    async fn anthropic_requests_carry_cache_breakpoints() {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let history = [ChatHistoryMessage::Text {
            role: assistant_core::ChatRole::User,
            content: "hi".into(),
        }];

        provider_with(&server.uri(), OpenRouterOptions::default())
            .chat("sys", &history, &[])
            .await
            .unwrap();

        let body = sent_body(&server).await;
        let ephemeral = json!({"type": "ephemeral"});
        assert_eq!(
            body["messages"][0]["content"][0]["cache_control"],
            ephemeral
        );
        assert_eq!(
            body["messages"][1]["content"][0]["cache_control"],
            ephemeral
        );
    }

    // ── Wiremock: web search ────────────────────────────────────────────

    fn web_search(search: assistant_core::types::llm::OpenRouterWebSearch) -> OpenRouterOptions {
        OpenRouterOptions {
            web_search: search,
            ..OpenRouterOptions::default()
        }
    }

    #[tokio::test]
    async fn enabled_web_search_sends_the_plugin_and_is_a_hosted_tool() {
        use assistant_core::types::llm::{OpenRouterSearchEngine, OpenRouterWebSearch};
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let provider = provider_with(
            &server.uri(),
            web_search(OpenRouterWebSearch {
                enabled: true,
                engine: Some(OpenRouterSearchEngine::Exa),
                max_results: Some(3),
                exclude_domains: Some(vec!["reddit.com".into()]),
                ..OpenRouterWebSearch::default()
            }),
        );

        assert_eq!(
            provider.capabilities().hosted_tools,
            [HostedTool::WebSearch]
        );
        provider.chat("sys", &[], &[]).await.unwrap();

        let body = sent_body(&server).await;
        assert_eq!(
            body["plugins"],
            json!([{"id": "web", "engine": "exa", "max_results": 3, "exclude_domains": ["reddit.com"]}])
        );
        assert!(body.get("web_search_options").is_none(), "{body}");
    }

    #[tokio::test]
    async fn search_context_size_is_sent_as_web_search_options() {
        use assistant_core::types::llm::{OpenRouterSearchContextSize, OpenRouterWebSearch};
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;

        provider_with(
            &server.uri(),
            web_search(OpenRouterWebSearch {
                enabled: true,
                search_context_size: Some(OpenRouterSearchContextSize::High),
                ..OpenRouterWebSearch::default()
            }),
        )
        .chat("sys", &[], &[])
        .await
        .unwrap();

        let body = sent_body(&server).await;
        assert_eq!(body["plugins"], json!([{"id": "web"}]));
        assert_eq!(
            body["web_search_options"],
            json!({"search_context_size": "high"})
        );
    }

    #[tokio::test]
    async fn disabled_web_search_sends_no_plugin_and_hosts_nothing() {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        let provider = provider_with(&server.uri(), OpenRouterOptions::default());

        assert!(provider.capabilities().hosted_tools.is_empty());
        provider.chat("sys", &[], &[]).await.unwrap();

        let body = sent_body(&server).await;
        assert!(body.get("plugins").is_none(), "{body}");
        assert!(body.get("web_search_options").is_none(), "{body}");
    }

    // ── Wiremock: citations ─────────────────────────────────────────────

    fn citation(url: &str, title: &str) -> Value {
        json!({"type": "url_citation", "url_citation": {"url": url, "title": title}})
    }

    #[tokio::test]
    async fn final_answer_ends_with_its_sources() {
        let server = MockServer::start().await;
        mount_once(
            &server,
            json!({
                "id": "gen-c",
                "model": MODEL,
                "choices": [{"finish_reason": "stop", "message": {
                    "role": "assistant",
                    "content": "It rained.",
                    "annotations": [citation("https://a.example", "A"), citation("https://a.example", "A")],
                }}],
            }),
        )
        .await;

        let response = make_provider(&server.uri())
            .chat("sys", &[], &[])
            .await
            .unwrap();
        let LlmResponse::FinalAnswer(text, _) = response else {
            panic!("expected a final answer, got {response:?}");
        };
        assert_eq!(text, "It rained.\n\nSources:\n- [A](https://a.example)");
    }

    #[tokio::test]
    async fn streamed_sources_arrive_as_the_last_chunk_and_match_the_answer() {
        let server = MockServer::start().await;
        mount_sse(
            &server,
            &[
                r#"data: {"choices":[{"delta":{"content":"It rained."}}]}"#,
                r#"data: {"choices":[{"delta":{"annotations":[{"type":"url_citation","url_citation":{"url":"https://a.example","title":"A"}}]},"finish_reason":"stop"}]}"#,
                "data: [DONE]",
            ],
        )
        .await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        let response = make_provider(&server.uri())
            .chat_streaming("sys", &[], &[], Some(tx))
            .await
            .unwrap();

        let LlmResponse::FinalAnswer(text, _) = response else {
            panic!("expected a final answer, got {response:?}");
        };
        let streamed = drain(&mut rx);
        assert_eq!(
            streamed.last().unwrap(),
            "\n\nSources:\n- [A](https://a.example)"
        );
        assert_eq!(
            streamed.concat(),
            text,
            "the persisted answer equals what was streamed"
        );
    }

    // ── Wiremock: PDFs ──────────────────────────────────────────────────

    fn pdf_message() -> Vec<ChatHistoryMessage> {
        vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                assistant_core::ContentBlock::Text("summarize this".into()),
                assistant_core::ContentBlock::Document {
                    media_type: "application/pdf".into(),
                    data: "JVBERi0=".into(),
                },
            ],
        }]
    }

    /// The body sent for `history` to a model with `inputs` in the catalog.
    async fn body_for(
        inputs: &[&str],
        options: OpenRouterOptions,
        history: &[ChatHistoryMessage],
    ) -> Value {
        let server = MockServer::start().await;
        mount_once(&server, chat_response("OK")).await;
        mount_catalog(
            &server,
            catalog_model(inputs, &["tools"]),
            json!([endpoint("Anthropic", "anthropic", 200_000)]),
        )
        .await;
        provider_with(&server.uri(), options)
            .chat("sys", history, &[])
            .await
            .unwrap();
        sent_body(&server).await
    }

    fn file_parser(engine: &str) -> Value {
        json!({"id": "file-parser", "pdf": {"engine": engine}})
    }

    #[tokio::test]
    async fn pdf_is_sent_as_a_file_part() {
        // Without the cache breakpoint Anthropic models put on the last part.
        let options = OpenRouterOptions {
            cache: assistant_core::types::llm::OpenRouterCache {
                enabled: false,
                ..Default::default()
            },
            ..OpenRouterOptions::default()
        };
        let body = body_for(&["text", "file"], options, &pdf_message()).await;

        let parts = body["messages"][1]["content"].as_array().unwrap();
        assert_eq!(parts[0]["type"], "text");
        assert_eq!(
            parts[1],
            json!({"type": "file", "file": {
                "filename": "document.pdf",
                "file_data": "data:application/pdf;base64,JVBERi0=",
            }})
        );
    }

    #[tokio::test]
    async fn native_file_input_needs_no_parser() {
        let body = body_for(
            &["text", "file"],
            OpenRouterOptions::default(),
            &pdf_message(),
        )
        .await;
        assert!(body.get("plugins").is_none(), "{body}");
    }

    #[tokio::test]
    async fn model_without_file_input_gets_cloudflare_ai() {
        let body = body_for(&["text"], OpenRouterOptions::default(), &pdf_message()).await;
        assert_eq!(body["plugins"], json!([file_parser("cloudflare-ai")]));
    }

    #[tokio::test]
    async fn configured_engine_is_used_for_any_model() {
        let options = OpenRouterOptions {
            pdf_engine: Some(assistant_core::types::llm::OpenRouterPdfEngine::MistralOcr),
            ..OpenRouterOptions::default()
        };
        let body = body_for(&["text", "file"], options, &pdf_message()).await;
        assert_eq!(body["plugins"], json!([file_parser("mistral-ocr")]));
    }

    #[tokio::test]
    async fn no_parser_without_a_pdf() {
        let options = OpenRouterOptions {
            pdf_engine: Some(assistant_core::types::llm::OpenRouterPdfEngine::MistralOcr),
            ..OpenRouterOptions::default()
        };
        let history = [ChatHistoryMessage::Text {
            role: assistant_core::ChatRole::User,
            content: "hi".into(),
        }];
        let body = body_for(&["text"], options, &history).await;
        assert!(body.get("plugins").is_none(), "{body}");
    }

    #[tokio::test]
    async fn parser_and_web_plugins_coexist() {
        let options = OpenRouterOptions {
            web_search: assistant_core::types::llm::OpenRouterWebSearch {
                enabled: true,
                ..Default::default()
            },
            ..OpenRouterOptions::default()
        };
        let body = body_for(&["text"], options, &pdf_message()).await;
        assert_eq!(
            body["plugins"],
            json!([{"id": "web"}, file_parser("cloudflare-ai")])
        );
    }

    #[tokio::test]
    async fn other_documents_are_not_sent() {
        let history = [ChatHistoryMessage::MultimodalUser {
            content: vec![
                assistant_core::ContentBlock::Text("read".into()),
                assistant_core::ContentBlock::Document {
                    media_type: "application/msword".into(),
                    data: "AAAA".into(),
                },
            ],
        }];
        let body = body_for(&["text", "file"], OpenRouterOptions::default(), &history).await;
        let parts = body["messages"][1]["content"].as_array().unwrap();
        assert_eq!(parts.len(), 1, "{parts:?}");
        assert!(body.get("plugins").is_none(), "{body}");
    }

    // ── Wiremock: embeddings ────────────────────────────────────────────

    /// Answers each embeddings request with one vector per input, `[i, len]`,
    /// listed in reverse order to check reordering by `index`.
    async fn mount_embeddings(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(|req: &wiremock::Request| {
                let body: Value = serde_json::from_slice(&req.body).unwrap();
                let count = body["input"].as_array().unwrap().len();
                let data: Vec<Value> = (0..count)
                    .rev()
                    .map(|i| json!({"object": "embedding", "index": i, "embedding": [i as f32, count as f32]}))
                    .collect();
                ResponseTemplate::new(200).set_body_json(json!({"object": "list", "data": data}))
            })
            .mount(server)
            .await;
    }

    async fn embedding_bodies(server: &MockServer) -> Vec<Value> {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == "/embeddings")
            .map(|r| serde_json::from_slice(&r.body).unwrap())
            .collect()
    }

    #[tokio::test]
    async fn embed_batch_sends_one_request_and_returns_input_order() {
        let server = MockServer::start().await;
        mount_embeddings(&server).await;

        let vectors = make_provider(&server.uri())
            .embed_batch(&["a", "b", "c"])
            .await
            .unwrap();

        assert_eq!(vectors, [vec![0.0, 3.0], vec![1.0, 3.0], vec![2.0, 3.0]]);
        let bodies = embedding_bodies(&server).await;
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            bodies[0],
            json!({"model": "openai/text-embedding-3-small", "input": ["a", "b", "c"], "encoding_format": "float"})
        );
    }

    #[tokio::test]
    async fn embed_returns_one_vector_with_the_configured_model() {
        let server = MockServer::start().await;
        mount_embeddings(&server).await;
        let cfg = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some(Secret::new("test-key")),
            base_url: server.uri(),
            embedding_model: Some("qwen/qwen3-embedding-8b".into()),
            ..LlmConfig::default()
        };

        let vector = OpenRouterProvider::from_llm_config(&cfg)
            .unwrap()
            .embed("hello")
            .await
            .unwrap();

        assert_eq!(vector, [0.0, 1.0]);
        assert_eq!(
            embedding_bodies(&server).await[0]["model"],
            "qwen/qwen3-embedding-8b"
        );
    }

    #[tokio::test]
    async fn batches_over_2048_texts_are_split() {
        let server = MockServer::start().await;
        mount_embeddings(&server).await;
        let texts: Vec<String> = (0..2050).map(|i| format!("t{i}")).collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();

        let vectors = make_provider(&server.uri())
            .embed_batch(&refs)
            .await
            .unwrap();

        assert_eq!(vectors.len(), 2050);
        assert_eq!(
            vectors[2049],
            [1.0, 2.0],
            "the second request's second vector"
        );
        let sizes: Vec<usize> = embedding_bodies(&server)
            .await
            .iter()
            .map(|b| b["input"].as_array().unwrap().len())
            .collect();
        assert_eq!(sizes, [2048, 2]);
    }

    #[tokio::test]
    async fn embedding_count_mismatch_is_an_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [
                {"index": 0, "embedding": [1.0]}
            ]})))
            .mount(&server)
            .await;

        let err = make_provider(&server.uri())
            .embed_batch(&["a", "b"])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("1") && err.contains("2"), "{err}");
    }

    #[tokio::test]
    async fn unknown_embedding_model_error_names_the_model() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({"error": {
                "code": 404, "message": "No endpoints found"
            }})))
            .expect(1)
            .mount(&server)
            .await;

        let err = make_provider(&server.uri()).embed("x").await.unwrap_err();
        let err = format!("{err:#}");
        assert!(err.contains("openai/text-embedding-3-small"), "{err}");
        assert!(err.contains("No endpoints found"), "{err}");
    }

    // ── Wiremock: OpenRouter SSE ────────────────────────────────────────

    async fn mount_sse(server: &MockServer, events: &[&str]) {
        let body: String = events.iter().map(|e| format!("{e}\n\n")).collect();
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
            .expect(1)
            .mount(server)
            .await;
    }

    fn drain(rx: &mut tokio::sync::mpsc::Receiver<StreamChunk>) -> Vec<String> {
        let mut texts = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            if let StreamChunk::Text(t) = chunk {
                texts.push(t);
            }
        }
        texts
    }

    #[tokio::test]
    async fn stream_skips_comments_and_reports_final_usage() {
        let server = MockServer::start().await;
        mount_sse(
            &server,
            &[
                ": OPENROUTER PROCESSING",
                r#"data: {"id":"gen-s","model":"openai/gpt-4o","choices":[{"delta":{"content":"Hel"}}]}"#,
                ": OPENROUTER PROCESSING",
                r#"data: {"id":"gen-s","model":"openai/gpt-4o","choices":[{"delta":{"content":"lo"},"finish_reason":"stop"}]}"#,
                r#"data: {"id":"gen-s","model":"openai/gpt-4o","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":2,"cost":0.0004}}"#,
                "data: [DONE]",
            ],
        )
        .await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        let response = make_provider(&server.uri())
            .chat_streaming("sys", &[], &[], Some(tx))
            .await
            .unwrap();

        let LlmResponse::FinalAnswer(text, meta) = response else {
            panic!("expected a final answer, got {response:?}");
        };
        assert_eq!(text, "Hello");
        assert_eq!(drain(&mut rx), ["Hel", "lo"], "comments produce no chunks");
        assert_eq!(meta.model.as_deref(), Some("openai/gpt-4o"));
        assert_eq!(meta.response_id.as_deref(), Some("gen-s"));
        assert_eq!(meta.finish_reason.as_deref(), Some("stop"));
        assert_eq!(meta.input_tokens, Some(7));
        assert_eq!(meta.output_tokens, Some(2));
        assert_eq!(meta.cost_usd, Some(0.0004));
    }

    #[tokio::test]
    async fn stream_request_sets_stream_true() {
        let server = MockServer::start().await;
        mount_sse(
            &server,
            &[
                r#"data: {"choices":[{"delta":{"content":"x"},"finish_reason":"stop"}]}"#,
                "data: [DONE]",
            ],
        )
        .await;

        make_provider(&server.uri())
            .chat_streaming("sys", &[], &[], None)
            .await
            .unwrap();

        assert_eq!(sent_body(&server).await["stream"], true);
    }

    #[tokio::test]
    async fn stream_accumulates_two_tool_calls_by_index() {
        let server = MockServer::start().await;
        mount_sse(
            &server,
            &[
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_a","function":{"name":"file-read","arguments":"{\"pa"}}]}}]}"#,
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"id":"call_b","function":{"name":"file-read","arguments":"{}"}}]}}]}"#,
                r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"th\":\"/a\"}"}}]},"finish_reason":"tool_calls"}]}"#,
                "data: [DONE]",
            ],
        )
        .await;

        let response = make_provider(&server.uri())
            .chat_streaming("sys", &[], &[read_tool()], None)
            .await
            .unwrap();

        let LlmResponse::ToolCalls(calls) = response else {
            panic!("expected tool calls, got {response:?}");
        };
        let ids: Vec<_> = calls.items.iter().map(|c| c.id.as_deref()).collect();
        assert_eq!(ids, [Some("call_a"), Some("call_b")]);
        assert_eq!(calls.items[0].params, json!({"path": "/a"}));
        assert_eq!(calls.items[1].params, json!({}));
    }

    #[tokio::test]
    async fn mid_stream_error_fails_the_call() {
        let server = MockServer::start().await;
        mount_sse(
            &server,
            &[
                r#"data: {"choices":[{"delta":{"content":"partial"}}]}"#,
                r#"data: {"error":{"code":"server_error","message":"Provider disconnected"},"choices":[{"delta":{"content":""},"finish_reason":"error"}]}"#,
            ],
        )
        .await;

        let err = make_provider(&server.uri())
            .chat_streaming("sys", &[], &[], None)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Provider disconnected"), "{err}");
    }

    #[tokio::test]
    async fn stream_ending_without_done_or_finish_fails() {
        let server = MockServer::start().await;
        mount_sse(
            &server,
            &[r#"data: {"choices":[{"delta":{"content":"cut"}}]}"#],
        )
        .await;

        let err = make_provider(&server.uri())
            .chat_streaming("sys", &[], &[], None)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("ended"), "{err}");
    }

    // ── Wiremock: error handling ─────────────────────────────────────────

    /// A provider whose retries back off for 10 ms instead of seconds.
    fn fast_retry_provider(server_uri: &str) -> OpenRouterProvider {
        let mut provider = make_provider(server_uri);
        provider.client.retry = crate::retry::RetryConfig {
            max_retries: 2,
            initial_delay_ms: 10,
            max_delay_ms: 5_000,
            jitter: false,
            ..crate::retry::RetryConfig::default()
        };
        provider
    }

    async fn mount_error(server: &MockServer, status: u16, body: Value, times: u64) {
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .up_to_n_times(times)
            .mount(server)
            .await;
    }

    async fn request_count(server: &MockServer) -> usize {
        chat_requests(server).await.len()
    }

    #[tokio::test]
    async fn rate_limit_waits_for_retry_after_then_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "1")
                    .set_body_json(json!({"error": {"code": 429, "message": "Rate limited"}})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        mount_once(&server, chat_response("OK")).await;

        let started = std::time::Instant::now();
        let response = fast_retry_provider(&server.uri())
            .chat("sys", &[], &[])
            .await
            .unwrap();

        assert!(matches!(response, LlmResponse::FinalAnswer(ref t, _) if t == "OK"));
        assert!(
            started.elapsed() >= std::time::Duration::from_secs(1),
            "waited only {:?}",
            started.elapsed()
        );
        assert_eq!(request_count(&server).await, 2);
    }

    #[tokio::test]
    async fn service_unavailable_is_retried() {
        let server = MockServer::start().await;
        mount_error(
            &server,
            503,
            json!({"error": {"code": 503, "message": "busy"}}),
            1,
        )
        .await;
        mount_once(&server, chat_response("OK")).await;

        fast_retry_provider(&server.uri())
            .chat("sys", &[], &[])
            .await
            .unwrap();
        assert_eq!(request_count(&server).await, 2);
    }

    #[tokio::test]
    async fn insufficient_credits_fails_at_once() {
        let server = MockServer::start().await;
        mount_error(
            &server,
            402,
            json!({"error": {"code": 402, "message": "Insufficient credits"}}),
            10,
        )
        .await;

        let err = fast_retry_provider(&server.uri())
            .chat("sys", &[], &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("lacks credits"), "{err}");
        assert!(err.contains("402"), "{err}");
        assert_eq!(request_count(&server).await, 1, "402 is not retried");
    }

    #[tokio::test]
    async fn moderation_error_names_the_reasons() {
        let server = MockServer::start().await;
        mount_error(
            &server,
            403,
            json!({"error": {
                "code": 403,
                "message": "Input was flagged",
                "metadata": {"reasons": ["violence", "harassment"], "flagged_input": "..."},
            }}),
            10,
        )
        .await;

        let err = fast_retry_provider(&server.uri())
            .chat("sys", &[], &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Input was flagged"), "{err}");
        assert!(err.contains("violence, harassment"), "{err}");
        assert_eq!(request_count(&server).await, 1);
    }

    #[tokio::test]
    async fn bad_request_is_not_retried_and_reports_error_type() {
        let server = MockServer::start().await;
        mount_error(
            &server,
            400,
            json!({"error": {
                "code": 400,
                "message": "Invalid tool schema",
                "metadata": {"error_type": "invalid_request"},
            }}),
            10,
        )
        .await;

        let err = fast_retry_provider(&server.uri())
            .chat("sys", &[], &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("400"), "{err}");
        assert!(err.contains("invalid_request"), "{err}");
        assert!(err.contains("Invalid tool schema"), "{err}");
        assert_eq!(request_count(&server).await, 1);
    }

    #[tokio::test]
    async fn ok_status_with_error_body_fails() {
        let server = MockServer::start().await;
        mount_once(
            &server,
            json!({"error": {"code": 502, "message": "Upstream provider failed"}}),
        )
        .await;

        let err = fast_retry_provider(&server.uri())
            .chat("sys", &[], &[])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Upstream provider failed"), "{err}");
    }

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
            api_key: Some(Secret::new(api_key)),
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
