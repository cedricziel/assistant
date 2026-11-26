# AGENTS.md

Guidance for AI coding agents working in this Rust workspace. Scoped guidance lives in
nested `AGENTS.md` files (`app/`, `crates/interfaces/`, `crates/tool-executor/`).

## Development Discipline

**You must follow Test Driven Development. We always start with a failing test.**

See `.claude/skills/tdd/SKILL.md` for the full TDD workflow, rules, and Rust-specific
guidance. In short:

1. Write a failing test first.
2. Confirm it is **red** before writing any implementation.
3. Write the minimum code to make it **green**.
4. Refactor under green.

## Build, Lint, Test

Targets live in the `Makefile` (`make build`, `make check`, `make test`, `make lint`, …).

**Always run `make lint` and `make format` before committing.** Pre-commit hooks
enforce `cargo fmt --check`, `cargo clippy -D warnings`, `cargo machete --with-metadata`,
`dart_pre_commit` (format, analyze, deps, OSV), and `flutter test`.
Install hooks after cloning: `make install-hooks`.

**Build cache:** use [sccache](https://github.com/mozilla/sccache) as the rustc wrapper
(`rustc-wrapper = "sccache"` in `~/.cargo/config.toml` or `RUSTC_WRAPPER=sccache`).
`.cargo/config.toml` turns incremental compilation off because sccache cannot cache
incremental builds. Keep it off: with it on, every worktree recompiles the workspace
and grows a `target/debug/incremental` cache that is never cleaned. Run `cargo clean`
in a worktree once its branch has merged.

**Note:** `cargo check --all-features` may require `protoc` (protobuf compiler) for certain features.

Integration tests (`make test-integration`) require Ollama; CI runs them with `continue-on-error: true`.

## Architecture Decisions

- ADRs live in `docs/adr/`.
- When making architectural changes, add or update an ADR in `docs/adr/`.
- Messenger adapters use thin `reqwest` + `tokio-tungstenite` clients. Never add
  `matrix-sdk`, `slack-morphism`, `mattermost_api`, or `presage` (see `crates/interfaces/AGENTS.md`).

## Terminology

- Canonical domain terms are defined in `docs/glossary.md`.
- Use `Persona`, `Subagent Process`, and `A2A Profile` in new docs and UX copy.
- Avoid unqualified `agent` in architecture prose unless referring to a literal code identifier.

## Code Style

### Lint policy

The workspace declares its baseline lint set in the root `Cargo.toml`
`[workspace.lints]` table. Crates pick one of two shapes:

- **Inherit** — `[lints]\nworkspace = true`. Used by crates that have no
  overrides (the default for new and clean crates).
- **Manual replay** — explicit `[lints.clippy]` / `[lints.rust]` blocks.
  Used by crates that need to raise or relax individual lints. Cargo forbids
  combining `workspace = true` with overrides in the same manifest, so the
  workspace lints must be replayed manually.

**Panic-free contract.** `clippy::unwrap_used`, `clippy::expect_used`, and
`clippy::panic` are `warn` at the workspace baseline and ratcheted to `deny`
in every crate with production code (only `assistant-integration-tests` stays
on `warn`). Production code must not unwrap, expect, or panic.

Test code (`#[cfg(test)]` modules and `tests/` directories) is exempt: the
default `cargo clippy --workspace` invocation used by `make lint` and CI
does not check test targets, so `.unwrap()` remains ergonomic in tests.

The enforcement scanner lives at `tests/workspace_lint_policy.rs` and runs as
part of `cargo test -p assistant`. See `openspec/changes/workspace-lint-policy/`
for the originating proposal.

### Imports

Group imports in this order (`cargo fmt` does not do this for you):

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

- **`anyhow::Result`** everywhere. **`thiserror`** only at library boundary errors (rare).
- **`anyhow::bail!`** for early error returns; `.with_context(|| "msg")` to add context.
- Tool handlers return `Ok(ToolOutput::error(...))` for non-fatal tool errors (shown to LLM), reserving `Err(...)` for truly unrecoverable failures.

### Async

- **`tokio`** runtime everywhere (features = `["full"]`).
- **`#[async_trait]`** on all trait definitions with async methods (`ToolHandler`, `LlmProvider`).
- **`tokio::fs`** for async file I/O in tool handlers; `std::fs` acceptable only for tiny files.
- **`tokio::sync::RwLock`** for async-safe interior mutability (not `std::sync::RwLock`).
- **`tokio::sync::mpsc`** for streaming channels.

### Naming Conventions

| Element          | Convention         | Example                                      |
| ---------------- | ------------------ | -------------------------------------------- |
| Crate names      | `assistant-*`      | `assistant-core`, `assistant-tool-executor`  |
| Handler structs  | `<Feature>Handler` | `FileReadHandler`, `BashHandler`             |
| Tool names (str) | `kebab-case`       | `"file-read"`, `"web-fetch"`, `"memory-get"` |
| Skill names      | `kebab-case`       | Must match directory name exactly            |

### Type Patterns

- `Arc<dyn Trait>` for dependency injection (`Arc<dyn LlmProvider>`, `Arc<dyn ToolHandler>`).
- `Arc<T>` for shared ownership (`Arc<StorageLayer>`, `Arc<SkillRegistry>`).
- `RwLock<HashMap<...>>` for mutable registries.
- `HashMap<String, serde_json::Value>` for dynamic tool parameters.
- `ToolOutput` with `success()`/`error()` constructors, `with_data()` and `with_attachment()` builders.
- Builder-style `with_*` methods for optional configuration on structs.

### Logging & Dependencies

- Use `tracing` macros only: `debug!`, `info!`, `warn!`, `error!`. No `println!` in library crates.
- Module-level docs: `//!`; function docs: `///`; section dividers: `// -- Name --`.
- Use `gray_matter` for SKILL.md frontmatter parsing (`serde_yaml` only in A2A agent store).
- All shared dependencies declared in `[workspace.dependencies]` in root `Cargo.toml`.

## Commit Style

Semantic commits with crate scope: `feat(runtime): add retry logic`, `fix(storage): handle null timestamps`.

Prefixes: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `perf`.

## Testing Patterns

- Unit tests in `#[cfg(test)] mod tests` at the bottom of each file.
- Use `#[tokio::test]` for all async tests.
- `StorageLayer::new_in_memory()` for test databases (no disk I/O, fully migrated).
- `wiremock` for HTTP mocking (Ollama/provider API responses).
- Helper functions for fixtures: `make_skill()`, `build()`, `mount_answer()`.
- Use `assert_eq!` with descriptive messages as the third argument.
- Per-crate coverage floor is 80%; see the `test-coverage` skill.
- Tests never touch the real `~/.assistant`. Library code takes the install root as a
  parameter; tests pass a `tempfile::tempdir()`. `tests/workspace_home_dir_lint.rs` bans
  `home_dir()` outside the entry points.

### Choosing fakes

The workspace ships three test seams; pick the one whose blast radius
matches what's actually under test:

- **`InMemoryFooStore`** — when the code under test takes
  `Arc<dyn FooStore>` (or `&dyn FooStore`) and you only need that store's
  surface. Constructs in microseconds, no SQLite, no migrations. Defined
  alongside the `SqliteFooStore` impl in `crates/storage/src/*.rs`;
  re-exported through `assistant_test_support::prelude`. See
  `docs/adr/adr-0009-testability-architecture.md` for the trait-pair
  pattern.
- **`StorageLayer::new_in_memory()`** — when the code under test reaches
  into multiple stores or runs a SQL query directly. Spins a `:memory:`
  SQLite pool with all migrations applied. ~10ms per test.
- **`wiremock::MockServer`** — when the code under test calls an HTTP
  endpoint. Pair with `with_client(client, base_url)` constructors
  (`HttpRequestActionExecutor`, `WebPushClient`, etc.) so the test can
  route requests to the mock instead of the real host.

For LLM-shaped behavior, use `ScriptedLlmProvider` from
`assistant_llm_provider::scripted` to queue canned `LlmResponse` values
without booting any backend.

For orchestration-shaped behavior, use the trait facades — `Arc<dyn
OrchestrationEngine>`, `Arc<dyn ToolDispatcher>`, `Arc<dyn SkillCatalog>`
— with `StubOrchestrationEngine`, `StubToolDispatcher`,
`InMemorySkillCatalog`. The `crates/mcp-server/tests/dispatch.rs` suite
is the canonical worked example.
