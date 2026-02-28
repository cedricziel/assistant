# OpenAI Provider

Connects the assistant to OpenAI's Chat Completions and Embeddings APIs
via the `assistant-provider-openai` crate.  Supports two authentication
modes: standard **API key** (pay-per-use) and **OAuth PKCE** (Codex /
ChatGPT subscription).

## Quick start

### API key mode

```sh
# Set your API key (or put it in config.toml)
export OPENAI_API_KEY=sk-...

# Edit ~/.assistant/config.toml
cat <<'EOF' >> ~/.assistant/config.toml
[llm]
provider = "openai"
model    = "gpt-4o"
EOF

# Run the assistant
cargo run -p assistant-cli
```

### OAuth mode (Codex subscription)

OAuth lets you authenticate with your ChatGPT account so usage is billed
against your Codex plan quota instead of API credits.

```sh
cat <<'EOF' >> ~/.assistant/config.toml
[llm]
provider = "openai"
model    = "gpt-4o"

[llm.openai]
auth_mode       = "oauth"
oauth_client_id = "your-oauth-client-id"
EOF

cargo run -p assistant-cli
# A browser window opens for ChatGPT sign-in.
# After authorizing, the token is cached locally.
```

## Configuration

All OpenAI settings live under the `[llm]` and `[llm.openai]` sections
in `~/.assistant/config.toml`.

### `[llm]` section

| Key | Env var | Default | Description |
|-----|---------|---------|-------------|
| `provider` | | `"ollama"` | Set to `"openai"` to use this provider |
| `model` | | `"gpt-4o"` | OpenAI model ID (`gpt-4o`, `gpt-4.1`, `o3-mini`, ...) |
| `api_key` | `OPENAI_API_KEY` | *(required in api-key mode)* | API key; env var takes precedence |
| `base_url` | | `"https://api.openai.com/v1"` | API base URL (for proxies or Azure-compatible endpoints) |
| `max_iterations` | | `80` | Max tool-calling iterations per turn |
| `timeout_secs` | | `120` | Timeout for each LLM call (seconds) |
| `embedding_model` | | `"text-embedding-3-small"` | Model used for `embed()` calls |

### `[llm.openai]` section

| Key | Default | Description |
|-----|---------|-------------|
| `auth_mode` | `"api-key"` | `"api-key"` or `"oauth"` |
| `oauth_client_id` | *(none)* | Required when `auth_mode = "oauth"` |
| `max_tokens` | `8192` | `max_completion_tokens` sent per request |

## Authentication modes

### API key

