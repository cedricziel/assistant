# assistant

A minimalist, self-improving personal AI assistant written in Rust.

- **Local-first** — runs entirely on your machine via [Ollama](https://ollama.com)
- **Agent Skills native** — skills are portable `SKILL.md` directories following the [agentskills.io](https://agentskills.io) open standard
- **Self-improving** — passively logs execution traces and proposes SKILL.md refinements for human review
- **MCP server** — exposes skills via the [Model Context Protocol](https://modelcontextprotocol.io) so Claude Code and other tools can discover and invoke them
- **Multi-interface** — single `assistant` binary runs the CLI REPL, Slack bot, Mattermost bot, and MCP server concurrently via subcommands and background tasks
- **Ambient skills** — active interfaces register their capabilities (e.g. `slack-post`) into the skill executor so the agent can use them from any context

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
assistant          # Interactive REPL + all configured interfaces in background
assistant mcp      # stdio MCP server (for Claude Code, Cursor, etc.)
assistant slack    # Slack bot only (no REPL)
assistant mattermost  # Mattermost bot only (no REPL)
```

If Slack and/or Mattermost credentials are present in `~/.assistant/config.toml`,
those bots start automatically in the background when running the interactive REPL.

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

## Built-in skills

| Skill           | Description                                                        | Tier    |
| --------------- | ------------------------------------------------------------------ | ------- |
| `memory-read`   | Read a persistent key/value entry                                  | builtin |
| `memory-write`  | Write a persistent key/value entry                                 | builtin |
| `memory-search` | Substring-search across memory entries                             | builtin |
| `web-fetch`     | Fetch a URL and return page text                                   | builtin |
| `bash`          | Run a bash command (mutating; ask for confirmation on risky turns)  | builtin |
| `list-skills`   | List all registered skills                                         | builtin |
| `self-analyze`  | Analyse execution traces and propose SKILL.md improvements         | builtin |
| `schedule-task` | Register a recurring cron-style prompt                             | builtin |
| `slack-post`    | Post a message to a Slack channel (ambient; requires Slack config) | builtin |

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

Every skill execution is recorded as an `ExecutionTrace` in SQLite. When you run:

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
# Conservative default (≤ 8 GB VRAM). For 12 GB VRAM, try qwen2.5:14b.
model = "qwen2.5:7b"          # any Ollama model with tool-calling support
base_url = "http://localhost:11434"
tool_call_mode = "auto"        # "auto" | "native" | "react"
max_iterations = 10

[skills]
disabled = []                   # optional list of tool names to disable
```

## Workspace layout

```
assistant/
├── crates/
│   ├── core/                  # SkillDef, parser, shared types
│   ├── llm/                   # LlmProvider trait + LlmClient
│   ├── provider-ollama/       # Ollama backend (native tool-call + ReAct)
│   ├── storage/               # SQLite, SkillRegistry, trace store, memory store
│   ├── runtime/               # ReAct orchestrator, scheduler
│   ├── tool-executor/         # Builtin tool registry + skill installer
│   ├── mcp-server/            # MCP stdio server library (used by `assistant mcp`)
│   ├── interface-cli/         # Unified binary: REPL + background interfaces
│   ├── interface-slack/       # Slack Socket Mode library + slack-post skill
│   ├── interface-mattermost/  # Mattermost WebSocket library
│   ├── interface-signal/      # Signal interface (feature-gated, separate binary)
│   └── web-ui/                # Optional trace analysis web UI
├── docker/                    # Dockerfiles (all build the unified assistant binary)
├── migrations/                # SQLite migration files
├── skills/                    # Built-in SKILL.md definitions
└── config.toml                # Default configuration template
```

## Development

```sh
make build          # cargo build --workspace
make test           # cargo test --workspace
make lint           # cargo clippy --workspace -D warnings
make format         # cargo fmt --all
make run            # cargo run -p assistant-cli  (REPL + background interfaces)
make run-mcp        # cargo run -p assistant-cli -- mcp
make run-slack      # cargo run -p assistant-cli -- slack
make run-mattermost # cargo run -p assistant-cli -- mattermost
# Trace analysis UI
cargo run -p assistant-web-ui -- --listen 127.0.0.1:8080
```

## Observability

The runtime emits [tracing](https://docs.rs/tracing/) spans for every turn and
tool execution. To forward them to an OpenTelemetry backend (Jaeger, Tempo,
Honeycomb, etc.), set the standard OTLP endpoint before starting the assistant:

```sh
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
cargo run -p assistant-cli -- slack
```

When the environment variable is present the CLI automatically installs an OTLP
exporter (batching on the Tokio runtime) and attaches span metadata such as the
conversation ID, iteration number, and tool name. Remove the variable to fall
back to local logging only.

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
```

### Stop / disable

```sh
systemctl --user disable --now assistant-slack
```

> **Note:** The interactive REPL (`assistant` with no subcommand) is not
> suited for running as a service — use `assistant slack` or
> `assistant mattermost` subcommands which handle `SIGTERM` gracefully.

## Docker

All interfaces are baked into the same `assistant` binary. The Dockerfiles in
`docker/` use the unified binary with a different entrypoint per mode:

```sh
# Interactive REPL (default)
docker run ghcr.io/cedricziel/assistant/assistant

# MCP server
docker run ghcr.io/cedricziel/assistant/assistant assistant mcp

# Slack bot
docker run ghcr.io/cedricziel/assistant/assistant assistant slack

# Mattermost bot
docker run ghcr.io/cedricziel/assistant/assistant assistant mattermost
```

Mount your config at runtime:

```sh
docker run -v ~/.assistant/config.toml:/etc/assistant/config.toml \
  ghcr.io/cedricziel/assistant/assistant
```

## Signal interface

The Signal interface is feature-gated and ships as a separate binary due to
dependency conflicts with other workspace crates:

```sh
cargo build -p assistant-interface-signal --features signal
```

See `crates/interface-signal/README.md` for setup instructions.

## License

MIT
