# Gemini Provider

## Why

Google Gemini is the most significant gap in our LLM provider lineup. Both major competitors (OpenClaw and Hermes Agent) treat it as first-class. Gemini offers a generous free tier (no credit card required), 1M+ token context windows on 2.5/3.x models, native function calling with parallel and compositional support, vision, embeddings, and built-in Google Search grounding.

Adding Gemini directly addresses our biggest adoption barrier for cost-sensitive users and gives us access to Google's rapidly advancing model family.

## What Changes

### Gemini provider

A new `GeminiProvider` implementing `LlmProvider`, built with raw `reqwest` (like the Anthropic provider). Gemini uses its own wire format — `generateContent` / `streamGenerateContent` endpoints with `parts`-based messages and `functionDeclarations` — so it cannot reuse the Chat Completions base from the OpenRouter change.

Key mappings:

| Our trait                 | Gemini API                                                  |
| ------------------------- | ----------------------------------------------------------- |
| `chat()`                  | `POST /v1beta/models/{model}:generateContent`               |
| `chat_streaming()`        | `POST /v1beta/models/{model}:streamGenerateContent?alt=sse` |
| `embed()`                 | `POST /v1beta/models/{model}:embedContent`                  |
| `ToolSpec` → request      | `tools[].functionDeclarations[]`                            |
| response → `ToolCallItem` | `candidates[].content.parts[].functionCall`                 |
| tool result → request     | `parts[].functionResponse` with matching `id`               |

### Google Search grounding

Gemini offers built-in Google Search as a tool, similar to Anthropic's hosted `web_search`. When enabled, it maps to `HostedTool::WebSearch` so the runtime suppresses local web-search tools (same pattern as Anthropic).

### Embeddings

Gemini provides `text-embedding-005` via the `embedContent` endpoint. A new `EmbeddingProviderKind::Gemini` variant allows using Gemini embeddings independently of the LLM provider.

### Config

```toml
[llm]
provider = "gemini"
model = "gemini-2.5-flash"
# api_key = "AI..." or set GEMINI_API_KEY env var
```

Optional provider-specific config:

```toml
[llm.gemini]
google_search = true  # enable Google Search grounding (default: false)
```

Embedding config:

```toml
[llm.embeddings]
provider = "gemini"
model = "text-embedding-005"
```

## Capabilities

### New Capabilities

- **gemini-provider**: Full Google Gemini integration with chat, streaming, function calling, vision, and embeddings. Supports Gemini 2.5 and 3.x model families.
- **gemini-google-search**: Built-in Google Search grounding as a hosted tool, suppressing local web-search equivalents when enabled.
- **gemini-embeddings**: Dedicated embedding provider using Gemini's `text-embedding-005` model.

## Impact

### Rust crates

- `crates/core/src/types.rs` — add `Gemini` to `LlmProviderKind`, add `Gemini` to `EmbeddingProviderKind`, add `GeminiOptions` struct, add field to `LlmConfig`
- `crates/llm-provider/src/lib.rs` — add `pub mod gemini`, update `create_provider()` match
- `crates/llm-provider/src/gemini/mod.rs` — **new**: full provider (~600 lines, reqwest-based)
  - `GeminiProvider` struct and config
  - `build_gemini_contents()` — `ChatHistoryMessage[]` → Gemini `contents[]` format
  - `build_gemini_tools()` — `ToolSpec[]` → `functionDeclarations[]`
  - `parse_gemini_response()` — `candidates[]` → `LlmResponse`
  - SSE streaming parser for `streamGenerateContent`
  - `embed()` implementation via `embedContent` endpoint
- `crates/llm-provider/Cargo.toml` — no new dependencies (uses existing `reqwest`, `serde_json`, `tokio`, `futures`)

### Documentation

- `config.toml` example — add Gemini provider section
- User-facing docs — document Gemini setup, model selection, Google Search grounding, and embedding configuration

## Non-goals

- **Vertex AI authentication** — only API key auth (`x-goog-api-key` header) is supported initially. Google Cloud Vertex AI (with gcloud/ADC auth) is a possible future addition.
- **Gemini Live API** — real-time bidirectional streaming is out of scope.
- **Code execution tool** — Gemini offers a hosted code execution tool. Not wired up in the initial integration.
- **Context caching** — Gemini's context caching API is a cost optimization we can add later.
- **Multimodal function responses** — Gemini 3 supports returning images in `functionResponse.parts`. Initial implementation returns text-only function results.

## User-facing documentation

Yes. A new section in docs covering Gemini provider setup, API key acquisition (free tier), model selection, Google Search grounding opt-in, and embedding configuration.
