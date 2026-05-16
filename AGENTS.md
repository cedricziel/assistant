# AGENTS.md

Guidance for AI coding agents working in this Rust workspace.

## Development Discipline

**You must follow Test Driven Development. We always start with a failing test.**

See `.claude/skills/tdd/SKILL.md` for the full TDD workflow, rules, and Rust-specific
guidance. In short:

1. Write a failing test first.
2. Confirm it is **red** before writing any implementation.
3. Write the minimum code to make it **green**.
4. Refactor under green.

## Build, Lint, Test

```sh
make build            # cargo build --workspace
make check            # cargo check --workspace  (fast, no codegen)
make test             # cargo test --workspace
make lint             # cargo clippy --workspace -- -D warnings
make format           # cargo fmt --all
make lint-flutter     # dart_pre_commit (format, analyze, deps, OSV)
make test-flutter     # flutter test
make precommit        # run all pre-commit checks manually
make test-integration # cargo test -p assistant-integration-tests --test smoke -- --ignored --nocapture
```

Run a single test in a specific crate:

```sh
cargo test -p assistant-runtime test_name
cargo test -p assistant-tool-executor test_name -- --nocapture
```

Run targets: `make run` (orchestrator), `make run-mcp` (MCP stdio), `make run-slack`, `make run-mattermost`, `make run-matrix`, `make run-nextcloud`, `make run-signal`, `make run-webui`, `make run-worker`.

**Always run `make lint` and `make format` before committing.** Pre-commit hooks
enforce `cargo fmt --check`, `cargo clippy -D warnings`, `cargo machete --with-metadata`,
`dart_pre_commit` (format, analyze, deps, OSV), and `flutter test`.
Install hooks after cloning: `make install-hooks`.

**Note:** `cargo check --all-features` may require `protoc` (protobuf compiler) for certain features.

## Flutter App (`app/`)

The `app/` directory contains a Flutter 3.x application targeting web and macOS platforms.

### Prerequisites

- Flutter SDK 3.x (stable channel): https://docs.flutter.dev/get-started/install
- Dart 3.x (bundled with Flutter)
- Xcode (macOS target only — required for `flutter build macos`)
- Chrome or another web browser (web target)

Verify installation: `flutter doctor`

### Flutter Commands

```sh
# Run from the app/ directory (cd app/ first, or prefix commands with `cd app &&`)
flutter pub get          # install dependencies (run after cloning or pubspec.yaml changes)
flutter analyze          # static analysis — zero issues required (--fatal-infos enforced in CI)
flutter test             # run all unit and widget tests
flutter run -d chrome    # launch on web (requires Chrome)
flutter run -d macos     # launch on macOS (requires Xcode)
flutter build web        # build static web site → app/build/web/
flutter build macos      # build macOS .app → app/build/macos/Build/Products/Release/
```

### App Structure

```text
app/lib/
  api/
    client.dart                    # AssistantClient: HTTP + SSE streaming
    models/                        # Data classes (ServerProfile, Conversation, Persona, …)
    endpoints/                     # Typed API endpoint wrappers
  features/
    connection/                    # Server profile setup & auth (US2)
    chat/                          # Streaming chat UI (US1)
    personas/                      # Persona picker (US3)
    traces/                        # Trace viewer (US4)
    logs/                          # Log viewer (US4)
    skills/                        # Skill browser (US5)
  router/
    app_router.dart                # go_router routes + auth redirect guards
  main.dart
app/test/
  unit/api/client_test.dart        # SSE model unit tests
  widget/connection_screen_test.dart
  widget_test.dart
```

### State Management

Riverpod 2.x (`flutter_riverpod`). Providers live in `*_provider.dart` files alongside their screens. All async providers use `AsyncNotifier` / `AutoDisposeAsyncNotifier`.

### CORS

The Rust web-ui server emits `Access-Control-Allow-Origin` headers for Flutter web. Configure the allowed origin with:

```sh
assistant webui serve --cors-origin http://localhost:4040
# or via env var:
ASSISTANT_WEB_CORS_ORIGIN=http://localhost:4040 assistant webui serve
```

### CI

GitHub Actions runs `flutter analyze --fatal-infos` and `flutter test` on every push/PR that touches `app/**` (see `.github/workflows/flutter.yml`).

---

## Workspace Structure

Multiple crates under `crates/`, one root crate. Edition 2024, resolver 2.

