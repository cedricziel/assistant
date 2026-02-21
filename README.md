# assistant

A minimalist, self-improving personal AI assistant written in Rust.

- **Local-first** — runs entirely on your machine via [Ollama](https://ollama.com)
- **Agent Skills native** — skills are portable `SKILL.md` directories following the [agentskills.io](https://agentskills.io) open standard
- **Self-improving** — passively logs execution traces and proposes SKILL.md refinements for human review
- **MCP server** — exposes skills via the [Model Context Protocol](https://modelcontextprotocol.io) so Claude Code and other tools can discover and invoke them
- **Multi-interface** — CLI REPL (default) and Signal messenger (feature-gated)

## Quick start

```sh
# 1. Install Ollama and pull the default model
ollama pull qwen2.5:7b

# 2. Build the CLI
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

## Built-in skills

| Skill           | Description                                                | Tier    |
| --------------- | ---------------------------------------------------------- | ------- |
| `memory-read`   | Read a persistent key/value entry                          | builtin |
| `memory-write`  | Write a persistent key/value entry                         | builtin |
| `memory-search` | Substring-search across memory entries                     | builtin |
| `web-fetch`     | Fetch a URL and return page text                           | builtin |
| `shell-exec`    | Run a shell command (requires confirmation)                | builtin |
| `list-skills`   | List all registered skills                                 | builtin |
| `self-analyze`  | Analyse execution traces and propose SKILL.md improvements | builtin |
| `schedule-task` | Register a recurring cron-style prompt                     | builtin |

## Skill discovery order

At startup the assistant scans three locations (highest priority first):

1. `~/.assistant/skills/` — personal skills
2. `<project>/.assistant/skills/` — project-scoped skills
3. `<binary dir>/skills/` — built-in skills shipped with the binary

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
cargo run -p mcp-server
```

Configure in Claude Code's `settings.json`:

```json
{
  "mcpServers": {
    "assistant": {
      "command": "/path/to/mcp-server"
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
model = "qwen2.5:7b"          # any Ollama model with tool-calling support
base_url = "http://localhost:11434"
tool_call_mode = "auto"        # "auto" | "native" | "react"
max_iterations = 10

[skills]
disabled = ["shell-exec"]      # disable specific skills
```

## Workspace layout

```
assistant/
├── crates/
│   ├── core/              # SkillDef, parser, shared types
│   ├── llm/               # Ollama client (native tool-call + ReAct fallback)
│   ├── storage/           # SQLite, SkillRegistry, trace store, memory store
│   ├── runtime/           # ReAct orchestrator, safety gate, scheduler
│   ├── skills-executor/   # Dispatches by tier (builtin / script / WASM)
│   ├── mcp-server/        # MCP stdio server
│   ├── interface-cli/     # reedline REPL binary
│   └── interface-signal/  # Signal interface (feature-gated)
├── migrations/            # SQLite migration files
├── skills/                # Built-in SKILL.md definitions
└── config.toml            # Default configuration template
```

## Development

```sh
make build      # cargo build --workspace
make test       # cargo test --workspace
make lint       # cargo clippy --workspace -D warnings
make format     # cargo fmt --all
make run        # cargo run -p assistant-cli
make run-mcp    # cargo run -p mcp-server
```

### Pre-commit hooks

Git hooks live in `.githooks/`. After cloning, activate them once:

```sh
make install-hooks
```

The pre-commit hook runs `cargo fmt --check`, `cargo clippy`, and `cargo machete` before every commit. Install [`cargo-machete`](https://github.com/bnjbvr/cargo-machete) if you don't have it:

```sh
cargo install cargo-machete
```

## Signal interface

The Signal interface is feature-gated and not enabled by default:

```sh
cargo build --workspace --features signal
```

See `crates/interface-signal/` for setup instructions.

## License

MIT
