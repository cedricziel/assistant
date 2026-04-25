## 1. Slice A — Storage layering inversion

- [x] 1.1 Add a failing test in `crates/storage/tests/` (or a doc-test in `lib.rs`) asserting that `assistant-storage` exposes only `assistant-core` types in its public API surface (compile-only test that imports from storage and never names `assistant_auth::` / `assistant_backup::`).
- [ ] 1.2 Define `MigrationBackupHook` trait in `crates/storage/src/migration.rs` (method: `async fn pre_migration_backup(&self, db_path: &Path) -> Result<()>`).
- [ ] 1.3 Refactor `Migrator::run` (or equivalent entry point) to take `Option<&dyn MigrationBackupHook>` instead of constructing `BackupEngine` directly.
- [ ] 1.4 Add a `BackupHook` adapter struct in `crates/backup/src/lib.rs` that implements `assistant_storage::MigrationBackupHook` and wraps `BackupEngine`.
- [ ] 1.5 Update CLI binary (`crates/interface-cli/src/main.rs` or equivalent migration call site) to construct the hook and pass it into the migrator.
- [ ] 1.6 Update web-ui binary call site to construct the hook and pass it into the migrator.
- [ ] 1.7 Remove the `assistant_backup::{BackupEngine, BackupOptions}` import from `crates/storage/src/migration.rs`.
- [ ] 1.8 Move `crates/storage/src/api_key_store.rs` to `crates/auth/src/storage/api_key_store.rs`; update `mod.rs` files in both crates; update all `use assistant_storage::ApiKeyStore` imports across the workspace.
- [ ] 1.9 Move `crates/storage/src/auth_state_store.rs` to `crates/auth/src/storage/auth_state_store.rs`; update imports.
- [ ] 1.10 Replace the `assistant_auth::password::hash_password` call inside `crates/storage/src/migration.rs:385` (bootstrap user creation) by either (a) moving the bootstrap-user creation step out of the migration into the auth crate, or (b) accepting a `BootstrapUserHook` closure parameter on the migrator.
- [ ] 1.11 Delete `assistant-auth = { path = "../auth" }` and `assistant-backup = { path = "../backup" }` from `crates/storage/Cargo.toml`.
- [ ] 1.12 Run `cargo build --workspace`, `cargo test --workspace`, `cargo machete --with-metadata`, and `cargo tree -p assistant-storage --no-default-features` to verify only `assistant-core` and `assistant-skills` (intentional carve-out) remain as workspace path deps.
- [ ] 1.13 Make the test from 1.1 pass (compile-only assertion of the new dep boundary).
- [ ] 1.14 Document the dep-boundary contract in `crates/storage/src/lib.rs` module-level docs and reference the spec at `openspec/changes/resolve-priority-tech-debt/specs/storage-layering/spec.md`.
- [ ] 1.15 Run `make lint` and `make format`; commit as `refactor(storage): invert auth/backup dependency layering`.

## 2. Slice B — Request-path error handling

- [ ] 2.1 Add a failing unit test for `crates/web-ui/src/oauth/mod.rs` that drives a malformed-input code path which currently panics via `.unwrap()` and asserts a 4xx `{"error": "..."}` response instead.
- [ ] 2.2 Add `#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used, clippy::panic))]` to the top of `crates/web-ui/src/oauth/mod.rs`.
- [ ] 2.3 Replace every `.unwrap()`/`.expect()` in non-test code in `crates/web-ui/src/oauth/mod.rs` with `?` + `anyhow::Context` or explicit `(StatusCode, Json<ErrorResponse>)` returns. Make the test from 2.1 pass.
- [ ] 2.4 Upgrade `crates/web-ui/src/oauth/mod.rs` lint attribute from `warn` to `deny`.
- [ ] 2.5 Add failing test, warn-then-fix-then-deny cycle for `crates/web-ui/src/auth.rs`.
- [ ] 2.6 Add failing test, warn-then-fix-then-deny cycle for `crates/web-ui/src/a2a/agent_store.rs`.
- [ ] 2.7 Add failing test, warn-then-fix-then-deny cycle for `crates/storage/src/migration.rs` (test asserts a missing-file or schema-mismatch path returns `Err` instead of panicking).
- [ ] 2.8 Apply warn-then-fix-then-deny to `crates/storage/src/conversation_events.rs`.
- [ ] 2.9 Apply warn-then-fix-then-deny to `crates/storage/src/traces.rs`.
- [ ] 2.10 Apply warn-then-fix-then-deny to `crates/storage/src/webhooks.rs`.
- [ ] 2.11 Apply warn-then-fix-then-deny to `crates/web-ui/src/api/mod.rs` (largest file — last; expect significant work).
- [ ] 2.12 Run `cargo clippy -p assistant-web-ui -- -D warnings` and `cargo clippy -p assistant-storage -- -D warnings`; verify zero unwrap/expect/panic clippy violations.
- [ ] 2.13 Run `make lint`, `make format`, `cargo test --workspace`. Commit as `refactor(web-ui,storage): replace panic-prone unwraps in request paths`.