| Crate (package name)             | Path                                    | Purpose                                                                                                                                         |
| -------------------------------- | --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `assistant-core`                 | `crates/core`                           | Shared types, LLM traits (LlmProvider, EmbeddingProvider), ToolHandler                                                                          |
| `assistant-auth`                 | `crates/auth`                           | OAuth2 server, JWT, API keys, OIDC federation, Axum auth middleware                                                                             |
| `assistant-llm-provider`         | `crates/llm-provider`                   | All LlmProvider implementations (Ollama, Anthropic, OpenAI, Moonshot)                                                                           |
| `assistant-skills`               | `crates/skills`                         | Skill parsing, validation, embedded builtins                                                                                                    |
| `assistant-storage`              | `crates/storage`                        | SQLite (sqlx), SkillRegistry, TraceStore, SqliteMessageBus                                                                                      |
| `assistant-bus-nats`             | `crates/bus-nats`                       | NATS JetStream MessageBus (optional, feature-gated)                                                                                             |
| `assistant-runtime`              | `crates/runtime`                        | Orchestrator (main ReAct loop), SafetyGate, Scheduler, ChannelRunner, TitleGeneratorWorker (consumes `turn.result` to auto-title conversations) |
| `assistant-tool-executor`        | `crates/tool-executor`                  | ToolHandler registry, builtin tools, dispatch                                                                                                   |
| `assistant-mcp-server`           | `crates/mcp-server`                     | stdio JSON-RPC 2.0 MCP server                                                                                                                   |
| `assistant-mcp-client`           | `crates/mcp-client`                     | MCP client for external MCP server connections                                                                                                  |
| `assistant-cli`                  | `crates/interface-cli`                  | Unified binary: REPL + subcommands                                                                                                              |
| `assistant-interfaces`           | `crates/interfaces`                     | All messenger adapters (Slack, Mattermost, Matrix, Nextcloud, Signal)                                                                           |
| `assistant-web-ui`               | `crates/web-ui`                         | Web UI/A2A server implementation (invoked via `assistant webui serve`)                                                                          |
| `assistant-transcription`        | `crates/transcription`                  | Voice transcription providers (Whisper, Ollama, etc)                                                                                            |
| `assistant-a2a-json-schema`      | `crates/a2a-json-schema`                | A2A protocol JSON Schema types                                                                                                                  |
| `assistant-backup`               | `crates/backup`                         | Backup and restore for the assistant installation                                                                                               |
| `assistant-workflow`             | `crates/workflow`                       | Workflow run engine and action executor abstractions                                                                                            |
| `assistant-workflow-http`        | `crates/workflow-http`                  | HTTP action node executor for assistant-workflow                                                                                                |
| `opentelemetry-exporter-sqlite`  | `crates/opentelemetry-exporter-sqlite`  | SQLite exporter for OpenTelemetry spans/logs                                                                                                    |
| `opentelemetry-exporter-iceberg` | `crates/opentelemetry-exporter-iceberg` | Apache Iceberg exporter for OpenTelemetry spans, logs, and metrics                                                                              |
| `assistant-integration-tests`    | `crates/integration-tests`              | End-to-end smoke tests                                                                                                                          |

Dependency order (no cycles):

```
interface-cli -> runtime -> core
                    |         '-> llm-provider -> core
                    |-> storage -> core
                    |-> bus-nats -> core  (optional, feature = "nats")
                    |-> tool-executor -> core, storage
                    '-> mcp-server, interfaces -> core, runtime, storage, transcription
web-ui -> auth -> core, storage
            '-> jwt, oauth2, oidc, api_keys, middleware
```

## Architecture Decisions

- ADRs live in `docs/adr/`.
- When making architectural changes, add or update an ADR in `docs/adr/`.

### Messenger Interface Clients

All messenger interface adapters live in `crates/interfaces/` (`assistant-interfaces`) as sub-modules (`slack`, `mattermost`, `matrix`, `nextcloud`, `signal`). They use **thin `reqwest` + `tokio-tungstenite` HTTP/WebSocket clients** — not heavy platform SDKs. This was a deliberate decision (see `openspec/changes/thin-messenger-http-clients/design.md`):

