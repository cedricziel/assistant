## Context

Three independent debt items, bundled because they reinforce architectural discipline:

1. **Storage layering inversion.** `crates/storage/Cargo.toml` declares `assistant-auth`, `assistant-backup`, and `assistant-skills` as path deps. Concrete coupling sits in:
   - `crates/storage/src/api_key_store.rs:10` → `assistant_auth::api_keys::{ApiKeyRecord, ApiKeyStore}`
   - `crates/storage/src/auth_state_store.rs:11-13` → `assistant_auth::oauth2::*`
   - `crates/storage/src/migration.rs:22, 385` → `assistant_backup::{BackupEngine, BackupOptions}` and `assistant_auth::password::hash_password`
   - `crates/storage/src/registry.rs:4, 73, 160` → `assistant_skills::*`
     The dependency direction documented in AGENTS.md is `auth -> storage -> core` and `runtime -> storage -> core`. The current arrangement is cyclic in spirit (storage depends on auth and auth depends on storage).

2. **Panic-prone production paths.** 2,870 unwraps across 152 files; the dangerous ones cluster in the OAuth/auth/migration request paths after the recent OAuth2 hardening (commit 777d1431). Hotspot counts: `web-ui/api/mod.rs` 171, `web-ui/oauth/mod.rs` 93, `web-ui/auth.rs` 65, `storage/migration.rs` 54, `storage/conversation_events.rs` 44, `storage/traces.rs` 44, `storage/webhooks.rs` 38, `web-ui/a2a/agent_store.rs` 36.

3. **Orchestrator argument explosion.** 10 `#[allow(clippy::too_many_arguments)]` annotations in `crates/runtime/src/orchestrator/{mod,worker,dispatch,turn_control}.rs` (mod.rs L376/414/472/497/914, dispatch.rs L100/305/306, worker.rs L635, turn_control.rs L27/157). The TODO at `mod.rs:962` (`let turn_had_errors = false; // TODO: track via tool dispatch error signals`) is a correctness regression — the variable is wired to downstream logic but never set to `true`.

## Goals / Non-Goals

**Goals:**

- `assistant-storage` declares only `assistant-core` as a workspace path dependency.
- Zero `.unwrap()`/`.expect()` in non-test code under the in-scope web-ui and storage modules, enforced by clippy at module level.
- Zero `#[allow(clippy::too_many_arguments)]` in `crates/runtime/src/orchestrator/`.
- `turn_had_errors` reflects real tool-dispatch error state.

**Non-Goals:**

- Splitting `web-ui/api/mod.rs` (separate change).
- Auditing unwraps in messenger interface crates or test code.
- Closing out unrelated OpenSpec proposals.
- Resolving duplicate dependency versions.

## Decisions

### D1: Move auth-coupled stores up, not down

Three of the four storage→auth coupling points (`api_key_store.rs`, `auth_state_store.rs`, plus the password-hash use in `migration.rs:385`) are conceptually owned by auth. **Move these files into `crates/auth/src/storage/`**, keeping the SQLite backend implementation but flipping the import direction. `assistant-auth` already depends on `sqlx` transitively; making it explicit costs little.

The `BackupEngine` use in `migration.rs:22` runs the bootstrap backup before applying schema migrations. **Invert via a callback trait** — define `trait MigrationBackupHook` in `assistant-storage` (or `assistant-core`), have `assistant-backup` implement it, and have the migration entry point accept `Option<&dyn MigrationBackupHook>`. The CLI/web-ui binaries (which already depend on both crates) wire the implementation.

The `assistant-skills` dep is benign — the skills crate is leaf-like and AGENTS.md does not place it below storage. **Leave it as-is** unless a concrete reason emerges; document the exception in the spec scope.

**Alternatives considered:** (a) Define traits in `assistant-core` and keep stores in storage — rejected because the trait surface (token rotation, refresh-token state) is large and auth-specific, leaking auth domain into core. (b) New thin `assistant-auth-storage` crate — rejected for now; the existing `assistant-auth` crate already contains query-shaped code, so co-locating is simpler.

### D2: Module-level clippy denies, file-by-file rollout

Add `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]` at the top of each in-scope module (or use `#![warn(...)]` then upgrade to `deny` when the file is clean). This makes the policy local — we do not flip workspace-wide lints and force every other crate to comply at once.

Each file is migrated independently:

1. Add `#![warn(clippy::unwrap_used, clippy::expect_used)]` to the file.
2. Run clippy, fix every warning by introducing `?` + `anyhow::Context` or returning `{"error": ...}` envelopes.
3. Upgrade to `#![deny(...)]`.

Test code is exempt because the lints are gated on `not(test)` (or use `#[cfg(test)] #[allow(...)]` on the test module).

**Alternatives considered:** workspace-wide `[lints.clippy]` in `Cargo.toml` — rejected because we'd need an all-or-nothing migration and the messenger interface crates are out of scope. Per-module attributes give us a clean, incremental path.

### D3: TurnContext as a borrowed bundle, not a god struct

