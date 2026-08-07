# assistant

[![Coverage](https://github.com/cedricziel/assistant/actions/workflows/coverage.yml/badge.svg?branch=main)](https://github.com/cedricziel/assistant/actions/workflows/coverage.yml)
[![CI](https://github.com/cedricziel/assistant/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/cedricziel/assistant/actions/workflows/ci.yml)

A minimalist, self-improving personal AI assistant written in Rust.

- **Local-first** — runs entirely on your machine via [Ollama](https://ollama.com)
- **Agent Skills native** — skills are portable `SKILL.md` directories following the [agentskills.io](https://agentskills.io) open standard
- **Self-improving** — passively logs execution traces and proposes SKILL.md refinements for human review
- **MCP server** — exposes skills via the [Model Context Protocol](https://modelcontextprotocol.io) so Claude Code and other tools can discover and invoke them
- **Multi-interface** — single `assistant` binary runs orchestrator, workers, web UI, MCP, and chat interfaces via subcommands
- **Ambient skills** — active interfaces register their capabilities (e.g. `slack-post`) into the skill executor so the Persona can use them from any context

## Domain model terms

- **Persona** — long-lived assistant context with its own memory and workspace scope.
- **Subagent Process** — ephemeral delegated task execution unit.
- **A2A Profile** — Persona-attached external protocol contract (discovery + auth + interfaces).

See [docs/glossary.md](docs/glossary.md) for canonical wording rules.

## Quick start

```sh
# 1. Install Ollama and pull the default model
ollama pull qwen2.5:7b

# 2. Build the unified binary
cargo build -p assistant-cli --release

# 3. Copy the default config
mkdir -p ~/.assistant
cp config.toml ~/.assistant/config.toml

# 4. Copy built-in skills next to the binary (or run from the repo root)
cp -r skills target/release/

# 5. Run
./target/release/assistant
```

The REPL starts immediately. Type a message or a `/command`:

```
assistant> What's the weather like in Paris?
assistant> /skills
assistant> /review
assistant> /install anthropics/skills/web-search
assistant> /quit
```

### Running specific modes

The single binary supports several subcommands:

```sh
assistant orchestrator run                          # REPL + configured interfaces
assistant orchestrator run --no-repl                # background daemon mode
assistant orchestrator run --interfaces slack       # only selected interface(s)
assistant worker --interface any --id worker-1      # dedicated turn worker
assistant webui serve --listen 127.0.0.1:8080 ...  # web UI + A2A server
assistant signal link --device-name AssistantBot    # link Signal device
assistant mcp                                       # stdio MCP server
```

If Slack, Mattermost, Nextcloud, and/or Signal credentials are present in
`~/.assistant/config.toml`, interfaces start automatically when running
`assistant orchestrator run`.

## Model recommendations (2026)

All models below support Ollama native tool-calling. Use `Q4_K_M` quantization for 14 B+
models to stay within the stated VRAM budget.

| VRAM budget | Model                      | VRAM (Q4_K_M) | Speed     | Notes                               |
| ----------- | -------------------------- | ------------- | --------- | ----------------------------------- |
| **≤ 8 GB**  | `qwen2.5:7b` _(default)_   | ~7 GB         | ~40 tok/s | Great tool-calling; multilingual    |
| **≤ 8 GB**  | `llama3.1:8b`              | ~8 GB         | ~40 tok/s | Excellent agentic quality           |
| **≤ 8 GB**  | `mistral:7b-instruct-v0.3` | ~7 GB         | ~45 tok/s | Fastest; 85 % tool accuracy         |
| **≤ 12 GB** | `qwen2.5:14b`              | ~10.7 GB      | ~20 tok/s | Best all-round; recommended upgrade |
| **≤ 12 GB** | `deepseek-r1:14b`          | ~11 GB        | ~15 tok/s | Best complex reasoning              |
| **≤ 12 GB** | `phi4:14b`                 | ~11 GB        | ~18 tok/s | Compact; good structured output     |
| **≤ 24 GB** | `qwen2.5:32b`              | ~22 GB        | ~10 tok/s | Near-frontier reasoning locally     |

Pull any model and set it in `~/.assistant/config.toml`:

```sh
ollama pull qwen2.5:14b
# then set model = "qwen2.5:14b" in ~/.assistant/config.toml
```

## Built-in tools

**File I/O**

| Tool         | Description                                                           |
| ------------ | --------------------------------------------------------------------- |
| `file-read`  | Read the contents of any file from disk                               |
| `file-write` | Write content to a file, creating it and parent directories if needed |
| `file-edit`  | Replace the first occurrence of a string in a file                    |
| `file-glob`  | Find files and directories matching a glob pattern                    |

**Shell**

| Tool      | Description                                                    |
| --------- | -------------------------------------------------------------- |
| `bash`    | Run a bash command and return its stdout/stderr                |
| `process` | Manage long-running background processes (start/poll/log/kill) |

**Web**

| Tool         | Description                                      |
| ------------ | ------------------------------------------------ |
| `web-fetch`  | Fetch a URL and return page text (HTML stripped) |
| `web-search` | Search the web via DuckDuckGo and return results |

**Memory**

| Tool            | Description                                                        |
| --------------- | ------------------------------------------------------------------ |
| `memory-get`    | Read the contents of a persistent memory file                      |
| `memory-append` | Append text to a persistent memory file                            |
| `memory-search` | Search indexed memory chunks using full-text and vector similarity |

**Skills & meta**

| Tool            | Description                                                |
| --------------- | ---------------------------------------------------------- |
| `list-skills`   | List all registered skills                                 |
| `load-skill`    | Load the body text of a skill into context by name         |
| `self-analyze`  | Analyse execution traces and propose SKILL.md improvements |
| `schedule-task` | Schedule a prompt task (cron or one-shot)                  |
| `list-tasks`    | List all scheduled tasks with status and next run time     |
| `cancel-task`   | Cancel a scheduled task by ID or name                      |

**Subagent Processes**

| Tool              | Description                                                    |
| ----------------- | -------------------------------------------------------------- |
| `agent-spawn`     | Spawn an isolated Subagent Process to perform a delegated task |
| `agent-status`    | Query the status of Subagent Processes                         |
| `agent-terminate` | Cancel a running Subagent Process by ID                        |

Interfaces may register additional **ambient tools** at runtime (e.g. `slack-post`,
`slack-send-dm` when Slack is configured).

## Skill discovery order

At startup the assistant scans several locations (highest priority first):

1. Entries from `[skills] extra_dirs` — defaults include `~/.claude/skills` and `./.claude/skills` so Claude Code / NanoClaw skills are auto-loaded
2. `~/.assistant/skills/` — personal skills
3. `<project>/.assistant/skills/` — project-scoped skills
4. `<binary dir>/skills/` — built-in skills shipped with the binary

## Installing new skills

```
# From a local directory
assistant> /install ~/my-skills/code-review

# From a GitHub repository (owner/repo[/sub/path])
assistant> /install anthropics/skills/code-review
```

Or via the MCP `install_skill` tool when connecting from Claude Code.

## Self-improvement

Every skill execution produces an OpenTelemetry span that lands in SQLite's
`distributed_traces` table. When you run:

```
assistant> Analyse the web-fetch skill and suggest improvements
```

The assistant invokes `self-analyze`, queries the recent traces, sends them along with the current SKILL.md to Ollama, and stores the proposed improvement in the database. Review and apply it:

```
assistant> /review
```

## MCP server

Run the MCP server to expose skills to Claude Code, Cursor, or any other MCP client:

```sh
# From source
cargo run -p assistant-cli -- mcp

# From release binary
assistant mcp
```

Configure in Claude Code's `settings.json`:

```json
{
  "mcpServers": {
    "assistant": {
      "command": "/path/to/assistant",
      "args": ["mcp"]
    }
  }
}
```

### Exposed tools

| Tool            | Description                               |
| --------------- | ----------------------------------------- |
| `list_skills`   | List all registered skills                |
| `invoke_skill`  | Invoke a named skill                      |
| `run_prompt`    | Send a full prompt through the ReAct loop |
| `install_skill` | Install a skill from disk or GitHub       |

### Exposed resources

| Resource          | Description                             |
| ----------------- | --------------------------------------- |
| `skills://list`   | JSON metadata for all skills            |
| `skills://<name>` | Full SKILL.md content for a named skill |

## Configuration

Copy `config.toml` to `~/.assistant/config.toml` and edit:

```toml
[llm]
provider = "ollama"            # "ollama" (default), "anthropic", "openai", or "moonshot"
model = "qwen2.5:7b"          # any Ollama model with tool-calling support
base_url = "http://localhost:11434"
max_iterations = 80

```

For cloud providers, set the provider and API key:

```toml
[llm]
provider = "anthropic"
model    = "claude-sonnet-4-20250514"
api_key  = "sk-ant-..."       # or set ANTHROPIC_API_KEY env var
```

#### Embeddings

Ollama and OpenAI support embeddings natively. When using Anthropic (which
lacks built-in embeddings), configure a dedicated embedding provider:

```toml
[llm.embeddings]
provider = "voyage"            # "ollama", "openai", or "voyage"
model    = "voyage-3-lite"     # optional, provider-specific default used
# api_key = "pa-..."          # or set VOYAGE_API_KEY env var
```

To use NATS JetStream instead of SQLite for the message bus (eliminates
write-lock contention at high concurrency), set `[bus] kind = "nats"` and
configure:

```toml
[bus]
kind = "nats"
nats_url = "nats://localhost:4222"   # or set NATS_URL env var
# username = "myuser"               # or NATS_USER
# password = "s3cret"               # or NATS_PASSWORD
# token = "t0k3n!"                  # or NATS_TOKEN
# credentials_file = "/path/to.creds"  # or NATS_CREDENTIALS_FILE
```

## Workspace layout

```
assistant/
├── crates/
│   ├── core/                           # Shared types, ToolHandler trait, MessageBus
│   ├── llm/                            # LlmProvider trait, EmbeddingProvider, LlmClient
│   ├── provider-ollama/                # Ollama backend (native tool-call + embeddings)
│   ├── provider-anthropic/             # Anthropic backend (Claude models)
│   ├── provider-openai/                # OpenAI backend (GPT models + embeddings)
│   ├── provider-moonshot/              # Moonshot/Kimi backend (OpenAI-compatible)
│   ├── skills/                         # Skill parsing, validation, embedded builtins
│   ├── storage/                        # SQLite, SkillRegistry, trace store, memory store
│   ├── bus-nats/                       # NATS JetStream MessageBus (optional, feature-gated)
│   ├── runtime/                        # ReAct orchestrator, scheduler, Subagent Processes
│   ├── tool-executor/                  # Builtin tool registry + skill installer
│   ├── transcription/                  # Voice transcription providers (Whisper, Ollama, Deepgram)
│   ├── mcp-server/                     # MCP stdio server library (used by `assistant mcp`)
│   ├── mcp-client/                     # MCP client for connecting to external MCP servers
│   ├── interface-cli/                  # Unified binary: orchestrator, worker, webui, MCP
│   ├── interface-slack/                # Slack Socket Mode library + ambient tools
│   ├── interface-mattermost/           # Mattermost WebSocket library
│   ├── interface-nextcloud/            # Nextcloud Talk webhook bot
│   ├── interface-signal/               # Signal interface library
│   ├── web-ui/                         # Trace analysis web UI + A2A protocol server
│   ├── a2a-json-schema/                # A2A protocol JSON Schema types
│   ├── opentelemetry-exporter-sqlite/  # SQLite exporter for OpenTelemetry spans/logs
│   └── integration-tests/              # End-to-end smoke tests
├── docker/                             # Dockerfiles (all build the unified assistant binary)
├── migrations/                         # SQLite migration files
├── skills/                             # Built-in SKILL.md definitions
└── config.toml                         # Default configuration template
```

## Development

```sh
make build          # cargo build --workspace
make test           # cargo test --workspace
make lint           # cargo clippy --workspace -D warnings
make format         # cargo fmt --all
make run            # cargo run -p assistant-cli  (REPL + background interfaces)
make run-mcp        # cargo run -p assistant-cli -- mcp
make run-slack      # cargo run -p assistant-cli -- orchestrator run --interfaces slack --no-repl
make run-mattermost # cargo run -p assistant-cli -- orchestrator run --interfaces mattermost --no-repl
make run-nextcloud  # cargo run -p assistant-cli -- orchestrator run --interfaces nextcloud --no-repl
make run-signal     # cargo run -p assistant-cli -- orchestrator run --interfaces signal --no-repl
# Trace analysis UI (auth token required)
ASSISTANT_WEB_TOKEN=changeme cargo run -p assistant-cli -- webui serve --listen 127.0.0.1:8080
```

## Observability

The runtime emits OpenTelemetry **traces, logs, and metrics** for every
conversation turn, LLM call, and tool invocation. Real observability lives
outside the assistant: export via OTLP to your own stack and query it there.
A small local SQLite store powers the built-in web UI viewers for single-node
debugging, and can be switched off with `[observability] exporter = "none"`.

```sh
# Send all signals to an OTLP collector (Tempo, Grafana, Honeycomb, …)
# Note: OTLP over HTTP — the :4318 port, not the gRPC :4317.
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 assistant

# Per-signal endpoints (used verbatim — include the full path)
OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://tempo:4318/v1/traces \
OTEL_EXPORTER_OTLP_LOGS_ENDPOINT=http://loki:4318/v1/logs \
  assistant

# Auth headers for managed backends
OTEL_EXPORTER_OTLP_HEADERS="Authorization=Bearer token" assistant

# Disable telemetry entirely
OTEL_SDK_DISABLED=true assistant
```

The `opentelemetry-otlp` crate reads the standard `OTEL_EXPORTER_OTLP_*` env
vars (endpoint, protocol, headers, timeout, compression) with per-signal
overrides — see [docs/opentelemetry.md](docs/opentelemetry.md) for the full
reference of emitted spans, metrics, and supported environment variables, and
[ADR-0010](docs/adr/adr-0010-external-observability.md) for why the built-in
analytics stack was removed.

### Pre-commit hooks

Git hooks live in `.githooks/`. After cloning, activate them once:

```sh
make install-hooks
```

The pre-commit hook runs `cargo fmt --check`, `cargo clippy`, and `cargo machete` before every commit. Install [`cargo-machete`](https://github.com/bnjbvr/cargo-machete) if you don't have it:

```sh
cargo install cargo-machete
```

## Running as a user service (Linux)

The `.deb` and `.rpm` packages ship systemd **user** unit files so the Slack
and Mattermost bots run in the background under your own account — with full
access to your desktop session (`$DISPLAY`, `$WAYLAND_DISPLAY`, D-Bus) for
future desktop integration.

### Quick start

```sh
# 1. Install the package (sets up unit files in /usr/lib/systemd/user/)
sudo apt install ./assistant_*.deb    # or rpm -i assistant_*.rpm

# 2. Edit your config
cp /etc/assistant/config.toml.example ~/.assistant/config.toml
$EDITOR ~/.assistant/config.toml      # add Slack/Mattermost credentials

# 3. Enable and start whichever bots you need
systemctl --user enable --now assistant-slack
systemctl --user enable --now assistant-mattermost
systemctl --user enable --now assistant-nextcloud
systemctl --user enable --now assistant-web-ui

# 4. (Once) persist across reboots without staying logged in
loginctl enable-linger $USER
```

### Upgrade path

```sh
sudo apt upgrade assistant
# Restart=on-failure in the unit file brings the service back up automatically
# after the binary is replaced.  No manual restart needed.
```

### View logs

```sh
journalctl --user -u assistant-slack -f
journalctl --user -u assistant-mattermost -f
journalctl --user -u assistant-nextcloud -f
journalctl --user -u assistant-web-ui -f
```

### Stop / disable

```sh
systemctl --user disable --now assistant-slack
```

> **Note:** The interactive REPL (`assistant` with no subcommand) is not
> suited for running as a service — use
> `assistant orchestrator run --interfaces <name> --no-repl`.

## Docker

The Docker image uses `assistant` as the primary entrypoint for all runtime
modes, including Web UI serving via `assistant webui serve`:

```sh
# Interactive REPL (default)
docker run ghcr.io/cedricziel/assistant/assistant

# MCP server
docker run ghcr.io/cedricziel/assistant/assistant assistant mcp

# Slack bot
docker run ghcr.io/cedricziel/assistant/assistant assistant orchestrator run --interfaces slack --no-repl

# Mattermost bot
docker run ghcr.io/cedricziel/assistant/assistant assistant orchestrator run --interfaces mattermost --no-repl

# Web UI
docker run ghcr.io/cedricziel/assistant/assistant assistant webui serve --listen 0.0.0.0:8080 --auth-token changeme
```

Mount your config at runtime:

```sh
docker run -v ~/.assistant/config.toml:/etc/assistant/config.toml \
  ghcr.io/cedricziel/assistant/assistant
```

## Signal interface

Signal is available through orchestrator mode:

```sh
assistant orchestrator run --interfaces signal --no-repl
```

See `crates/interface-signal/README.md` for setup and device linking details.

## Further documentation

| Topic                                          | Description                                           |
| ---------------------------------------------- | ----------------------------------------------------- |
| [OpenTelemetry](docs/opentelemetry.md)         | Emitted spans, metrics, and supported env vars        |
| [Slack interface](docs/slack.md)               | Setup, ambient tools, and event handling              |
| [Nextcloud Talk interface](docs/nextcloud.md)  | Webhook bot setup and message flow                    |
| [OpenAI provider](docs/openai.md)              | API key and OAuth PKCE auth, Azure/vLLM compatibility |
| [Moonshot provider](docs/moonshot.md)          | Kimi K2/K2.5 models, regional endpoints               |
| [Web UI](docs/web-ui.md)                       | Trace analysis dashboard and A2A protocol server      |
| [Authentication](docs/authentication.md)       | Web UI token auth, cookie flow, and A2A security      |
| [Message bus](docs/messaging.md)               | Durable topic-based message bus architecture          |
| [Voice transcription](docs/transcription.md)   | Whisper, Ollama, and Deepgram transcription providers |
| [Glossary](docs/glossary.md)                   | Canonical Persona/Subagent/A2A terminology            |
| [Persona migration](docs/persona-migration.md) | Breaking cutover guide for Persona model migration    |

## License

MIT
