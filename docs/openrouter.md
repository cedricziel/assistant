# OpenRouter Provider

Connects the assistant to [OpenRouter](https://openrouter.ai/), a unified
API gateway providing access to 300+ models from Anthropic, OpenAI, Meta,
Google, Mistral, and many more — all through a single API key and the
OpenAI Chat Completions wire format.

## Quick start

```sh
# Set your API key (or put it in config.toml)
export OPENROUTER_API_KEY=sk-or-...

# Edit ~/.assistant/config.toml
cat <<'EOF' >> ~/.assistant/config.toml
[llm]
provider = "openrouter"
model    = "anthropic/claude-sonnet-4-20250514"
EOF

# Run the assistant
cargo run -p assistant-cli
```

## Configuration

All OpenRouter settings live under the `[llm]` and `[llm.openrouter]`
sections in `~/.assistant/config.toml`.

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

| Key          | Default  | Description                                                     |
| ------------ | -------- | --------------------------------------------------------------- |
| `referer`    | _(none)_ | `HTTP-Referer` header — required by OpenRouter TOS for rankings |
| `title`      | _(none)_ | `X-Title` header — shown in the OpenRouter dashboard            |
| `max_tokens` | `8192`   | Maximum completion tokens per response                          |

## Authentication

Obtain an API key from [openrouter.ai/keys](https://openrouter.ai/keys)
and set it via environment variable or config file:

```sh
# Environment variable (recommended — avoids secrets in config files)
export OPENROUTER_API_KEY=sk-or-...

# Or in config.toml
[llm]
provider = "openrouter"
api_key  = "sk-or-..."
```

The provider checks `api_key` in the config first, then falls back to
the `OPENROUTER_API_KEY` environment variable.

## Capabilities

| Capability   | Value                 |
| ------------ | --------------------- |
| Tool calling | Native                |
| Streaming    | Yes                   |
| Vision       | Yes (model-dependent) |
| Embeddings   | No                    |

Since OpenRouter does not expose an embeddings endpoint, configure a
dedicated embedding provider if you need vector search:

```toml
[llm.embeddings]
provider = "voyage"
model    = "voyage-3-lite"
# api_key = "pa-..."  # or set VOYAGE_API_KEY env var
```

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

Or add `api_key = "sk-or-..."` to the `[llm]` section in config.toml.

### Requests timing out

Some models on OpenRouter can have higher latency due to routing.
Increase the timeout:

```toml
[llm]
timeout_secs = 300
```

### Model not found

Verify the model slug at [openrouter.ai/models](https://openrouter.ai/models).
Model slugs use `provider/model-name` format (e.g. `anthropic/claude-sonnet-4-20250514`).

### Rate limiting on free models

Free-tier models have request limits. If you hit rate limits, either
switch to a paid model or add credits to your OpenRouter account.
