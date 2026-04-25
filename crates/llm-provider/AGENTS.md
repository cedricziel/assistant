# AGENTS.md — LLM Provider Crate

Guidance for AI agents adding or modifying LLM providers in this crate.

## Architecture

All providers share a layered design:

```
assistant-core (trait)          LlmProvider trait + types
    │
assistant-llm-provider          Concrete implementations
    ├── chat_completions/       Shared Chat Completions client
    │   ├── mod.rs              ChatCompletionsProvider, ChatCompletionsConfig
    │   ├── messages.rs         build_chat_messages(), build_raw_messages()
    │   └── tools.rs            tool_spec_to_chat(), parse_tool_calls()
    ├── moonshot/               Moonshot AI (wraps ChatCompletionsProvider)
    ├── openrouter/             OpenRouter (wraps ChatCompletionsProvider)
    ├── anthropic/              Anthropic (native Messages API)
    ├── openai/                 OpenAI (Responses API)
    ├── ollama/                 Ollama (local, custom format)
    ├── http.rs                 Shared reqwest client with tracing middleware
    ├── retry.rs                Shared retry logic
    └── voyage/                 Voyage AI embeddings
```

## Adding an OpenAI-Compatible Provider

If the new provider speaks the OpenAI Chat Completions wire format:

1. **Create `<provider>/mod.rs`** with a struct wrapping `ChatCompletionsProvider`.
2. **Implement `from_llm_config()`** — resolve API key (config + env fallback),
   set default base URL, build `ChatCompletionsConfig`, inject extra headers.
3. **Implement `LlmProvider`** — delegate `chat()` / `chat_streaming()` to
   `inner.chat_non_streaming()` / `inner.chat_sse()`.
4. **Add types to `crates/core/src/types.rs`**:
   - New variant on `LlmProviderKind`
   - `<Provider>Options` struct (with `#[derive(Default)]`)
   - Field on `LlmConfig` (with `#[serde(default)]`)
   - Add to `Default for LlmConfig`
   - Re-export from `crates/core/src/lib.rs`
5. **Wire into `create_provider()`** match in `lib.rs`.
6. **Export** from `lib.rs`.
7. **Update CLI arg parsing** in `crates/web-ui/src/main.rs` (provider string match).
8. **Add config example** to `config.toml`.

## Adding a Non-Compatible Provider

If the provider has its own wire format (like Anthropic or a future Gemini):

- Build a standalone module with its own HTTP client logic.
- Use `crate::http::build_http_client()` for the traced reqwest client.
- Use `crate::retry::with_retry()` for transient error handling.
- Follow the same type registration steps (4-8 above).

## Testing Patterns

- **Unit tests**: Config construction, capabilities, error messages.
- **Wiremock tests**: Full HTTP round-trips with a mock server.
  - Mock responses must include all required fields (`id`, `object`, `created`,
    `model`, `choices`, `usage` with `total_tokens`).
  - SSE mocks use `set_body_raw(body, "text/event-stream")` — not
    `insert_header` + `set_body_string` (which sets `text/plain`).
- **Ignored integration test**: Live call with a cheap/free model, gated by
  `#[ignore = "requires <PROVIDER>_API_KEY"]`.

## Key Conventions

- `provider_name()` returns a lowercase slug matching the `LlmProviderKind` serde name.
- `embed()` returns `Err(...)` if the provider doesn't support embeddings.
  Users configure `[llm.embeddings]` separately.
- Extra headers use `ChatCompletionsConfig::extra_headers: Vec<(String, String)>`.
  The `ChatCompletionsProvider` converts them to `HeaderName`/`HeaderValue` and
  injects via `async-openai`'s `with_header()`.
- API key resolution: check `config.api_key` first, then env var fallback.