Define `TurnContext<'a>` in `crates/runtime/src/orchestrator/turn_context.rs` holding borrowed handles to per-turn dependencies (LLM provider, tool registry, persona, conversation ID, span/trace context, etc.) plus a small owned `TurnState` for mutating fields like `turn_had_errors` and per-turn counters.

Function signatures change from `fn turn(a, b, c, d, e, f, g)` to `fn turn(ctx: &mut TurnContext<'_>)`. The 10 `too_many_arguments` allows are deleted.

Tool dispatch gains a `record_tool_error(&mut self, err: &ToolError)` method on `TurnContext` (or `TurnState`). The dispatch sites in `worker.rs` and `dispatch.rs` call it on every `ToolOutput::error` or recoverable `Err`. `mod.rs:962` reads the recorded value instead of hard-coding `false`.

**Alternatives considered:** (a) Owned `TurnContext` — rejected because per-turn dependencies (LLM provider, tool registry) live for the whole conversation and we should not clone Arcs for every turn. Borrowed handles are cheaper and clearer. (b) Builder pattern — adds ceremony with no payoff for an internal struct.

## Risks / Trade-offs

- **[Storage move ripples through CLI/web-ui imports]** → mitigation: `cargo check --workspace` after each file move; the moved types keep the same names so call sites only update the `use` path.
- **[Auth crate compile time grows]** → acceptable; storage compile time shrinks correspondingly. Net workspace build time should be unchanged or slightly better.
- **[Module-level deny lints surface unrelated unwraps in shared helpers]** → mitigation: file-by-file rollout. If a helper used by multiple in-scope files has unwraps, fix the helper but only enforce the lint on files in scope.
- **[`TurnContext` refactor breaks the 3,374-line `orchestrator/tests.rs` suite]** → mitigation: introduce `TurnContext::for_test(...)` builder first and migrate tests in the same PR as the production refactor. Run `cargo test -p assistant-runtime` between every signature change.
- **[`MigrationBackupHook` callback adds indirection for a single caller]** → acceptable; the indirection is the point. It documents the layering contract.

## Migration Plan

Three independent slices, each landable separately:

1. **Slice A — Storage layering (PR 1, ~M):**
   a. Add `MigrationBackupHook` trait in `assistant-storage`; refactor `migration.rs` to accept it.
   b. Move `assistant_auth::password::hash_password` call out of `migration.rs:385` into `assistant-auth` (or pass a closure).
   c. Move `api_key_store.rs` and `auth_state_store.rs` from `assistant-storage` to `assistant-auth`.
   d. Update CLI/web-ui binaries to wire the new `MigrationBackupHook`.
   e. Drop `assistant-auth` and `assistant-backup` from `crates/storage/Cargo.toml`.
   f. `cargo machete` + `cargo build --workspace` + full test suite.

2. **Slice B — Error handling (PR 2, ~M):**
   a. Add module-level `#![warn(clippy::unwrap_used, clippy::expect_used)]` to each in-scope file.
   b. Fix one file per commit: `web-ui/oauth/mod.rs` first (highest panic risk), then `web-ui/auth.rs`, `web-ui/a2a/agent_store.rs`, `storage/migration.rs`, `storage/{conversation_events,traces,webhooks}.rs`, `web-ui/api/mod.rs` last.
   c. Upgrade each file to `#![deny(...)]` once clean.
   d. Add a CI job (or rely on the per-file deny) to prevent regression.

3. **Slice C — TurnContext (PR 3, ~M-L):**

   **Status (2026-04):** the _error-signal_ half is **already shipped** in
   the same PR as Slices A/B — concretely:
   - `TurnResult.had_errors: bool` field added (`mod.rs:54`).
   - `FinalizedTool { had_error }` returned from `dispatch.rs::finalize_tool_result`.
   - `DispatchOutcome::Executed { had_error }` named-field variant.
   - `turn_had_errors` accumulator threaded through `run_turn_with_tools_impl`
     and `run_turn_core`, propagated through `handle_final_answer_with_extensions`
     (extension-tools path) and `bus_messages::TurnResult` (worker bus path).
   - Unit tests `run_turn_marks_had_errors_when_tool_fails` and
     `run_turn_clears_had_errors_when_no_tool_fails` cover the contract.

   **Still remaining for a follow-up Slice C PR** (deferred — significant
   public-API churn, aesthetic refactor):
   a. Introduce `TurnContext<'a>` and `TurnState`. Add `for_test(...)` builder.
   b. Migrate `mod.rs` entry points first, then `dispatch.rs`, `worker.rs`, `turn_control.rs`. Delete each `#[allow(clippy::too_many_arguments)]` as the function it guards is migrated.

Rollback: each slice is a single PR; revert is `git revert`. No data migrations, no flag flips.

## Open Questions

- Should `assistant-skills` be moved into the same scope as a fourth slice? (Currently out of scope; AGENTS.md does not forbid the dep.)
- Do we want a `clippy::indexing_slicing` ban in the same scope as Slice B? (Likely yes for OAuth code, but adds work — defer to a follow-up unless a panic surfaces during the audit.)
- Where does `MigrationBackupHook` live — `assistant-storage` (closer to caller) or `assistant-core` (more reusable)? Default to `assistant-storage` and revisit if a second consumer appears.
