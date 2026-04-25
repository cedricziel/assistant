# Tasks: OpenRouter Provider

## Phase 1: Extract Chat Completions Base

- [x] **1.1** Create `crates/llm-provider/src/chat_completions/mod.rs` with `ChatCompletionsConfig` and `ChatCompletionsProvider` structs
- [x] **1.2** Extract `build_chat_messages()` and `build_raw_messages()` from `moonshot/mod.rs` into `chat_completions/messages.rs`
- [x] **1.3** Extract `tool_spec_to_chat()`, `parse_tool_calls()`, `extract_chat_meta()` into `chat_completions/tools.rs`
- [x] **1.4** Extract `chat_non_streaming()` logic into `ChatCompletionsProvider::chat_non_streaming()`
- [x] **1.5** Extract `chat_sse()` logic into `ChatCompletionsProvider::chat_sse()`
- [x] **1.6** Add `chat_raw_http()` helper to `ChatCompletionsProvider` for raw HTTP requests (used by Moonshot `$web_search`)
- [x] **1.7** Add `extra_headers` support to `ChatCompletionsConfig` — merge into `async-openai` client config on construction
- [x] **1.8** Move all existing `moonshot/mod.rs` unit tests for message building, tool conversion, and meta extraction into `chat_completions/` tests
- [x] **1.9** Verify all moved tests pass: `cargo test -p assistant-llm-provider`

## Phase 2: Refactor Moonshot to Use Base

- [x] **2.1** Rewrite `MoonshotProvider` to hold a `ChatCompletionsProvider` inner field, delegate `chat_non_streaming()` and `chat_sse()` to it
- [x] **2.2** Keep `chat_with_web_search()` on `MoonshotProvider` — use `inner.chat_raw_http()` for individual HTTP calls
- [x] **2.3** Keep `from_llm_config()` on `MoonshotProvider` — constructs `ChatCompletionsConfig` and passes to `ChatCompletionsProvider::new()`
- [x] **2.4** Verify all existing Moonshot wiremock tests pass unchanged: `cargo test -p assistant-llm-provider moonshot`
- [x] **2.5** Verify full workspace builds: `cargo check --workspace`

## Phase 3: Add OpenRouter Provider

- [x] **3.1** Add `OpenRouter` variant to `LlmProviderKind` in `crates/core/src/types.rs`
- [x] **3.2** Add `OpenRouterOptions` struct to `crates/core/src/types.rs` (fields: `referer`, `title`, `max_tokens`)
- [x] **3.3** Add `openrouter: OpenRouterOptions` field to `LlmConfig`
- [x] **3.4** Create `crates/llm-provider/src/openrouter/mod.rs` with `OpenRouterProvider` wrapping `ChatCompletionsProvider`
- [x] **3.5** Implement `from_llm_config()`: resolve `OPENROUTER_API_KEY`, set base URL default to `https://openrouter.ai/api/v1`, build `extra_headers` from `OpenRouterOptions`
- [x] **3.6** Implement `LlmProvider` trait — delegate `chat()` and `chat_streaming()` to inner, return error for `embed()`
- [x] **3.7** Register in `create_provider()` match in `crates/llm-provider/src/lib.rs`
- [x] **3.8** Export from `crates/llm-provider/src/lib.rs`

## Phase 4: Tests

- [x] **4.1** Add config construction test: `OpenRouterOptions` fields applied to provider
- [x] **4.2** Add `create_provider()` factory test for OpenRouter (same pattern as existing factory tests)
- [x] **4.3** Add wiremock test: non-streaming chat round-trip with correct `Authorization`, `HTTP-Referer`, and `X-Title` headers
- [x] **4.4** Add wiremock test: streaming SSE round-trip through OpenRouter wrapper
- [x] **4.5** Add wiremock test: tool calling request/response through OpenRouter (non-streaming + streaming)
- [x] **4.6** Add test: missing API key produces clear error message
- [x] **4.7** Add ignored integration test: live call to OpenRouter with a cheap model (requires `OPENROUTER_API_KEY`)

## Phase 5: Documentation & Polish

- [x] **5.1** Add OpenRouter section to `config.toml` example with commented-out configuration
- [x] **5.2** Add `AGENTS.md` to `crates/llm-provider/` for future provider work guidance
- [x] **5.3** Run `make lint && make format` — fix any warnings
- [x] **5.4** Run full test suite: `make test`
