## Why

A workspace audit (2026-05-17) found test density is uneven and the bottom of the distribution is driven by structural untestability, not absent intent:

- 2 crates with **0% test coverage** (`mcp-server` 530 LOC, `workflow-http` 232 LOC).
- 17 production files >200 LOC with **zero inline tests**, including `interface-cli/main.rs` (1002), `web-ui/backends/iceberg.rs` (984), `runtime/orchestrator/dispatch.rs` (418), `tool-executor/builtins/bash.rs` (261).
- Median crate ratio is ~30% (test LOC / src LOC); only 4 crates exceed 40%.
- Root causes: load-bearing components (`Orchestrator`, `ToolExecutor`, `SkillRegistry`, `StorageLayer`) are exposed as concrete structs with 20–30 fields, not trait facades. `reqwest::Client::new()`, `Utc::now()`, and `std::env::var` appear in constructors and method bodies across the workspace, leaving no test seams.

Establishing an 80% per-crate **line-coverage floor**, measured by `cargo llvm-cov` and enforced in CI, locks in the discipline. Reaching it requires modest trait-facade refactors first.

## What Changes

- **Adopt `cargo llvm-cov` as the workspace coverage tool**; commit a `coverage.toml` configuration and a `make coverage` target.
- **Add a per-crate 80% line-coverage gate to CI** (`coverage.yml` workflow). Gate fires per-crate, not per-workspace, so a regression in one crate cannot be hidden by gains elsewhere.
- **Extract traits for every persistence component still exposed as a concrete struct** (`ConversationStore`, `ConversationEventStore`, `CommandEventStore`, `TraceStore`, `LogStore`, `MetricsStore`, `AttachmentStore`, `AudioStore`, `AgentStore`, `MemoryChunkStore`, `PersonaStore`, `PersonaSkillAccess`, `PushSubscriptionStore`, `RefinementStore`, `ScheduledTasksStore`, `SlackThreadStore`, `WebhookStore`, `WorkflowStore`). Traits live in `assistant-core`; SQLite implementations stay in `assistant-storage` (or their owning domain crate). Consumers above storage take `Arc<dyn StoreTrait>`.
- **Add in-memory implementations alongside the SQLite ones** in the same crate (typically `assistant-storage`), as plain `pub` types. No `#[cfg(test)]`. No `[features] test-support` gate. They are alternative `Trait` impls — same status as `SqliteConversationStore` or `OllamaProvider` — following the existing pattern set by `InMemoryConversationBroadcaster`.
- **Add a thin `assistant-test-support` crate** under `[dev-dependencies]` of every consumer. It hosts only `FixtureBuilder` and a `prelude` module that re-exports the in-memory impls + the orchestration stubs + `ScriptedLlmProvider`. The actual fakes live in their owning crates. Production binaries do not link `assistant-test-support`.
- **Add a workspace lint test** (`tests/workspace_test_impls_in_prod.rs`) that fails the build when types matching `InMemory*`, `Scripted*`, or `Stub*` are constructed in non-test production code paths, with an exempt list for the defining module, `#[cfg(test)]` modules, `tests/` dirs, the `assistant-test-support` crate, and documented production-fallback sites (e.g., `InMemoryConversationBroadcaster`).
- **Run contract tests per persistence trait**: a single test script under `crates/storage/tests/contract/<trait>.rs` exercises every implementation of the trait (SQLite + in-memory). Drift between impls fails CI.
- **Introduce trait facades in `assistant-core`** for the load-bearing runtime types currently exposed concretely:
  - `OrchestrationEngine` (subset of `Orchestrator` actually consumed by `mcp-server`, `web-ui`, `interfaces`)
  - `ToolDispatcher` (subset of `ToolExecutor`)
  - `SkillCatalog` (subset of `SkillRegistry`)