The simplest option. Obtain a key from
[platform.openai.com/api-keys](https://platform.openai.com/api-keys)
and set it via environment variable or config file:

```sh
# Environment variable (recommended — avoids secrets in config files)
export OPENAI_API_KEY=sk-...

# Or in config.toml
[llm]
provider = "openai"
api_key  = "sk-..."
```

The provider checks `api_key` in the config first, then falls back to
the `OPENAI_API_KEY` environment variable.

### OAuth PKCE (Codex subscription)

For users with a ChatGPT Pro / Team / Enterprise subscription that
includes Codex access.  Usage is billed against your subscription quota
rather than pay-per-use API credits.

The flow works as follows:

1. On first run, a browser window opens to `https://auth.openai.com/oauth/authorize`.
2. Sign in with your ChatGPT account and authorize the application.
3. A local callback server on `http://127.0.0.1:1455/auth/callback`
   captures the authorization code.
4. The code is exchanged for access + refresh tokens using PKCE (S256).
5. Tokens are cached in `~/.assistant/openai-oauth.json`.
6. On subsequent runs, the cached token is reused.  If it is within
   2 minutes of expiry, it is automatically refreshed using the
   refresh token.

```toml
[llm]
provider = "openai"
model    = "gpt-4o"

[llm.openai]
auth_mode       = "oauth"
oauth_client_id = "your-oauth-client-id"
```

Token storage location: `~/.assistant/openai-oauth.json`

To re-authenticate, delete the token file and restart:

```sh
rm ~/.assistant/openai-oauth.json
cargo run -p assistant-cli
```

## Compatible endpoints

The `base_url` setting makes the provider compatible with any
OpenAI-compatible API:

```toml
# Azure OpenAI
[llm]
provider = "openai"
model    = "gpt-4o"
base_url = "https://your-resource.openai.azure.com/openai/deployments/gpt-4o"
api_key  = "your-azure-key"

# Local vLLM / LM Studio / text-generation-inference
[llm]
provider = "openai"
model    = "meta-llama/Llama-3.1-8B-Instruct"
base_url = "http://localhost:8000/v1"
api_key  = "not-needed"

# OpenRouter
[llm]
provider = "openai"
model    = "anthropic/claude-3.5-sonnet"
base_url = "https://openrouter.ai/api/v1"
api_key  = "sk-or-..."
```

The provider automatically normalises the base URL:

- Appends `/v1` if missing (e.g. `https://api.openai.com` becomes
  `https://api.openai.com/v1`).
- Rewrites the default Ollama URL (`http://localhost:11434`) to the
  OpenAI base URL, so switching from `provider = "ollama"` to
  `provider = "openai"` does not accidentally point at Ollama.

## Capabilities

The OpenAI provider reports the following capabilities to the runtime:

| Capability | Value |
|------------|-------|
| Tool calling | Native (always) |
| Streaming | Yes |
| Embeddings | Yes (`text-embedding-3-small` default) |

## Models

Any model available through the OpenAI Chat Completions API works.
Common choices:

| Model | Context | Notes |
|-------|---------|-------|
| `gpt-4o` | 128k | Default.  Best balance of quality and speed. |
| `gpt-4.1` | 1M | Latest flagship model. |
| `gpt-4o-mini` | 128k | Faster, cheaper, good for simple tasks. |
| `o3-mini` | 200k | Reasoning model with thinking tokens. |
| `o3` | 200k | Most capable reasoning model. |

## Example configurations

### Minimal (API key)

```toml
[llm]
provider = "openai"
model    = "gpt-4o"
# api_key set via OPENAI_API_KEY env var
```

### Full (API key with all options)

```toml
[llm]
provider        = "openai"
model           = "gpt-4.1"
api_key         = "sk-..."
base_url        = "https://api.openai.com/v1"
max_iterations  = 80
timeout_secs    = 120
embedding_model = "text-embedding-3-large"

[llm.openai]
max_tokens = 16384
```

### OAuth (Codex subscription)

```toml
[llm]
provider       = "openai"
model          = "gpt-4o"
max_iterations = 80
timeout_secs   = 120

[llm.openai]
auth_mode       = "oauth"
oauth_client_id = "your-client-id"
max_tokens      = 8192
```

## Architecture

```
assistant-provider-openai
  src/
    lib.rs        # Module exports
    provider.rs   # OpenAIProvider (LlmProvider trait impl)
    oauth.rs      # OAuthManager (PKCE flow, token storage, refresh)
```

The provider uses `async-openai` (v0.33.0) for HTTP transport.  In
API-key mode a single `Client` is created at startup.  In OAuth mode
the `Client` is held behind a `tokio::sync::RwLock` and recreated
whenever the access token is refreshed, since the underlying
`async-openai` config carries the API key as a sync field.

## Troubleshooting

### "OpenAI API key not found"

Set the key via environment variable or config file:

```sh
export OPENAI_API_KEY=sk-...
```

Or add `api_key = "sk-..."` to the `[llm]` section in config.toml.

### "OAuth mode requires openai.oauth_client_id in config"

Add the client ID to `[llm.openai]`:

```toml
[llm.openai]
auth_mode       = "oauth"
oauth_client_id = "your-client-id"
```

### OAuth browser window does not open

The provider uses `open::that()` to launch the default browser.  If
running in a headless environment, copy the URL printed to stderr and
open it manually.

### Token refresh fails

Delete the cached token file and re-authenticate:

```sh
rm ~/.assistant/openai-oauth.json
```

### Requests going to wrong endpoint

Check that `base_url` in config.toml points to the correct API.  The
provider auto-appends `/v1` if missing, which may conflict with
endpoints that already include a version path.  Set the full URL
explicitly:

```toml
base_url = "https://your-endpoint.example.com/v1"
```
