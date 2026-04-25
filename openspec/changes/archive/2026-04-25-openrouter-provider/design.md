# Design: OpenRouter Provider

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CHAT COMPLETIONS EXTRACTION                              │
│                                                                             │
│  BEFORE                                                                     │
│  ──────                                                                     │
│  moonshot/mod.rs (870 lines)                                               │
│    ├── Generic Chat Completions logic (~400 lines)                          │
│    └── Moonshot-specific $web_search (~170 lines)                          │
│                                                                             │
│  AFTER                                                                      │
│  ─────                                                                      │
│  chat_completions/                                                          │
│    ├── mod.rs ............. re-exports, ChatCompletionsProvider struct       │
│    ├── messages.rs ........ build_chat_messages(), build_raw_messages()     │
│    ├── tools.rs ........... tool_spec_to_chat(), parse_tool_calls()         │
│    └── streaming.rs ....... SSE stream accumulator (chat_sse logic)         │
│                                                                             │
│  moonshot/                                                                  │
│    └── mod.rs ............. MoonshotProvider delegates to                    │
│                             ChatCompletionsProvider + $web_search           │
│                                                                             │
│  openrouter/                                                                │
│    └── mod.rs ............. OpenRouterProvider delegates to                  │
│                             ChatCompletionsProvider + custom headers        │
│                                                                             │
│                                                                             │
│            ┌──────────────────────────────┐                                 │
│            │  ChatCompletionsProvider     │                                 │
│            │  (async-openai client)       │                                 │
│            │                              │                                 │
│            │  chat_non_streaming()        │                                 │
│            │  chat_sse()                  │                                 │
│            │  build_chat_messages()       │                                 │
│            │  build_raw_messages()        │                                 │
│            │  tool_spec_to_chat()         │                                 │
│            │  parse_tool_calls()          │                                 │
│            │  extract_chat_meta()         │                                 │
│            └──────────┬──────────────────┘                                 │
│                       │                                                     │
│            ┌──────────┼──────────────┐                                     │
│            │          │              │                                      │
│       ┌────┴────┐ ┌──┴───────┐ ┌───┴──────────┐                          │
│       │Moonshot │ │OpenRouter│ │ Future       │                           │
│       │         │ │          │ │ (Groq, etc.) │                           │
│       │+web_srch│ │+headers  │ │ +config only │                           │
│       └─────────┘ └──────────┘ └──────────────┘                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Key Type Changes

### ChatCompletionsConfig

```rust
/// Shared configuration for any OpenAI Chat Completions-compatible provider.
pub struct ChatCompletionsConfig {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    /// Extra HTTP headers injected into every request.
    /// Used by OpenRouter for HTTP-Referer and X-Title.
    pub extra_headers: Vec<(String, String)>,
}
```

### ChatCompletionsProvider

```rust
/// Reusable Chat Completions client.
///
/// Handles message conversion, tool-spec mapping, SSE streaming, response
/// parsing. Provider-specific wrappers compose this with their own config
/// and capabilities.
pub struct ChatCompletionsProvider {
    client: Client<AsyncOpenAIConfig>,
    config: ChatCompletionsConfig,
    http_client: reqwest_middleware::ClientWithMiddleware,
}

impl ChatCompletionsProvider {
    pub fn new(config: ChatCompletionsConfig) -> anyhow::Result<Self>;

    pub async fn chat_non_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse>;

    pub async fn chat_sse(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse>;

    /// Raw-HTTP chat for providers that need non-standard request bodies
    /// (e.g. Moonshot $web_search with builtin_function tools).
    pub async fn chat_raw_http(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value>;
}
```

### OpenRouterProvider

```rust
/// LlmProvider backed by OpenRouter's Chat Completions API.
pub struct OpenRouterProvider {
    inner: ChatCompletionsProvider,
}

impl OpenRouterProvider {
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        let api_key = cfg.api_key.clone()
            .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!(
                "OpenRouter API key not found. Set api_key in [llm] config \
                 or OPENROUTER_API_KEY environment variable."
            ))?;

        let base_url = if cfg.base_url == "http://localhost:11434" {
            "https://openrouter.ai/api/v1".to_string()
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

        let config = ChatCompletionsConfig {
            model: cfg.model.clone(),
            base_url,
            api_key,
            timeout_secs: cfg.timeout_secs,
            max_tokens: cfg.openrouter.max_tokens.unwrap_or(8192),
            extra_headers,
        };

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
            vision: true,  // model-dependent, but safe to declare
            hosted_tools: vec![],
        }
    }

    async fn chat(&self, system: &str, history: &[ChatHistoryMessage], tools: &[ToolSpec])
        -> anyhow::Result<LlmResponse>
    {
        self.inner.chat_non_streaming(system, history, tools).await
    }

    async fn chat_streaming(&self, system: &str, history: &[ChatHistoryMessage],
        tools: &[ToolSpec], chunk_sink: Option<mpsc::Sender<StreamChunk>>)
        -> anyhow::Result<LlmResponse>
    {
        self.inner.chat_sse(system, history, tools, chunk_sink).await
    }

    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!("OpenRouter does not support embeddings"))
    }

    fn provider_name(&self) -> &str { "openrouter" }
    fn model_name(&self) -> &str { &self.inner.config.model }
    fn server_address(&self) -> &str { &self.inner.config.base_url }
}
```

