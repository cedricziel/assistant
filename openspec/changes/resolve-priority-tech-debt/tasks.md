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

## 3. Slice C — Orchestrator turn_had_errors signal

**Implementation note:** The original prescription called for a `TurnContext<'a>` /
`TurnState` struct and migrating eight public-API entry points to take
`&mut TurnContext<'_>`, removing all `#[allow(clippy::too_many_arguments)]`
attributes from `crates/runtime/src/orchestrator/`. On audit this turned out to
be a much larger change than the actual bug warranted: those entry points
(`run_turn_with_tools`, `run_turn_with_tools_streaming`, `run_turn`,
`run_turn_streaming`, plus `run_turn_with_tools_impl`/`run_turn_core`) are
public APIs called from `assistant-cli`, `assistant-web-ui`, `assistant-interfaces`,
and the integration-tests crate — and the parameters they receive
(`user_message`, `conversation_id`, `interface`, `extensions`, …) are caller-
provided inputs, not internal state to be hidden behind a context.

The _bug_ Slice C addresses is at `mod.rs:962`:

```rust
let turn_had_errors = false; // TODO: track via tool dispatch error signals
```

This stub disables the post-turn `had_errors` signal that gates skill-learner
evaluation. Slice C therefore pivots to a focused fix: extend
`DispatchOutcome` to carry an `error` flag, accumulate it in the tool-calling
loop, and feed it into the `skill_learner::TurnContext`. The `too_many_arguments`
parameter-list cleanup is deferred to a separate change — it's an aesthetic
refactor with significant API churn, while the `turn_had_errors` wiring is a
single-call-site bug fix.

- [x] 3.1 Add a failing unit test in `crates/runtime/src/orchestrator/tests.rs` that drives a turn where a tool returns `Err(...)` and asserts the post-turn `had_errors` signal observable on `TurnResult` is `true`. Added `run_turn_marks_had_errors_when_tool_fails` and `run_turn_clears_had_errors_when_no_tool_fails`.
- [x] 3.2 Extend `DispatchOutcome` with a `had_error: bool` field — chose the `Executed { had_error: bool }` named-field variant for readability at call sites.
- [x] 3.3 Update `finalize_tool_result` in `dispatch.rs` to return a new `FinalizedTool { had_error }` struct instead of bare `String` — observation is recorded into history internally so callers don't need it back.
- [x] 3.4 Accumulate the error flag in `run_turn_with_tools_impl` (extension-tools loop and global executor branch) and `run_turn_core` (global executor branch) into a `turn_had_errors` mutable bool.
- [x] 3.5 Replace `let turn_had_errors = false; // TODO: ...` at `mod.rs:965` with the accumulated value, and surface it on `TurnResult.had_errors`. Tests pass.
- [x] 3.6 Run `cargo test -p assistant-runtime` (185/185 pass) and `cargo clippy -p assistant-runtime --tests --no-deps -- -D warnings` (the only failures are 3 pre-existing toolchain-drift lints in `otel_spans.rs` / `scheduler.rs` — confirmed they reproduce on the base branch with no Slice C edits applied). Committed as `fix(runtime): wire turn_had_errors signal through tool dispatch`.
- [x] 3.7 Follow-up: the `#[allow(clippy::too_many_arguments)]` attributes in `crates/runtime/src/orchestrator/` (e.g. `dispatch_global_tool`, `finalize_tool_result`, `run_turn_with_tools_impl`, `run_turn_core`) remain. They are an aesthetic refactor with significant public-API churn — deferred to a future dedicated change ("orchestrator: introduce TurnContext to collapse parameter lists") rather than mixed into this bug fix.

## 4. Verification and archive

- [x] 4.1 Run `make precommit` on the merged worktree — all checks green (765/765 Flutter tests pass, Rust fmt + clippy + machete all clean).
- [ ] 4.2 Run `make test-integration` (Ollama-dependent; `continue-on-error: true` is acceptable).
- [x] 4.3 Update `AGENTS.md` to remove the stale claim that `matrix-sdk` remains in `Cargo.toml` (already fully removed). The line lived in `AGENTS.md` (referenced from `CLAUDE.md`), not in `CLAUDE.md` itself.
- [ ] 4.4 Archive this OpenSpec change with `/opsx:archive resolve-priority-tech-debt` once all three slices have shipped.
