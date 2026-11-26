# OpenRouter Provider

Connects the assistant to [OpenRouter](https://openrouter.ai/), a unified
API gateway providing access to 300+ models from Anthropic, OpenAI, Meta,
Google, Mistral, and many more — all through a single API key and the
OpenAI Chat Completions wire format.

## Quick start

```sh
# Set your API key (or put it in org.toml)
export OPENROUTER_API_KEY=sk-or-...

# Edit ~/.assistant/orgs/default/org.toml
mkdir -p ~/.assistant/orgs/default
cat <<'EOF' >> ~/.assistant/orgs/default/org.toml
[llm]
provider = "openrouter"
model    = "anthropic/claude-sonnet-4-20250514"
EOF

# Run the assistant
cargo run -p assistant-cli
```

## Configuration

All OpenRouter settings live under the `[llm]` and `[llm.openrouter]`
sections in `~/.assistant/orgs/default/org.toml`.

### `[llm]` section

| Key              | Env var              | Default                          | Description                                   |
| ---------------- | -------------------- | -------------------------------- | --------------------------------------------- |
| `provider`       |                      | `"ollama"`                       | Set to `"openrouter"` to use this provider    |
| `model`          |                      | _(required)_                     | OpenRouter model slug (see [Models](#models)) |
| `api_key`        | `OPENROUTER_API_KEY` | _(required)_                     | API key; env var is checked as fallback       |
| `base_url`       |                      | `"https://openrouter.ai/api/v1"` | API base URL                                  |
| `max_iterations` |                      | `80`                             | Max tool-calling iterations per turn          |
| `timeout_secs`   |                      | `120`                            | Timeout for each LLM call (seconds)           |

### `[llm.openrouter]` section

| Key               | Default  | Description                                                                                        |
| ----------------- | -------- | -------------------------------------------------------------------------------------------------- |
| `referer`         | _(none)_ | `HTTP-Referer` header, used by OpenRouter for app attribution                                      |
| `title`           | _(none)_ | `X-OpenRouter-Title` header, shown in the OpenRouter dashboard                                     |
| `categories`      | `[]`     | `X-OpenRouter-Categories` header, comma-joined. At most two; more fails config load                |
| `max_tokens`      | `8192`   | Maximum completion tokens per response, sent as `max_completion_tokens`                            |
| `fallback_models` | `[]`     | Models to try in order when `model` fails. Sent as `models` with `model` first; omitted when empty |
| `pdf_engine`      | unset    | PDF parser: `"native"`, `"cloudflare-ai"` or `"mistral-ocr"`. See [PDFs](#pdfs)                    |

### `[llm.openrouter.provider]` section

Provider routing preferences, sent as the request's `provider` object.
Only the keys you set are sent. An unknown key fails config load.

| Key                  | Type                                              | Description                                           |
| -------------------- | ------------------------------------------------- | ----------------------------------------------------- |
| `order`              | list of provider slugs                            | Providers to try, in this order                       |
| `only`               | list of provider slugs                            | Use only these providers                              |
| `ignore`             | list of provider slugs                            | Never use these providers                             |
| `allow_fallbacks`    | boolean                                           | Whether OpenRouter may fall back to other providers   |
| `require_parameters` | boolean                                           | Use only providers that support every request field   |
| `data_collection`    | `"allow"` or `"deny"`                             | Whether providers that store or train on data qualify |
| `zdr`                | boolean                                           | Use only zero-data-retention endpoints                |
| `sort`               | `"price"`, `"throughput"` or `"latency"`          | How to rank providers                                 |
| `quantizations`      | list                                              | Accepted quantization levels                          |
| `max_price`          | table: `prompt`, `completion`, `request`, `image` | Highest acceptable prices; unknown keys fail load     |

### `[llm.openrouter.reasoning]` section

Reasoning settings, sent as the request's `reasoning` object. Only the
keys you set are sent. An unknown key fails config load.

| Key          | Type                                                                       | Description                            |
| ------------ | -------------------------------------------------------------------------- | -------------------------------------- |
| `effort`     | `"none"`, `"minimal"`, `"low"`, `"medium"`, `"high"`, `"xhigh"` or `"max"` | How hard the model reasons             |
| `max_tokens` | integer                                                                    | Token budget for reasoning             |
| `exclude`    | boolean                                                                    | Reason without returning the reasoning |
| `enabled`    | boolean                                                                    | Turn reasoning on or off               |

Setting both `effort` and `max_tokens` fails config load with
`` `effort` and `max_tokens` cannot both be set ``.

When `effort` is set, the provider checks it against the model's entry
in the catalog (see [Capabilities](#capabilities)). If the model can't
take that effort, the provider logs the reason followed by
`; chat calls will fail`, and every chat call fails with that reason
without contacting OpenRouter. See
[Troubleshooting](#troubleshooting) for the messages.

### `[llm.openrouter.cache]` section

Prompt caching settings. An unknown key fails config load. See
[Prompt caching](#prompt-caching).

| Key       | Default | Description                                                               |
| --------- | ------- | ------------------------------------------------------------------------- |
| `enabled` | `true`  | Send `cache_control` breakpoints. `false` sends none                      |
| `ttl`     | `"5m"`  | Cache lifetime, `"5m"` or `"1h"`. `"1h"` applies to Anthropic models only |

### `[llm.openrouter.web_search]` section

OpenRouter's `web` plugin. Only set keys are sent. An unknown key fails
config load. See [Web search](#web-search).

| Key                   | Default | Description                                                                                      |
| --------------------- | ------- | ------------------------------------------------------------------------------------------------ |
| `enabled`             | `false` | Send the `web` plugin                                                                            |
| `engine`              | unset   | `"native"`, `"exa"`, `"firecrawl"`, `"parallel"` or `"perplexity"`. Unset lets OpenRouter choose |
| `max_results`         | unset   | Maximum search results                                                                           |
| `search_prompt`       | unset   | Prompt that introduces the results to the model                                                  |
| `include_domains`     | unset   | Domains to search                                                                                |
| `exclude_domains`     | unset   | Domains to skip                                                                                  |
| `search_context_size` | unset   | `"low"`, `"medium"` or `"high"`. Needs `engine = "native"` or no `engine`                        |

Which domain filters each engine honours is up to OpenRouter; see
[OpenRouter's web search docs](https://openrouter.ai/docs/features/web-search).

## Web search

With `enabled = true`, every request carries
`plugins: [{"id": "web", ...}]` with the options you set.
`search_context_size` goes in the top-level
`web_search_options: {"search_context_size": ...}` instead. The provider
reports web search as a hosted tool, so the model is not offered the
local `web-search` tool. With `enabled = false`, no plugin is sent and
the local tool stays available.

When a final answer carries `url_citation` annotations, it ends with a
`Sources:` list. Each cited URL appears once, in order of first
citation, as a markdown link labelled with its title, or the URL when
there is no title. Brackets in titles are escaped. URLs the answer
already links inline are not listed again. Responses that call tools
get no list. When streaming, the list is the last text chunk, so the
saved answer matches what was streamed.

## PDFs

A PDF attachment goes in the user message as a `file` part:

```json
{
  "type": "file",
  "file": {
    "filename": "document.pdf",
    "file_data": "data:application/pdf;base64,..."
  }
}
```

Attachments carry no filename, so it is always `document.pdf`. Other
document types are not sent.

A request with a PDF carries
`plugins: [{"id": "file-parser", "pdf": {"engine": ...}}]`:

- With `pdf_engine` set, that engine is used for every model. Use
  `"mistral-ocr"` for scanned PDFs.
- With `pdf_engine` unset, a model whose catalog entry lists `file` in
  its input modalities reads the PDF itself and gets no plugin. Other
  models get `"cloudflare-ai"`. Until the catalog loads, or when it
  can't be reached, every model counts as having no `file` input.

Requests without a PDF get no parser plugin. With web search on, both
plugins are sent, `web` first.

## Request identity

During a conversation turn, each request carries:

- `session_id`: the conversation ID.
- `user`: a SHA-256 hash of the org-scoped user ID, truncated to 32 hex
  characters. The raw user ID and email are never sent.

Background calls made outside a turn send neither field.

## Prompt caching

The model's author is the slug prefix before `/`. Only `anthropic`,
`google` and `qwen` models need explicit breakpoints. For them the
provider sends the system prompt as a text part with
`cache_control: {"type": "ephemeral"}`. Anthropic models add
`"ttl": "1h"` when `ttl = "1h"`; Google and Qwen caches always last 5
minutes, so they get no `ttl`. Anthropic models also get the breakpoint on the last part
of the last user message. Other models get no `cache_control`, because
their upstreams cache automatically.

Every request in a conversation carries the same `session_id` (see
[Request identity](#request-identity)), which keeps it on the same
upstream so the cache can hit.

Usage reports cache reads as cached input tokens, from
`prompt_tokens_details.cached_tokens`, and cache writes as cache-write
input tokens, from `prompt_tokens_details.cache_write_tokens`.

## Errors and retries

Responses with status 408, 429, 500, 502, 503, 504, 524 or 529 are
retried with exponential backoff, up to 5 retries by default. When
OpenRouter sends `Retry-After`, the wait is at least that long. A
`Retry-After` above the 60-second retry cap ends retries at once.

Status 400, 401, 402, 403, 404 and 413 fail without a retry.

Error messages include the HTTP status and, when present,
OpenRouter's `metadata.error_type`. A 200 response that carries only an
`error` object also fails.

When streaming, the provider ignores keep-alive comments. An error in the
middle of a stream fails the call. A stream that ends without `[DONE]` or
a finish reason fails instead of returning partial text.

## Reasoning

### Output

Reasoning entries of type `reasoning.text` and `reasoning.summary` are
shown as thinking. Encrypted entries are never shown. A response without
`reasoning_details` falls back to its plain `reasoning` text. With
`exclude = true`, no thinking is shown.

- Streaming: thinking is streamed as it arrives.
- Non-streaming, tool calls: the thinking is attached to the tool calls.
- Non-streaming, final answer: the thinking is dropped, because a final
  answer has no slot for it.

A response that stops at the token limit (`finish_reason: "length"`)
with no answer text and no tool call fails. See
[Troubleshooting](#openrouter-stopped-at-the-token-budget-before-answering).

### Continuity

When a tool-call response carries `reasoning_details`, they are stored
unchanged with that assistant message, including encrypted entries and
signatures. Streamed pieces are merged by their `index`. Later requests
in the conversation send the details back on that message, in their
original order. This holds after a server restart, because history is
rebuilt from the database.

Details are sent back only when they were stored by OpenRouter for the
model configured now. Switching provider or model drops them.

## Authentication

Obtain an API key from [openrouter.ai/keys](https://openrouter.ai/keys)
and set it via environment variable or config file:

```sh
# Environment variable (recommended — avoids secrets in config files)
export OPENROUTER_API_KEY=sk-or-...

# Or in org.toml
[llm]
provider = "openrouter"
api_key  = "sk-or-..."
```

The provider checks `api_key` in the config first, then falls back to
the `OPENROUTER_API_KEY` environment variable.

## Capabilities

The provider reads what the configured model can do from OpenRouter's
model catalog. When the provider is built, it fetches `{base_url}/models`
and `{base_url}/models/{model}/endpoints` in the background. The first
chat call waits for that fetch for at most 5 seconds.

| Capability     | Value                                                                                                                                                                                                                              |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tool calling   | Native when the model lists `tools` in `supported_parameters`, else none                                                                                                                                                           |
| Streaming      | Yes                                                                                                                                                                                                                                |
| Vision         | Yes when the model lists `image` in its input modalities; otherwise each image is replaced by a short text note                                                                                                                    |
| PDF input      | Native when the model lists `file` in its input modalities; otherwise parsed by the `file-parser` plugin. See [PDFs](#pdfs)                                                                                                        |
| Context window | Smallest `context_length` among the endpoints routing can select                                                                                                                                                                   |
| Chat options   | `tool_choice`, `parallel_tool_calls` and JSON schemas are sent natively when the model lists them in `supported_parameters`, with `provider.require_parameters = true`; otherwise `tool_choice` falls back to a prompt instruction |
| Embeddings     | Yes, through OpenRouter's embeddings endpoint                                                                                                                                                                                      |

History compaction reads the context window on every turn.
`[llm.openrouter.provider]` decides which endpoints count:

- `ignore` removes the named providers.
- `only` keeps only the named providers.
- `order` keeps only the named providers when `allow_fallbacks = false`.

A slug names an endpoint when it equals the endpoint's tag, is the start
of a tag variant (`deepinfra` matches `deepinfra/fp8`), or equals its
provider name. Without endpoint data, the model's own `context_length`
is used.

Until the catalog loads, or when it can't be reached, the provider
reports native tools, no vision, and an unknown context window. A failed
fetch is retried on a chat call, no sooner than 10 minutes later. Once a
fetch succeeds, the catalog is not fetched again for the life of the
provider.

Embeddings for vector search use OpenRouter's embeddings endpoint with
`[llm] embedding_model`, or `openai/text-embedding-3-small` when it is unset.

## Models

OpenRouter provides access to 300+ models. Use the `provider/model`
slug format. Browse the full catalog at
[openrouter.ai/models](https://openrouter.ai/models).

### Popular choices

| Model slug                           | Provider  | Notes                        |
| ------------------------------------ | --------- | ---------------------------- |
| `anthropic/claude-sonnet-4-20250514` | Anthropic | Claude Sonnet 4, recommended |
| `anthropic/claude-opus-4-20250514`   | Anthropic | Claude Opus 4, most capable  |
| `openai/gpt-4o`                      | OpenAI    | GPT-4o                       |
| `openai/o3-mini`                     | OpenAI    | Reasoning model              |
| `google/gemini-2.5-pro-preview`      | Google    | Gemini 2.5 Pro               |
| `meta-llama/llama-3.3-70b-instruct`  | Meta      | Llama 3.3 70B                |
| `mistralai/mistral-large-latest`     | Mistral   | Mistral Large                |

### Free models

OpenRouter offers several free-tier models (rate-limited):

| Model slug                              | Notes             |
| --------------------------------------- | ----------------- |
| `meta-llama/llama-3.3-8b-instruct:free` | Llama 3.3 8B free |
| `mistralai/mistral-7b-instruct:free`    | Mistral 7B free   |
| `google/gemma-2-9b-it:free`             | Gemma 2 9B free   |

## OpenRouter vs direct providers

You can reach Claude and GPT through OpenRouter, but the **direct
providers** (Anthropic, OpenAI) offer provider-specific features that
OpenRouter does not expose:

| Feature                       | Direct | OpenRouter |
| ----------------------------- | ------ | ---------- |
| Tool calling                  | Yes    | Yes        |
| Streaming                     | Yes    | Yes        |
| Vision                        | Yes    | Yes        |
| Anthropic hosted web search   | Yes    | No         |
| Anthropic hosted web fetch    | Yes    | No         |
| OpenAI OAuth (Codex billing)  | Yes    | No         |
| 300+ model selection          | No     | Yes        |
| Single API key for all models | No     | Yes        |

Use OpenRouter when you want model flexibility. Use direct providers
when you need provider-specific features.

## Example configurations

### Minimal

```toml
[llm]
provider = "openrouter"
model    = "anthropic/claude-sonnet-4-20250514"
# api_key set via OPENROUTER_API_KEY env var
```

### Full

```toml
[llm]
provider        = "openrouter"
model           = "anthropic/claude-sonnet-4-20250514"
api_key         = "sk-or-..."
max_iterations  = 80
timeout_secs    = 120

[llm.openrouter]
referer    = "https://my-app.example.com"
title      = "My App"
max_tokens = 8192

[llm.embeddings]
provider = "voyage"
model    = "voyage-3-lite"
```

### Private routing

Route only to providers that neither collect data nor retain it:

```toml
[llm.openrouter.provider]
data_collection = "deny"
zdr             = true
```

### Fallback models

Requests send `models = ["anthropic/claude-sonnet-4-20250514", "openai/gpt-4o"]`:

```toml
[llm]
provider = "openrouter"
model    = "anthropic/claude-sonnet-4-20250514"

[llm.openrouter]
fallback_models = ["openai/gpt-4o"]
```

### Reasoning

```toml
[llm.openrouter.reasoning]
effort = "high"
```

### Web search

```toml
[llm.openrouter.web_search]
enabled         = true
engine          = "exa"
max_results     = 5
exclude_domains = ["example.com"]
```

### Using a free model for testing

```toml
[llm]
provider = "openrouter"
model    = "meta-llama/llama-3.3-8b-instruct:free"
```

## Troubleshooting

### "OpenRouter API key not found"

Set the key via environment variable or config file:

```sh
export OPENROUTER_API_KEY=sk-or-...
```

Or add `api_key = "sk-or-..."` to the `[llm]` section in org.toml.

### Requests timing out

Some models on OpenRouter can have higher latency due to routing.
Increase the timeout:

```toml
[llm]
timeout_secs = 300
```

### "OpenRouter has no model `<slug>`"

The catalog does not list the configured model. The provider logs
`OpenRouter has no such model; chat calls will fail`, and every chat call
fails without sending anything to OpenRouter. The catalog is not fetched
again, so fix the slug and restart.

Check the slug at [openrouter.ai/models](https://openrouter.ai/models).
Slugs use the `provider/model-name` format (e.g.
`anthropic/claude-sonnet-4-20250514`).

### "OpenRouter model catalog unavailable; using conservative capabilities"

The catalog fetch failed. Chat still works, with native tools, no vision,
and an unknown context window. The provider retries on a chat call at least 10
minutes later. Check the network path to OpenRouter and the API key.

If the catalog is only slow, the first chat call logs
`OpenRouter model catalog is slow; continuing without it` after 5
seconds and goes ahead without it.

### Rate limiting (429)

The provider retries a 429, waiting at least as long as `Retry-After`.
If OpenRouter asks for more than 60 seconds, the call fails at once.
Free-tier models have tight request limits; switch to a paid model or
add credits to your OpenRouter account.

### "the OpenRouter account lacks credits"

OpenRouter returned 402. Add credits to your OpenRouter account. The
call is not retried.

### "moderation reasons: ..."

OpenRouter returned 403 because the input was flagged. The message lists
the `metadata.reasons` OpenRouter gave. Change the input or pick a model
without that moderation. The call is not retried.

### "`categories` takes at most two values"

`[llm.openrouter] categories` has more than two entries. Remove the extra
ones.

### "`search_context_size` works only with `engine = "native"`"

`[llm.openrouter.web_search]` sets `search_context_size` with an `engine`
other than `"native"`, so config load fails. Set `engine = "native"`,
remove `engine`, or remove `search_context_size`.

### "OpenRouter stream ended before the response was complete"

The stream closed before `[DONE]` or a finish reason. The partial text is
discarded; send the message again.

### "OpenRouter stopped at the token budget before answering"

The response hit the token limit before any answer or tool call. When
the model reasoned, the message reads:

```text
OpenRouter stopped at the token budget before answering: the budget was used up by reasoning. Raise [llm.openrouter] max_tokens or lower the reasoning effort.
```

Otherwise:

```text
OpenRouter stopped at the token budget before answering. Raise [llm.openrouter] max_tokens.
```

Raise `[llm.openrouter] max_tokens`, or lower `effort` or
`[llm.openrouter.reasoning] max_tokens`.

### "`effort` and `max_tokens` cannot both be set"

`[llm.openrouter.reasoning]` sets both. Keep one.

### "OpenRouter model `<slug>` does not support reasoning effort; remove `effort` from [llm.openrouter.reasoning]"

The model's catalog entry has no effort selection. Remove `effort`, or
pick a reasoning model. Restart after the fix; the catalog is not
fetched again.

### "OpenRouter model `<slug>` does not support reasoning effort `<effort>`; it supports ..."

The model lists `supported_efforts` and `effort` is not one of them.
Set `effort` to one listed in the message, then restart.

### "reasoning is mandatory for OpenRouter model `<slug>`, so effort `none` is not allowed"

The model always reasons. Set another `effort`, then restart.