### MoonshotProvider refactor

```rust
/// Moonshot provider — refactored to delegate to ChatCompletionsProvider.
pub struct MoonshotProvider {
    inner: ChatCompletionsProvider,
    web_search_enabled: bool,
}

#[async_trait]
impl LlmProvider for MoonshotProvider {
    async fn chat(&self, system: &str, history: &[ChatHistoryMessage], tools: &[ToolSpec])
        -> anyhow::Result<LlmResponse>
    {
        if self.web_search_enabled {
            self.chat_with_web_search(system, history, tools).await
        } else {
            self.inner.chat_non_streaming(system, history, tools).await
        }
    }

    async fn chat_streaming(&self, system: &str, history: &[ChatHistoryMessage],
        tools: &[ToolSpec], chunk_sink: Option<mpsc::Sender<StreamChunk>>)
        -> anyhow::Result<LlmResponse>
    {
        if self.web_search_enabled {
            // $web_search echo-back not compatible with SSE
            let result = self.chat_with_web_search(system, history, tools).await?;
            if let LlmResponse::FinalAnswer(ref text, _) = result
                && let Some(sink) = chunk_sink
            {
                let _ = sink.send(StreamChunk::Text(text.clone())).await;
            }
            Ok(result)
        } else {
            self.inner.chat_sse(system, history, tools, chunk_sink).await
        }
    }

    // ... rest unchanged
}

impl MoonshotProvider {
    /// $web_search echo-back uses inner.chat_raw_http() for the HTTP calls
    /// but manages the loop and builtin_function injection itself.
    async fn chat_with_web_search(&self, ...) -> anyhow::Result<LlmResponse> {
        // Same logic as today, but uses self.inner.chat_raw_http()
        // for individual HTTP calls instead of managing its own client.
    }
}
```

## Config Types

### OpenRouterOptions

```rust
/// Provider-specific OpenRouter options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenRouterOptions {
    /// `HTTP-Referer` header (required by OpenRouter TOS for rankings).
    pub referer: Option<String>,
    /// `X-Title` header (shown in OpenRouter dashboard).
    pub title: Option<String>,
    /// Max output tokens (default: 8192).
    pub max_tokens: Option<u32>,
}
```

Added to `LlmConfig`:

```rust
pub struct LlmConfig {
    // ... existing fields ...
    #[serde(default)]
    pub openrouter: OpenRouterOptions,
}
```

## Extra Headers in async-openai

The `async-openai` crate supports custom headers via `AsyncOpenAIConfig::with_headers()`. The `ChatCompletionsProvider` constructor merges `extra_headers` into the config:

```rust
let mut oai_cfg = AsyncOpenAIConfig::new()
    .with_api_key(&config.api_key)
    .with_api_base(&config.base_url);

if !config.extra_headers.is_empty() {
    let mut headers = reqwest::header::HeaderMap::new();
    for (key, value) in &config.extra_headers {
        headers.insert(
            reqwest::header::HeaderName::from_bytes(key.as_bytes())?,
            reqwest::header::HeaderValue::from_str(value)?,
        );
    }
    oai_cfg = oai_cfg.with_headers(headers);
}
```

## Testing Strategy

- **Unit tests**: All existing Moonshot tests must pass after refactor (they exercise the same Chat Completions logic, now through `ChatCompletionsProvider`).
- **OpenRouter wiremock tests**: Stand up a mock server, verify correct headers (`HTTP-Referer`, `X-Title`, `Authorization`), correct request format, and correct response parsing.
- **Streaming test**: Verify SSE stream accumulation works through the OpenRouter wrapper.
- **Config test**: Verify `LlmProviderKind::OpenRouter` deserializes from TOML and creates a provider via `create_provider()`.
- **Integration test** (ignored, requires key): Live call to OpenRouter with a cheap model.

## Migration / Backwards Compatibility

- The Moonshot refactor is internal — no config changes, no behavior changes for existing Moonshot users.
- `LlmProviderKind::OpenRouter` is a new enum variant. Existing configs with `provider = "ollama"` / `"anthropic"` / `"openai"` / `"moonshot"` are unaffected.
- No database migrations.
