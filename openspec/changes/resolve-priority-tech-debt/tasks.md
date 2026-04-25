## 1. Slice A — Storage layering inversion

**Implementation note:** During execution we replaced the originally proposed
`MigrationBackupHook`/`BootstrapUserHook` trait approach with a cleaner design
that matches existing patterns: trait contracts (`ApiKeyStore`, `ClientStore`,
`AuthCodeStore`, `RefreshTokenStore`, `DeviceCodeStore`) were moved to
`assistant_core::auth`, and the storage-only side-effect helpers
(`backup_legacy`, `create_admin_user`) were extracted to
`assistant_backup::backup_legacy_install` and
`assistant_auth::bootstrap::create_admin_user` respectively. Production callers
compose the four steps explicitly. This achieves the same dep-boundary outcome
with no extra hook indirection.

- [x] 1.1 Add a failing test in `crates/storage/tests/dep_boundary.rs` asserting that `assistant-storage`'s `[dependencies]` contain only `assistant-core` and `assistant-skills` workspace path deps.
- [x] 1.2 Move auth trait contracts (`ApiKeyStore`, `ClientStore`, `AuthCodeStore`, `RefreshTokenStore`, `DeviceCodeStore`) and their record types (`ApiKeyRecord`, `AuthCode`, `StoredRefreshToken`, `DeviceState`) into `assistant_core::auth`; re-export from `assistant_auth` for back-compat.
- [x] 1.3 Move `backup_legacy` from `storage::migration` into `assistant_backup::backup_legacy_install`.
- [x] 1.4 Move `create_admin_user` + `AdminCredentials` from `storage::migration` into `assistant_auth::bootstrap`; takes `&dyn UserStore`+`&dyn MembershipStore` so auth needn't depend on storage.
- [x] 1.5 Refactor `migrate_database` to return `(OrgStorageLayer, OrgId, SpaceId)` and stop bootstrapping the admin user internally; remove `run_migration` orchestrator.
- [x] 1.6 Update web-ui binary call site to compose the four steps explicitly: `backup_legacy_install` → `migrate_filesystem` → `migrate_database` → `bootstrap::create_admin_user`.
- [x] 1.7 Add `assistant-backup = { path = "../backup" }` to `crates/web-ui/Cargo.toml`.
- [x] 1.8 Drop `assistant-auth` and `assistant-backup` from `crates/storage/Cargo.toml` `[dependencies]`; keep them as `[dev-dependencies]` so the round-trip migration test can compose the production pipeline.
- [x] 1.9 Run `cargo check --workspace`, `cargo test -p assistant-storage`; verify the `dep_boundary` integration test passes.
- [x] 1.10 Document the dep-boundary contract in `crates/storage/src/lib.rs` module-level docs.
- [x] 1.11 Run `make lint` and `make format`; commit as `refactor(storage): invert auth/backup dependency layering`.

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
