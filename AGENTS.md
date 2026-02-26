# AGENTS.md

Guidance for AI coding agents working in this Rust workspace.

## Build, Lint, Test

```sh
make build            # cargo build --workspace
make check            # cargo check --workspace  (fast, no codegen)
make test             # cargo test --workspace
make lint             # cargo clippy --workspace -- -D warnings
make format           # cargo fmt --all
make test-integration # cargo test -p assistant-integration-tests --test smoke -- --ignored --nocapture
```

Run a single test in a specific crate:

```sh
cargo test -p assistant-runtime test_name
cargo test -p assistant-tool-executor test_name -- --nocapture
```

**Always run `make lint` and `make format` before committing.** Pre-commit hooks
enforce `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo machete --with-metadata`.
Install hooks after cloning: `make install-hooks`.

## Workspace Structure

12 crates under `crates/`, one root crate. Edition 2021, resolver 2.

| Crate (package name)             | Path                          | Purpose                                              |
|----------------------------------|-------------------------------|------------------------------------------------------|
| `assistant-core`                 | `crates/core`                 | Shared types, ToolHandler trait, MessageBus trait      |
| `assistant-llm`                  | `crates/llm`                  | LlmProvider trait, LlmClient, prompt builder          |
| `assistant-provider-ollama`      | `crates/provider-ollama`      | Ollama LlmProvider implementation                     |
| `assistant-storage`              | `crates/storage`              | SQLite (sqlx), SkillRegistry, TraceStore, MessageBus  |
| `assistant-runtime`              | `crates/runtime`              | Orchestrator (main ReAct loop), SafetyGate, Scheduler |
| `assistant-tool-executor`        | `crates/tool-executor`        | ToolHandler registry, builtin tools, dispatch         |
| `assistant-mcp-server`           | `crates/mcp-server`           | stdio JSON-RPC 2.0 MCP server                        |
| `assistant-cli`                  | `crates/interface-cli`        | Unified binary: REPL + subcommands                    |
| `assistant-interface-slack`      | `crates/interface-slack`      | Slack bot library                                     |
| `assistant-interface-mattermost` | `crates/interface-mattermost` | Mattermost bot library                                |
| `assistant-interface-signal`     | `crates/interface-signal`     | Signal interface stub (feature-gated)                 |
| `assistant-integration-tests`    | `crates/integration-tests`    | End-to-end smoke tests                                |

Dependency order (no cycles):
```
interface-cli -> runtime -> llm -> core
                    |         '-> provider-ollama
                    |-> storage -> core
                    |-> tool-executor -> core, storage, llm
                    '-> mcp-server, interface-slack, interface-mattermost (optional features)
```

## Code Style

### Formatting

Default `cargo fmt` (no `rustfmt.toml`). Default clippy with `-D warnings` (all warnings are errors).

### Imports

Standard Rust ordering enforced by `cargo fmt`:
1. `std` imports
2. External crate imports
3. Workspace crate imports (`assistant_*`)
4. `crate::` / `self::` imports

Separate groups with blank lines:
```rust
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tracing::{debug, info, warn};

use assistant_core::{Message, ToolHandler, ToolOutput};

use crate::helpers::build_context;
```

### Error Handling

- **`anyhow::Result`** everywhere. This is the standard return type for all functions.
- **`thiserror`** only at library boundary errors (rare).
- **`anyhow::bail!`** for early error returns.
- **`anyhow::Context`** via `.with_context(|| "descriptive msg")` to add context to errors.
- Tool handlers return `Ok(ToolOutput::error(...))` for non-fatal tool errors (shown to LLM), reserving `Err(...)` for truly unrecoverable failures.

### Async

- **`tokio`** runtime everywhere (features = `["full"]`).
- **`#[async_trait]`** on all trait definitions with async methods (`ToolHandler`, `LlmProvider`).
- **`tokio::fs`** for async file I/O in tool handlers; `std::fs` acceptable only for tiny files.
- **`tokio::sync::RwLock`** for async-safe interior mutability (not `std::sync::RwLock`).
- **`tokio::sync::mpsc`** for streaming channels.

### Naming Conventions

| Element          | Convention        | Example                                      |
|------------------|-------------------|----------------------------------------------|
| Crate names      | `assistant-*`     | `assistant-core`, `assistant-tool-executor`   |
| Module files     | `snake_case`      | `skill_registry.rs`, `tool_executor.rs`       |
| Structs          | `PascalCase`      | `ToolExecutor`, `FileReadHandler`             |
| Traits           | `PascalCase`      | `ToolHandler`, `LlmProvider`                  |
| Handler structs  | `<Feature>Handler` | `FileReadHandler`, `BashHandler`             |
| Tool names (str) | `kebab-case`      | `"file-read"`, `"web-fetch"`, `"memory-get"`  |
| Skill names      | `kebab-case`      | Must match directory name exactly              |
| Constants        | `SCREAMING_SNAKE`  | `DEFAULT_LIMIT`, `BOOTSTRAP_MAX_CHARS`        |
| Enum variants    | `PascalCase`      | `MessageRole::User`, `Interface::Cli`         |

### Type Patterns

- `Arc<dyn Trait>` for dependency injection (`Arc<dyn LlmProvider>`, `Arc<dyn ToolHandler>`).
- `Arc<T>` for shared ownership (`Arc<StorageLayer>`, `Arc<SkillRegistry>`).
- `RwLock<HashMap<...>>` for mutable registries.
- `HashMap<String, serde_json::Value>` for dynamic tool parameters.
- `ToolOutput` with `success()`/`error()` constructors and `with_data()` builder.
- Builder-style `with_*` methods for optional configuration on structs.

### Logging

Use `tracing` macros only: `debug!`, `info!`, `warn!`, `error!`. No `println!` in library crates.

### Doc Comments

- Module-level: `//!` at the top of the file.
- Functions/methods: `///` with `# Parameters` sections for public APIs.
- Section dividers: `// -- Section Name --` with em-dashes.

### Dependencies

- **No `serde_yaml`**: use `gray_matter` for SKILL.md frontmatter parsing.
- All shared dependencies are declared in `[workspace.dependencies]` in root `Cargo.toml`.

## Commit Style

Semantic commits with crate scope: `feat(runtime): add retry logic`, `fix(storage): handle null timestamps`.

Prefixes: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `perf`.

## Adding a Builtin Tool

1. Create `crates/tool-executor/src/builtins/<name>.rs` with a handler struct.
2. Implement `#[async_trait] impl ToolHandler` with `name()`, `description()`, `params_schema()`, `run()`.
3. Export from `crates/tool-executor/src/builtins/mod.rs`.
4. Register in `ToolExecutor::register_builtins()` in `executor.rs`.
5. Optionally add `skills/<name>/SKILL.md` for documentation.

## Testing Patterns

- Unit tests in `#[cfg(test)] mod tests` at the bottom of each file.
- Use `#[tokio::test]` for all async tests.
- `StorageLayer::new_in_memory()` for test databases (no disk I/O).
- `wiremock` for HTTP mocking (Ollama API responses).
- Helper functions for fixtures: `make_skill()`, `build()`, `mount_answer()`.
- Use `assert_eq!` with descriptive messages as the third argument.

## CI

GitHub Actions runs on push to `main` and PRs: check, test, lint (clippy), format.
Integration tests run with `continue-on-error: true` (require Ollama).