- **Matrix**: uses plain long-poll `/sync` via `reqwest`, _not_ `matrix-sdk`. The old `matrix-sdk` dep was removed entirely (including from `Cargo.toml`) to eliminate its 80k+ line transitive footprint and SQLite state store.
- **Slack**: uses `reqwest` + `tokio-tungstenite` Socket Mode, _not_ `slack-morphism`.
- **Mattermost**: uses `reqwest` + `tokio-tungstenite`, _not_ `mattermost_api`.
- **Signal**: uses a thin `reqwest` + `tokio-tungstenite` client against [signal-cli-rest-api](https://github.com/bbernhard/signal-cli-rest-api). Receives messages via WebSocket `GET /v1/receive/{number}` and sends via `POST /v1/send`. No `presage` dependency; no feature flag required. Operators must run the signal-cli daemon separately.

## Terminology

- Canonical domain terms are defined in `docs/glossary.md`.
- Use `Persona`, `Subagent Process`, and `A2A Profile` in new docs and UX copy.
- Avoid unqualified `agent` in architecture prose unless referring to a literal code identifier.

## Code Style

### Formatting

Default `cargo fmt` (no `rustfmt.toml`). Default clippy with `-D warnings` (all warnings are errors).

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
`clippy::panic` are `warn` at the workspace baseline. Most crates ratchet
them to `deny` at the crate level via a manual replay of the workspace
lint block in their `Cargo.toml` (`[lints.clippy]` + `[lints.rust]`).
Currently ratcheted to `deny`:

- `assistant-a2a-json-schema`, `assistant-auth`, `assistant-backup`,
  `assistant-core`, `assistant-interfaces`, `assistant-llm-provider`,
  `assistant-mcp-client`, `assistant-mcp-server`, `assistant-runtime`,
  `assistant-skills`, `assistant-storage`, `assistant-tool-executor`,
  `assistant-web-ui`, `assistant-workflow-http`,
  `opentelemetry-exporter-iceberg`.

The remaining crates still inherit the workspace `warn` default. Promoting
a crate from `warn` to `deny` is a self-contained follow-up PR: clean the
unwraps in production code, replace `[lints]\nworkspace = true` with the
manual replay block, run `make lint && make test`.

Test code (`#[cfg(test)]` modules and `tests/` directories) is exempt: the
default `cargo clippy --workspace` invocation used by `make lint` and CI
does not check test targets, so `.unwrap()` remains ergonomic in tests.

The enforcement scanner lives at `tests/workspace_lint_policy.rs` and runs as
part of `cargo test -p assistant`. See `openspec/changes/workspace-lint-policy/`
for the originating proposal.

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
| Module files     | `snake_case`       | `skill_registry.rs`, `tool_executor.rs`      |
| Structs          | `PascalCase`       | `ToolExecutor`, `FileReadHandler`            |
| Traits           | `PascalCase`       | `ToolHandler`, `LlmProvider`                 |
| Handler structs  | `<Feature>Handler` | `FileReadHandler`, `BashHandler`             |
| Tool names (str) | `kebab-case`       | `"file-read"`, `"web-fetch"`, `"memory-get"` |
| Skill names      | `kebab-case`       | Must match directory name exactly            |
| Constants        | `SCREAMING_SNAKE`  | `DEFAULT_LIMIT`, `BOOTSTRAP_MAX_CHARS`       |
| Enum variants    | `PascalCase`       | `MessageRole::User`, `Interface::Cli`        |

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

## Adding a Builtin Tool

1. Create `crates/tool-executor/src/builtins/<name>.rs` with a handler struct.
2. Implement `#[async_trait] impl ToolHandler` -- 4 required methods: `name()`, `description()`,
   `params_schema()`, `run()`. Optionally override `is_mutating()`, `requires_confirmation()`,
   or `output_schema()`.
3. Export from `crates/tool-executor/src/builtins/mod.rs`.
4. Register in `ToolExecutor::register_builtins()` in `executor.rs`.
5. Optionally add `skills/<name>/SKILL.md` for documentation.

## Testing Patterns

- Unit tests in `#[cfg(test)] mod tests` at the bottom of each file.
- Use `#[tokio::test]` for all async tests.
- `StorageLayer::new_in_memory()` for test databases (no disk I/O, fully migrated).
- `wiremock` for HTTP mocking (Ollama/provider API responses).
- Helper functions for fixtures: `make_skill()`, `build()`, `mount_answer()`.
- Use `assert_eq!` with descriptive messages as the third argument.

## CI

GitHub Actions runs on push to `main` and PRs: check, test, lint (clippy), format.
All messenger interfaces compile unconditionally as part of the `assistant-interfaces` crate.
Integration tests run with `continue-on-error: true` (require Ollama).