- **Introduce `Clock` trait in `assistant-core`** with `SystemClock` and `FakeClock` implementations. Replace all 92 non-test `Utc::now()` / `SystemTime::now()` calls with injected clocks.
- **Inject `reqwest::Client` at constructor seams** in `workflow-http`, `web-ui/push`, `nextcloud`, `matrix`, `oidc`, `cmd_login`. Always honor `with_base_url(...)` so `wiremock` can be wired.
- **Extract pure dispatch functions in each messenger `runner.rs`** (Slack, Mattermost, Matrix, Nextcloud, Signal). The WebSocket / long-poll loop stays I/O-only; payload-to-action logic moves to a pure function unit-testable with `serde_json::json!` fixtures.
- **Split `interface-cli/main.rs`**: extract subcommand bodies into `cmd_*.rs` files so `main()` is just argument parsing + dispatch. Existing untested subcommands (`cmd_login`, `cmd_account`, `bootstrap`) get inline test modules.
- **Split `web-ui/backends/iceberg.rs` and `opentelemetry-exporter-iceberg/{span,log,metric}.rs`** to make filtering/aggregation pure functions over `RecordBatch` slices, then add a tiny parquet fixture corpus under `tests/fixtures/warehouse/`.

## Capabilities

### New Capabilities

- `test-coverage-floor` — defines the 80% per-crate line-coverage rule, the `cargo llvm-cov` measurement contract, and the CI enforcement gate.
- `persistence-trait-symmetry` — defines the rule that every persistence component is exposed as a trait, has at least one in-memory implementation co-located with its SQLite peer as plain `pub` (no feature gate), is reachable to tests via a thin `assistant-test-support` composition crate, and is covered by per-trait contract tests; production misuse is caught by a workspace lint.
- `orchestration-trait-seams` — defines the `OrchestrationEngine`, `ToolDispatcher`, `SkillCatalog` traits in `assistant-core` and the rule that consumers MUST take the trait, not the concrete type.
- `clock-abstraction` — defines the `Clock` trait, `SystemClock` + `FakeClock` implementations, and the rule banning direct `Utc::now()` in non-test production code.
- `http-client-injection` — defines the rule that any type making outbound HTTP calls MUST accept an injected client and a configurable base URL.
- `messenger-runner-dispatch` — defines the pure-function dispatch contract for messenger runners.

### Modified Capabilities

None. Existing user-facing specs are unchanged.

## Impact

- **Code**: every crate (the floor applies workspace-wide); concrete refactors in `assistant-core` (new trait definitions), `assistant-storage` (each concrete store gets a `Sqlite<Name>` wrapper around the existing struct), `runtime/orchestrator/`, `mcp-server`, `workflow-http`, `interfaces/*/runner.rs`, `interface-cli/main.rs`, `web-ui/backends/iceberg.rs`, `web-ui/oauth/`, `web-ui/a2a/handlers.rs`, all `llm-provider/*`, `auth/jwt.rs`, `auth/oauth2/device.rs`.
- **New crate**: `assistant-test-support` under `crates/test-support/` (dev-dependency only).
- **Build**: `cargo llvm-cov` adds ~30 s to local `make test` when run with coverage; CI gains a coverage job (~3–5 min on cold cache). `assistant-test-support` adds compile time only to test builds.
- **Runtime**: no behavior changes. Trait facades and store traits are added alongside existing concrete types and adopted incrementally.
- **Tests**: thousands of new LOC. See `tasks.md` for per-crate sequencing; the lowest-coverage crates ship first because they unblock the gate.
- **Dependencies**: `cargo-llvm-cov` as a dev tool (installed via `cargo install` in CI; not a workspace dependency).

## Non-goals

- Mutation testing or branch-coverage targets (line coverage only for now).
- Property-based testing adoption (a possible follow-up).
- Removing `StorageLayer::new_in_memory()` — it stays as the SQLite-backed bootstrap path for `assistant-storage`'s own tests and contract tests; consumers above storage migrate to per-trait in-memory impls but `new_in_memory()` itself is not deleted.
- Splitting the `interfaces` crate into per-messenger crates (separate change; out of scope here).
- Splitting `Orchestrator` itself; only its trait facade is added.
- Splitting `web-ui/api/messages.rs` (2124 LOC) — already passes the 80% bar via test_helpers.
- Backfilling coverage on `assistant-integration-tests` (it is the integration harness, not a production library).
- Migrations and pool factory get no in-memory variant; documented as exemptions in the `persistence-trait-symmetry` spec.

## User-facing documentation

**Not required.** This is an internal engineering-discipline change. The only externally visible artifact is a coverage badge in the README; the CI gate is invisible to users.
