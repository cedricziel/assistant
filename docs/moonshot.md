# Moonshot AI (Kimi) Provider

Connects the assistant to Moonshot AI's chat completions API via the
`assistant-provider-moonshot` crate. Moonshot AI (also branded as Kimi)
exposes an OpenAI-compatible endpoint, so the provider is implemented as
a thin facade over the existing `assistant-provider-openai` crate.

## Quick start

```sh
# Set your API key (or put it in config.toml)
export MOONSHOT_API_KEY=sk-...

# Edit ~/.assistant/config.toml
cat <<'EOF' >> ~/.assistant/config.toml
[llm]
provider = "moonshot"
model    = "kimi-k2.5"
EOF

# Run the assistant
cargo run -p assistant-cli
```

## Configuration

All Moonshot settings live under the `[llm]` and `[llm.moonshot]`
sections in `~/.assistant/config.toml`.

### `[llm]` section

| Key              | Env var            | Default                        | Description                               |
| ---------------- | ------------------ | ------------------------------ | ----------------------------------------- |
| `provider`       |                    | `"ollama"`                     | Set to `"moonshot"` to use this provider  |
| `model`          |                    | `"kimi-k2.5"`                  | Moonshot model ID (see [Models](#models)) |
| `api_key`        | `MOONSHOT_API_KEY` | _(required)_                   | API key; env var is checked as fallback   |
| `base_url`       |                    | `"https://api.moonshot.ai/v1"` | API base URL                              |
| `max_iterations` |                    | `80`                           | Max tool-calling iterations per turn      |
| `timeout_secs`   |                    | `120`                          | Timeout for each LLM call (seconds)       |

### `[llm.moonshot]` section

| Key          | Default | Description                            |
| ------------ | ------- | -------------------------------------- |
| `max_tokens` | `8192`  | Maximum completion tokens per response |

## Authentication

Obtain an API key from
[platform.moonshot.ai](https://platform.moonshot.ai/) and set it via
environment variable or config file:

```sh
# Environment variable (recommended — avoids secrets in config files)
export MOONSHOT_API_KEY=sk-...

# Or in config.toml
[llm]
provider = "moonshot"
api_key  = "sk-..."
```

The provider checks `api_key` in the config first, then falls back to
the `MOONSHOT_API_KEY` environment variable.

## Regional endpoints

Moonshot AI offers two base URLs:

| Region           | Base URL                     |
| ---------------- | ---------------------------- |
| Global (default) | `https://api.moonshot.ai/v1` |
| China            | `https://api.moonshot.cn/v1` |

To use the China endpoint:

```toml
[llm]
provider = "moonshot"
model    = "kimi-k2.5"
base_url = "https://api.moonshot.cn/v1"
```

## Capabilities

| Capability   | Value                 |
| ------------ | --------------------- |
| Tool calling | Native                |
| Streaming    | Yes                   |
| Vision       | Yes (model-dependent) |
| Embeddings   | No                    |

Since Moonshot does not provide an embeddings endpoint, configure a
dedicated embedding provider if you need vector search:

```toml
[llm.embeddings]
provider = "voyage"
model    = "voyage-3-lite"
# api_key = "pa-..."  # or set VOYAGE_API_KEY env var
```

## Models

### Current generation (Kimi K2 / K2.5)

| Model                    | Context | Tool calling | Vision | Notes                        |
| ------------------------ | ------- | ------------ | ------ | ---------------------------- |
| `kimi-k2.5`              | 262K    | Yes          | Yes    | Latest flagship, recommended |
| `kimi-k2-0905-preview`   | 262K    | Yes          | No     | K2 preview (Sep 2025)        |
| `kimi-k2-turbo-preview`  | 262K    | Yes          | No     | Faster K2 variant            |
| `kimi-k2-thinking`       | 262K    | Yes          | No     | Reasoning model              |
| `kimi-k2-thinking-turbo` | 262K    | Yes          | No     | Faster reasoning variant     |

### Kimi Latest

| Model              | Context | Vision | Notes                      |
| ------------------ | ------- | ------ | -------------------------- |
| `kimi-latest`      | 131K    | Yes    | Auto context window        |
| `kimi-latest-128k` | 131K    | Yes    | Fixed 128K context         |
| `kimi-latest-32k`  | 32K     | Yes    | Fixed 32K context          |
| `kimi-latest-8k`   | 8K      | Yes    | Fixed 8K context, cheapest |

### Legacy (moonshot-v1)

| Model              | Context | Vision | Notes                  |
| ------------------ | ------- | ------ | ---------------------- |
| `moonshot-v1-auto` | 131K    | No     | Auto context selection |
| `moonshot-v1-128k` | 131K    | No     | Fixed 128K             |
| `moonshot-v1-32k`  | 32K     | No     | Fixed 32K              |
| `moonshot-v1-8k`   | 8K      | No     | Fixed 8K, cheapest     |

Vision variants (`moonshot-v1-*-vision-preview`) are available for 8K,
32K, and 128K context sizes.

## API quirks

Moonshot AI is OpenAI-compatible but has a few known differences:

- **Temperature range**: Only `[0.0, 1.0]` (OpenAI allows up to 2.0).
  The provider sends `1.0` by default.
- **`tool_choice = "required"`**: Not supported by the Moonshot API.
- **No embeddings endpoint**: Use a dedicated embedding provider
  (Voyage, OpenAI, or Ollama).

## Example configurations

### Minimal

```toml
[llm]
provider = "moonshot"
model    = "kimi-k2.5"
# api_key set via MOONSHOT_API_KEY env var
```

### Full

```toml
[llm]
provider        = "moonshot"
model           = "kimi-k2.5"
api_key         = "sk-..."
base_url        = "https://api.moonshot.ai/v1"
max_iterations  = 80
timeout_secs    = 120

[llm.moonshot]
max_tokens = 8192

[llm.embeddings]
provider = "voyage"
model    = "voyage-3-lite"
```

### China endpoint with thinking model

```toml
[llm]
provider = "moonshot"
model    = "kimi-k2-thinking"
base_url = "https://api.moonshot.cn/v1"
```

## Architecture

```
assistant-provider-moonshot
  src/
    lib.rs        # Module exports
    provider.rs   # MoonshotProvider (thin facade over OpenAIProvider)
```

The provider delegates all chat completions, streaming, and tool-call
logic to `assistant-provider-openai`. It overrides construction
(different default base URL, API key env var, model) and identity
(`provider_name()` returns `"moonshot"`). The `embed()` method returns
an error since Moonshot has no embeddings API.

## Troubleshooting

### "Moonshot API key not found"

Set the key via environment variable or config file:

```sh
export MOONSHOT_API_KEY=sk-...
```

Or add `api_key = "sk-..."` to the `[llm]` section in config.toml.

### Requests timing out

Moonshot's API can be slower than OpenAI for large context windows.
Increase the timeout:

```toml
[llm]
timeout_secs = 300
```

### Vision not working

Only certain models support vision (e.g. `kimi-k2.5`, `kimi-latest`,
`moonshot-v1-*-vision-preview`). Check the [Models](#models) table
and ensure you are using a vision-capable model.
