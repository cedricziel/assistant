# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

A Rust workspace implementing a local, self-improving AI assistant. Key properties:

- **LLM**: Ollama (hardcoded fallback `qwen2.5:7b`; override via `config.toml`); native tool-calling only
- **Skills**: [Agent Skills](https://agentskills.io) open standard — `SKILL.md` directories, portable across tools
- **Storage**: SQLite via `sqlx` with 4 embedded migrations
- **Self-improvement**: every execution writes an `ExecutionTrace`; `self-analyze` generates SKILL.md proposals queued for `/review`
- **MCP server**: stdio JSON-RPC 2.0 exposing skills to Claude Code and other MCP clients

## Recommended Ollama models (2026, ≤ 12 GB VRAM)

| Model                      | VRAM (Q4_K_M) | Tool-calling | Notes                               |
| -------------------------- | ------------- | ------------ | ----------------------------------- |
| `qwen2.5:14b`              | ~10.7 GB      | Excellent    | Best all-round; recommended upgrade |
| `qwen2.5:7b`               | ~7-8 GB       | Very good    | Default; fastest option on 12 GB    |
| `llama3.1:8b`              | ~8 GB         | Excellent    | 40+ tok/s; strong agentic quality   |
| `mistral:7b-instruct-v0.3` | ~7 GB         | Good (85 %)  | Fastest; 45 tok/s                   |
| `deepseek-r1:14b`          | ~11 GB        | Excellent    | Best for complex reasoning          |
| `phi4:14b`                 | ~11 GB        | Good         | Compact; good structured output     |

> Use `Q4_K_M` quantization for 14 B models. Avoid Q3 — significant quality loss.

## Crate map

| Crate                            | Path                          | Purpose                                                                                                                           |
| -------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `assistant-core`                 | `crates/core`                 | `SkillDef`, parser, all shared types (`Message`, `ExecutionContext`, `AssistantConfig`, …)                                        |
| `assistant-llm`                  | `crates/llm`                  | `LlmProvider` trait + `LlmClient`; `ReActParser`; prompt builder; `Arc<dyn LlmProvider>` is the extension point for new backends  |
| `assistant-provider-ollama`      | `crates/provider-ollama`      | Concrete `LlmProvider` impl for Ollama — native tool-call + ReAct fallback                                                        |
| `assistant-storage`              | `crates/storage`              | `StorageLayer` (SQLite pool + migrations), `SkillRegistry`, `TraceStore`, `MemoryStore`, `RefinementsStore`, `ScheduledTaskStore` |
| `assistant-runtime`              | `crates/runtime`              | `Orchestrator` (main ReAct loop), `SafetyGate`, background `Scheduler`                                                            |
| `assistant-tool-executor`        | `crates/tool-executor`        | Registry of `ToolHandler`s (file/web/memory/etc.), ambient tool wiring, and `install_skill_from_source`                            |
| `assistant-mcp-server`           | `crates/mcp-server`           | stdio MCP server library — `tools/list`, `tools/call`, `resources/list`, `resources/read`; run via `assistant mcp`                |
| `assistant-cli`                  | `crates/interface-cli`        | **Unified binary** (`assistant`): reedline REPL + background Slack/Mattermost + `mcp`/`slack`/`mattermost` subcommands            |
| `assistant-interface-slack`      | `crates/interface-slack`      | Slack bot **library** (no binary); `SlackInterface::ambient_skills()` contributes `slack-post`; used by `assistant-cli`           |
| `assistant-interface-mattermost` | `crates/interface-mattermost` | Mattermost bot **library** (no binary); used by `assistant-cli`                                                                   |
| `assistant-interface-signal`     | `crates/interface-signal`     | Signal interface stub (feature-gated, no active deps)                                                                             |
| `assistant-integration-tests`    | `crates/integration-tests`    | End-to-end smoke tests (run with `make test-integration`)                                                                         |

Dependency order (no cycles):

```
interface-cli ──► runtime ──► llm ──► core
                      │         └──► provider-ollama
                      ├──► storage ──► core
                      ├──► tool-executor ──► core, storage, llm
                      ├──► mcp-server (optional, feature=mcp) ──► runtime, tool-executor, storage, core
                      ├──► interface-slack (optional, feature=slack) ──► runtime, storage, core
                      └──► interface-mattermost (optional, feature=mattermost) ──► runtime, storage, core
```

## Key files

| File                                      | Role                                                                                                         |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `crates/core/src/skill.rs`                | `SkillDef`, `SkillTier`, `SkillHandler` trait, `SkillOutput`, `SkillSource`                                  |
| `crates/core/src/types.rs`                | `Message`, `ExecutionContext`, `ExecutionTrace`, `AssistantConfig`, `Interface`                              |
| `crates/core/src/parser.rs`               | `parse_skill_dir()`, `parse_skill_content()`, `discover_skills()`                                            |
| `crates/core/src/memory.rs`               | `MemoryLoader` — loads/bootstraps `SOUL.md`, `IDENTITY.md`, `USER.md`, `MEMORY.md` from `~/.assistant/`      |
| `crates/llm/src/provider.rs`              | `LlmProvider` trait — implement to add a new backend (OpenAI, Anthropic, …)                                  |
| `crates/llm/src/client.rs`                | `LlmClient::chat()` — wraps `Arc<dyn LlmProvider>`; routes to provider                                       |
| `crates/provider-ollama/src/provider.rs`  | `OllamaProvider` — concrete Ollama impl of `LlmProvider`                                                     |
| `crates/storage/src/registry.rs`          | `SkillRegistry` — in-memory + SQLite skill map                                                               |
| `crates/runtime/src/orchestrator.rs`      | `Orchestrator::run_turn()` — the main loop                                                                   |
| `crates/runtime/src/safety.rs`            | `SafetyGate::check()` — blocks shell-exec on Signal, honours disabled list                                   |
| `crates/tool-executor/src/executor.rs`    | `ToolExecutor::new(storage, llm, registry)` + `register_ambient_tool()` (interior mutability via `RwLock`)   |
| `crates/tool-executor/src/installer.rs`   | `install_skill_from_source()` — local path or GitHub                                                         |
| `migrations/`                             | `001_conversations.sql` → `004_memory.sql` (embedded via `include_str!`)                                     |
| `skills/*/SKILL.md`                       | Built-in skill definitions (13 skills)                                                                       |
| `config.toml`                             | Config template — copy to `~/.assistant/config.toml`                                                         |

## Skill discovery order

At startup the assistant scans several locations (highest priority first):

1. Entries from `[skills] extra_dirs` — defaults cover `~/.claude/skills` and `./.claude/skills`
2. `~/.assistant/skills/` — personal skills
3. `<project>/.assistant/skills/` — project-scoped skills
4. `<binary dir>/skills/` — built-in skills shipped with the binary

## Skill tiers

Determined by `metadata.tier` in a `SKILL.md` frontmatter:

| `SkillTier`             | How it runs                                                           |
| ----------------------- | --------------------------------------------------------------------- |
| `Prompt`                | Orchestrator makes a sub-LLM call with SKILL.md body as system prompt |
| `Script { entrypoint }` | Subprocess via `script_executor::run_script()`                        |
| `Wasm { plugin }`       | extism (not yet implemented — returns error)                          |
| `Builtin`               | Rust `ToolHandler` registered in `ToolExecutor`                        |

## Conventions

- **Error handling**: use `anyhow::Result` throughout; `thiserror` only for library boundary errors
- **Async**: `tokio` everywhere; `#[async_trait]` on `SkillHandler` impls
- **Logging**: `tracing::{debug, info, warn}` — no `println!` in library crates
- **Config**: all config flows through `AssistantConfig` from `crates/core/src/types.rs`
- **Skill names**: kebab-case, match directory name exactly
- **No `serde_yaml`**: use `gray_matter` for SKILL.md frontmatter parsing (serde_yaml is deprecated)
- **`ToolExecutor::new`** takes `(storage, llm, registry, config)` — register all builtins up front
- **`ToolExecutor::register_ambient_tool`** — call after bootstrapping to inject interface tools like `slack-post`

## Make targets

```sh
make build            # cargo build --workspace
make build-release    # cargo build --workspace --release
make check            # cargo check --workspace (fast syntax check, no codegen)
make test             # cargo test --workspace
make test-integration # integration smoke tests (requires --ignored flag internally)
make lint             # cargo clippy --workspace -D warnings   ← run before committing
make lint-signal      # clippy for the signal interface crate (separate due to dep conflicts)
make format           # cargo fmt --all                        ← run before committing
make run              # cargo run -p assistant-cli
make run-mcp          # cargo run -p assistant-cli -- mcp
make run-slack        # cargo run -p assistant-cli --features slack -- slack
make run-mattermost   # cargo run -p assistant-cli --features mattermost -- mattermost
make build-signal     # cargo build -p assistant-interface-signal --features signal
```

To run a single test in a specific crate:

```sh
cargo test -p assistant-runtime my_test_name
# Integration tests (note: requires --ignored)
cargo test -p assistant-integration-tests --test smoke -- --ignored --nocapture
```

Always run `make lint` and `make format` before committing.

**Pre-commit hooks are mandatory.** Run `make install-hooks` after cloning to activate them. The hook enforces `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo machete --with-metadata` on every commit. Do not bypass or disable the hook.

## Commit style

Semantic commits: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `perf`.
Include the affected crate/area in parens: `feat(runtime): …`, `fix(storage): …`.

## Adding a new builtin tool

1. Add a handler struct in `crates/tool-executor/src/builtins/<name>.rs` implementing `ToolHandler`
2. Export it from `crates/tool-executor/src/builtins/mod.rs`
3. Register it inside `ToolExecutor::register_builtins()` in `executor.rs`
4. (Optional) If you also want a SKILL.md for documentation, place it under `skills/<name>/` with the desired tier

## Adding an ambient skill from an interface

Interface crates can contribute skills that are always available to the agent regardless
of which interface is active. The `slack-post` skill is an example.

1. Create `skills/<name>/SKILL.md` with `metadata.tier: builtin`
2. Add a handler in `crates/interface-<X>/src/tools/<name>.rs` (or inline) implementing `ToolHandler`
3. Implement `pub fn ambient_tools(&self) -> Vec<Arc<dyn ToolHandler>>` on the interface struct
4. In `interface-cli/src/main.rs`, call `executor.register_ambient_tool(handler)` for each ambient tool after bootstrapping

## Database schema summary

| Table               | Purpose                                                        |
| ------------------- | -------------------------------------------------------------- |
| `conversations`     | Conversation metadata                                          |
| `messages`          | Per-turn messages (user + assistant)                           |
| `skills`            | Persisted skill registry snapshot                              |
| `execution_traces`  | Every skill invocation with result + duration                  |
| `memory_entries`    | Persistent key/value store                                     |
| `skill_refinements` | LLM-proposed SKILL.md improvements (pending/accepted/rejected) |
| `scheduled_tasks`   | Cron-style recurring prompts                                   |

## MCP interface

The MCP server speaks JSON-RPC 2.0 over stdio. Tools:

- `list_skills(filter?)` — filtered skill list
- `invoke_skill(name, params?)` — run a skill via the orchestrator
- `run_prompt(prompt)` — full ReAct turn
- `install_skill(source)` — install from local path or `owner/repo[/path]`

Resources: `skills://list`, `skills://<name>`
