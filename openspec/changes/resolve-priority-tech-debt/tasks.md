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

**Implementation note:** When this slice was scoped, the OpenSpec change anticipated
that hot-path files in `crates/web-ui` and `crates/storage` would contain numerous
panic-prone `.unwrap()`/`.expect()` calls in production code. Audit at apply time
revealed that recent hardening (notably PR #653 OAuth2 hardening and earlier
clippy passes) had already eliminated almost all of them. The remaining
non-test panic-prone calls are `Response::builder()...body(Body::empty()).unwrap()`
and `cookie.parse().unwrap()` — both **infallible by construction** in their
current call sites (static body, cookie string built from a JWT we just signed).

Slice B therefore pivots to: **lock in the current cleaned state with file-level
clippy deny attributes, and refactor the few remaining infallible `.unwrap()`s
into idiomatic axum patterns that don't trip the lint** (`Response::new(...)`,
`(StatusCode, body).into_response()`, `Redirect::to(url)`, header-value
coercion via `From`). This is the same dep-boundary-style guardrail Slice A
established for storage's Cargo.toml — a regression-prevention contract enforced
by `cargo clippy -- -D warnings`.

- [x] 2.1 Audit `crates/web-ui/src/oauth/`, `crates/web-ui/src/auth.rs`, `crates/web-ui/src/a2a/agent_store.rs`, `crates/web-ui/src/api/mod.rs`, `crates/storage/src/{migration,conversation_events,traces,webhooks}.rs` for non-test `.unwrap()`/`.expect()`/`panic!` calls. Result: only `oauth/token.rs` (2) and `auth.rs` (8) have non-test occurrences; all are infallible-by-construction.
- [x] 2.2 Refactor `crates/web-ui/src/oauth/token.rs`: replace `cookie.parse().unwrap()` with a helper that returns `HeaderValue` directly (or use `match` + 500 fallback). Add `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]` to `crates/web-ui/src/oauth/mod.rs`.
- [x] 2.3 Refactor `crates/web-ui/src/auth.rs`: replace each `Response::builder()...body(Body::empty()).unwrap()` with `(StatusCode, [(h, v)], Body::empty()).into_response()` or `Redirect::to(url).into_response()`. Add the same `deny` attribute to the file head.
- [x] 2.4 Add the `deny` attribute to `crates/web-ui/src/a2a/agent_store.rs` (no fixes needed — already clean).
- [x] 2.5 Add the `deny` attribute to `crates/web-ui/src/api/mod.rs`. The deny propagated to submodules and caught two infallible `.expect()` calls in `api/webhooks.rs` (HMAC-SHA256 key construction and OS RNG fill); both are documented invariants — added per-function `#[allow]` with `reason = "..."`.
- [x] 2.6 Apply crate-level `#![cfg_attr(not(test), deny(...))]` to `crates/storage/src/lib.rs` (broader than per-file but storage was already clean — zero fixes needed).
- [x] 2.7 Storage covered by 2.6.
- [x] 2.8 Storage covered by 2.6.
- [x] 2.9 Storage covered by 2.6.
- [x] 2.10 Run `cargo clippy -p assistant-web-ui --no-deps` and `cargo clippy -p assistant-storage --no-deps`; both clean.
- [x] 2.11 Run `make lint`, `make format`, `cargo test --workspace` — all clean (1365 tests pass). Committed atomically per-concept along the way (oauth/token + lint, auth.rs + lint, api/a2a + webhooks fixups, storage crate deny).

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
