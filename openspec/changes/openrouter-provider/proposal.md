# OpenRouter Provider

## Why

We support four LLM providers (Anthropic, OpenAI, Ollama, Moonshot). Competing projects like OpenClaw (30+) and Hermes Agent (28+) support far more. Users who want to use DeepSeek, Mistral, Llama, or any of 300+ models have no first-class path — they must informally override the OpenAI provider's `base_url`, which is undocumented and fragile since we migrated to the Responses API.

OpenRouter is a unified gateway that speaks the OpenAI Chat Completions wire format and routes to 300+ models from dozens of providers. A single integration closes the breadth gap without maintaining 20 separate providers.

## What Changes

### Shared Chat Completions base

The Moonshot provider already implements the full OpenAI Chat Completions protocol via `async-openai`. All of the message conversion, tool-spec mapping, SSE streaming accumulation, and response parsing is generic — only the `$web_search` echo-back loop and config resolution are Moonshot-specific.

Extract the generic Chat Completions machinery into a shared `chat_completions` module within `crates/llm-provider/`. Moonshot becomes a thin wrapper that adds its `$web_search` logic on top. OpenRouter becomes another thin wrapper that adds custom HTTP headers and its own config defaults.

### OpenRouter provider

A new `OpenRouterProvider` that delegates to the shared Chat Completions base with:

- Base URL: `https://openrouter.ai/api/v1`
- Auth: `OPENROUTER_API_KEY` env var or `api_key` in config
- Extra headers: `HTTP-Referer` and `X-Title` (required by OpenRouter TOS)
- Model format: pass-through (user specifies `anthropic/claude-sonnet-4-20250514`, `deepseek/deepseek-r1`, etc.)
- No embeddings (OpenRouter is chat-only)

### Config

```toml
[llm]
provider = "openrouter"
model = "anthropic/claude-sonnet-4-20250514"
# api_key = "sk-or-..." or set OPENROUTER_API_KEY env var
```

Optional provider-specific config:

```toml
[llm.openrouter]
referer = "https://my-app.example.com"  # HTTP-Referer header
title = "My Assistant"                   # X-Title header
```

## Capabilities

### New Capabilities

- **openrouter-provider**: Access 300+ models from dozens of providers through a single configuration. Any model available on OpenRouter can be used by setting `provider = "openrouter"` and the appropriate `model` identifier.

### Modified Capabilities

- **chat-completions-base**: The existing Moonshot Chat Completions implementation is extracted into a reusable module. This makes future OpenAI-compatible provider integrations trivial (Groq, Together, DeepSeek direct, etc. become ~50-line wrappers).

## Impact

### Rust crates

- `crates/core/src/types.rs` — add `OpenRouter` to `LlmProviderKind`, add `OpenRouterOptions` struct, add field to `LlmConfig`
- `crates/llm-provider/src/lib.rs` — add `pub mod chat_completions`, `pub mod openrouter`, update `create_provider()` match
- `crates/llm-provider/src/chat_completions/` — **new module**: extracted from `moonshot/mod.rs` (message builders, tool conversion, streaming accumulator, response parsing)
- `crates/llm-provider/src/openrouter/mod.rs` — **new**: thin wrapper over `chat_completions` base
- `crates/llm-provider/src/moonshot/mod.rs` — **refactor**: delegate to `chat_completions` base, keep only `$web_search` logic

### Documentation

- `config.toml` example — add OpenRouter provider section
- User-facing docs — document OpenRouter setup and model selection

## Non-goals

- **Model validation** — we will not validate model names against OpenRouter's model list. Pass-through is simpler, always current, and lets users access new models immediately.
- **Provider routing preferences** — OpenRouter supports routing preferences (cheapest, fastest). This is out of scope for the initial integration but could be added to `OpenRouterOptions` later.
- **Embeddings via OpenRouter** — OpenRouter is chat-only. Users needing embeddings should configure a separate `[llm.embeddings]` provider.
- **Other OpenAI-compatible providers as named variants** — Groq, Together, etc. could be added as thin wrappers over the same base in the future, but are not part of this change. Users can reach them via OpenRouter.

## User-facing documentation

Yes. A new section in docs covering OpenRouter provider setup, model selection syntax, and API key configuration.
