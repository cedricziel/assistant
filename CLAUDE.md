# CLAUDE.md — Assistant Codebase Guide

This file is automatically loaded by Claude Code. It describes the project structure,
conventions, and rules for working in this repository.

## Project overview

A Rust workspace implementing a local, self-improving AI assistant. Key properties:

- **LLM**: Ollama (default `qwen2.5:7b`); native tool-calling first, ReAct text-parsing fallback
- **Skills**: [Agent Skills](https://agentskills.io) open standard — `SKILL.md` directories, portable across tools
- **Storage**: SQLite via `sqlx` with 4 embedded migrations
- **Self-improvement**: every execution writes an `ExecutionTrace`; `self-analyze` generates SKILL.md proposals queued for `/review`
- **MCP server**: stdio JSON-RPC 2.0 exposing skills to Claude Code and other MCP clients

## Crate map

| Crate                        | Path                      | Purpose                                                                                                                           |
| ---------------------------- | ------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `assistant-core`             | `crates/core`             | `SkillDef`, parser, all shared types (`Message`, `ExecutionContext`, `AssistantConfig`, …)                                        |
| `assistant-llm`              | `crates/llm`              | `LlmClient` — Ollama native tool-call + ReAct fallback; `ReActParser`; prompt builder                                             |
| `assistant-storage`          | `crates/storage`          | `StorageLayer` (SQLite pool + migrations), `SkillRegistry`, `TraceStore`, `MemoryStore`, `RefinementsStore`, `ScheduledTaskStore` |
| `assistant-runtime`          | `crates/runtime`          | `ReactOrchestrator` (main ReAct loop), `SafetyGate`, background `Scheduler`                                                       |
| `assistant-skills-executor`  | `crates/skills-executor`  | `SkillExecutor` dispatches by tier; all builtin handlers; `install_skill_from_source`                                             |
| `assistant-mcp-server`       | `crates/mcp-server`       | stdio MCP server — `tools/list`, `tools/call`, `resources/list`, `resources/read`                                                 |
| `assistant-cli`              | `crates/interface-cli`    | reedline REPL binary; `/skills`, `/review`, `/install`, `/model`, `/help`                                                         |
| `assistant-interface-signal` | `crates/interface-signal` | Signal interface stub (feature-gated, no active deps)                                                                             |

Dependency order (no cycles):

```
interface-cli ──► runtime ──► llm ──► core
                      │
                      ├──► storage ──► core
                      └──► skills-executor ──► llm, storage, core
mcp-server ──► runtime, skills-executor, storage, core
```

## Key files

| File                                      | Role                                                                            |
| ----------------------------------------- | ------------------------------------------------------------------------------- |
| `crates/core/src/skill.rs`                | `SkillDef`, `SkillTier`, `SkillHandler` trait, `SkillOutput`, `SkillSource`     |
| `crates/core/src/types.rs`                | `Message`, `ExecutionContext`, `ExecutionTrace`, `AssistantConfig`, `Interface` |
| `crates/core/src/parser.rs`               | `parse_skill_dir()`, `parse_skill_content()`, `discover_skills()`               |
| `crates/llm/src/client.rs`                | `LlmClient::chat()` — auto-detects native vs ReAct mode                         |
| `crates/storage/src/registry.rs`          | `SkillRegistry` — in-memory + SQLite skill map                                  |
| `crates/runtime/src/orchestrator.rs`      | `ReactOrchestrator::run_turn()` — the main loop                                 |
| `crates/runtime/src/safety.rs`            | `SafetyGate::check()` — blocks shell-exec on Signal, honours disabled list      |
| `crates/skills-executor/src/executor.rs`  | `SkillExecutor::new(storage, llm, registry)`                                    |
| `crates/skills-executor/src/installer.rs` | `install_skill_from_source()` — local path or GitHub                            |
| `migrations/`                             | `001_conversations.sql` → `004_memory.sql` (embedded via `include_str!`)        |
| `skills/*/SKILL.md`                       | Built-in skill definitions (8 skills)                                           |
| `config.toml`                             | Config template — copy to `~/.assistant/config.toml`                            |

## Skill tiers

Determined by `metadata.tier` in a `SKILL.md` frontmatter:

| `SkillTier`             | How it runs                                                           |
| ----------------------- | --------------------------------------------------------------------- |
| `Prompt`                | Orchestrator makes a sub-LLM call with SKILL.md body as system prompt |
| `Script { entrypoint }` | Subprocess via `script_executor::run_script()`                        |
| `Wasm { plugin }`       | extism (not yet implemented — returns error)                          |
| `Builtin`               | Rust handler registered in `SkillExecutor`                            |

## Conventions

- **Error handling**: use `anyhow::Result` throughout; `thiserror` only for library boundary errors
- **Async**: `tokio` everywhere; `#[async_trait]` on `SkillHandler` impls
- **Logging**: `tracing::{debug, info, warn}` — no `println!` in library crates
- **Config**: all config flows through `AssistantConfig` from `crates/core/src/types.rs`
- **Skill names**: kebab-case, match directory name exactly
- **No `serde_yaml`**: use `gray_matter` for SKILL.md frontmatter parsing (serde_yaml is deprecated)
- **`SkillExecutor::new`** takes `(storage, llm, registry)` — all three are required; no lazy registration

## Make targets

```sh
make build      # cargo build --workspace
make test       # cargo test --workspace
make lint       # cargo clippy --workspace -D warnings   ← run before committing
make format     # cargo fmt --all                        ← run before committing
make run        # cargo run -p assistant-cli
make run-mcp    # cargo run -p mcp-server
```

Always run `make lint` and `make format` before committing.

**Pre-commit hooks are mandatory.** Run `make install-hooks` after cloning to activate them. The hook enforces `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo machete --with-metadata` on every commit. Do not bypass or disable the hook.

## Commit style

Semantic commits: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `perf`.
Include the affected crate/area in parens: `feat(runtime): …`, `fix(storage): …`.

## Adding a new builtin skill

1. Create `skills/<name>/SKILL.md` with `metadata.tier: builtin`
2. Add a handler struct in `crates/skills-executor/src/builtins/<name>.rs` implementing `SkillHandler`
3. Export it from `crates/skills-executor/src/builtins/mod.rs`
4. Register it in `SkillExecutor::register_builtins()` in `executor.rs`

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