## 3. Slice C — Orchestrator TurnContext

- [ ] 3.1 Add a failing test in `crates/runtime/src/orchestrator/tests.rs` that drives a turn where a tool returns `ToolOutput::error(...)` and asserts the post-turn `turn_had_errors` signal is `true` (currently fails because the variable is hard-coded to `false` at `mod.rs:962`).
- [ ] 3.2 Create `crates/runtime/src/orchestrator/turn_context.rs` with `TurnContext<'a>` (borrowed handles) and `TurnState` (owned mutable state including `turn_had_errors: bool` and a `record_tool_error(&mut self, &ToolError)` method).
- [ ] 3.3 Add `TurnContext::for_test(...)` builder for tests.
- [ ] 3.4 Migrate the entry point at `crates/runtime/src/orchestrator/mod.rs:376` to take `&mut TurnContext<'_>`; remove the `#[allow(clippy::too_many_arguments)]`. Update all call sites.
- [ ] 3.5 Migrate the entry point at `mod.rs:414`. Remove the allow.
- [ ] 3.6 Migrate the entry point at `mod.rs:472`. Remove the allow.
- [ ] 3.7 Migrate the entry point at `mod.rs:497`. Remove the allow.
- [ ] 3.8 Migrate the function at `mod.rs:914`. Remove the allow.
- [ ] 3.9 Migrate `crates/runtime/src/orchestrator/dispatch.rs:100`, `:305`, `:306`. Remove the three allows.
- [ ] 3.10 Migrate `crates/runtime/src/orchestrator/worker.rs:635`. Remove the allow.
- [ ] 3.11 Migrate `crates/runtime/src/orchestrator/turn_control.rs:27` and `:157`. Remove the two allows.
- [ ] 3.12 Wire `record_tool_error(...)` at every dispatch error site in `worker.rs` and `dispatch.rs` (every `ToolOutput::error` and recoverable `Err` from tool dispatch).
- [ ] 3.13 Replace `let turn_had_errors = false; // TODO: track via tool dispatch error signals` at `mod.rs:962` with `let turn_had_errors = ctx.state.turn_had_errors();`.
- [ ] 3.14 Update `tests.rs` so all orchestrator tests construct via `TurnContext::for_test(...)`. Make the test from 3.1 pass.
- [ ] 3.15 Run `cargo clippy -p assistant-runtime -- -D warnings` and confirm zero `#[allow(clippy::too_many_arguments)]` remain in `crates/runtime/src/orchestrator/`.
- [ ] 3.16 Run `make lint`, `make format`, `cargo test -p assistant-runtime`, `cargo test --workspace`. Commit as `refactor(runtime): introduce TurnContext and wire turn_had_errors signal`.

## 4. Verification and archive

- [ ] 4.1 Run `make precommit` on the merged worktree.
- [ ] 4.2 Run `make test-integration` (Ollama-dependent; `continue-on-error: true` is acceptable).
- [ ] 4.3 Update `CLAUDE.md` to remove the stale claim that `matrix-sdk` remains in `Cargo.toml` (already fully removed) — small piggyback fix surfaced by the audit.
- [ ] 4.4 Archive this OpenSpec change with `/opsx:archive resolve-priority-tech-debt` once all three slices have shipped.
