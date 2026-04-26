## Why

A tech-debt audit surfaced three issues that compound risk and slow delivery: `assistant-storage` violates the dependency order documented in AGENTS.md (storage → auth/backup), 100+ `.unwrap()` calls in the OAuth/auth/migration request paths can panic the web server, and the orchestrator's ReAct loop carries five `#[allow(clippy::too_many_arguments)]` allows that block refactor work — including a silently disabled error-tracking signal at `crates/runtime/src/orchestrator/mod.rs:962`.

Tackling these together establishes durable architectural boundaries before they harden further.

## What Changes

- **Invert the storage layering**: remove `assistant-auth` and `assistant-backup` dependencies from `crates/storage/Cargo.toml`. Move auth/backup-coupled types up the dependency tree (likely into `assistant-web-ui` or new thin crates), or invert with traits defined in `assistant-core`. **POTENTIALLY BREAKING for direct re-exporters** — this PR preserves back-compat re-exports (e.g. `pub use assistant_core::auth::{DeviceCodeStore, DeviceState};` in `crates/auth/src/oauth2/device.rs`) so external consumers of `assistant_auth::oauth2::*` keep compiling. The `BREAKING` label applies once those shims are removed in a follow-up.
- **Replace panic-prone `.unwrap()`/`.expect()` in production request paths** for `crates/web-ui/src/api/mod.rs`, `crates/web-ui/src/oauth/mod.rs`, `crates/web-ui/src/auth.rs`, `crates/web-ui/src/a2a/agent_store.rs`, `crates/storage/src/migration.rs`, `crates/storage/src/conversation_events.rs`, `crates/storage/src/traces.rs`, `crates/storage/src/webhooks.rs`. Convert to `?` + `anyhow::Context` or explicit error envelopes. Test-only `.unwrap()` is out of scope.
- **Introduce `TurnContext` struct in `assistant-runtime`** to bundle the parameters threaded through the ReAct loop. Remove the five `#[allow(clippy::too_many_arguments)]` allows in `crates/runtime/src/orchestrator/{mod,worker,dispatch,turn_control}.rs`. Wire real `turn_had_errors` tracking through tool dispatch error signals (resolves the TODO at `orchestrator/mod.rs:962`).

## Capabilities

### New Capabilities

- `storage-layering`: enforces the dependency contract that `assistant-storage` depends only on `assistant-core`. Auth/backup-specific persistence lives in higher crates.
- `request-path-error-handling`: bans `.unwrap()`/`.expect()`/`panic!()` in non-test code under web-ui and storage migration/event paths; mandates structured error propagation.
- `orchestrator-turn-context`: defines the `TurnContext` shape that consolidates per-turn ReAct loop state and the error-tracking contract for tool dispatch.

### Modified Capabilities

None. These are internal architectural concerns; no existing user-facing spec changes.

## Impact

- **Code**: `crates/storage/`, `crates/auth/`, `crates/backup/`, `crates/web-ui/`, `crates/runtime/orchestrator/`, possibly new thin crates for moved types.
- **Build**: removing `auth`/`backup` from storage shrinks compile graph for CLI/runtime targets and unblocks future headless deployments.
- **Runtime**: error-handling refactor eliminates panic vectors in the auth request path. No behavior change for happy paths.
- **Tests**: orchestrator tests in `crates/runtime/src/orchestrator/tests.rs` (3,374 lines) need signature updates when introducing `TurnContext`.
- **Dependencies**: no new external deps.

## Non-goals

- Splitting the 4,299-line `web-ui/api/mod.rs` god module (separate change).
- Splitting the 2,000+ line Flutter chat/workflow screens (separate change).
- Auditing `.unwrap()` in test code or in messenger interface crates.
- Closing out stale OpenSpec proposals (`multi-user-orgs`, `share-files-native`).
- Resolving duplicate dependency versions in `Cargo.lock`.

## User-facing documentation

**Not required.** All changes are internal: dependency graph, error handling discipline, and an internal struct refactor. No API surface changes, no UX changes, no user-observable behavior changes beyond fewer panics.
