## Why

The `resolve-priority-tech-debt` change locked down panics in 5 hot files (`storage/lib.rs`, `web-ui/auth.rs`, `web-ui/oauth/mod.rs`, `web-ui/a2a/agent_store.rs`, `web-ui/api/mod.rs`) via per-file `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]` attributes. That was a one-shot cleanup of the highest-risk surfaces.

The other 282 source files in the workspace have no policy. There are ~2,600 `.unwrap()` calls outside tests. The runtime ReAct loop, the LLM provider clients, the channel runners, the scheduler, and the interface adapters — all the longest-running, hardest-to-recover paths — can still acquire panic vectors on any PR without anything flagging it.

The fix is structural: promote the proven per-file pattern to a `[workspace.lints]` policy (Rust 1.74+ feature), set the default to `warn`, and ratchet per-crate to `deny` as each is cleaned. No bulk refactor needed up front — the policy gate prevents _new_ unwraps while leaving existing ones visible as warnings until tackled.

## What Changes

- **Add `[workspace.lints]` to root `Cargo.toml`** declaring `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::unimplemented`, `clippy::todo` at level `warn`, plus `rust::unused_must_use = "deny"`.
- **Add `[lints] workspace = true` to every crate's `Cargo.toml`** (21 crates).
- **Per-crate allow-list ratchet**: crates that currently contain unwraps in non-test code get `[lints.clippy] unwrap_used = { level = "allow", priority = 1 }` overrides, recorded with a TODO comment naming the crate. Each follow-up cleanup PR removes one allow.
- **Migrate the 5 existing per-file deny attributes to crate-level `deny` overrides** in `crates/storage/Cargo.toml` and `crates/web-ui/Cargo.toml`, then delete the `#![cfg_attr(not(test), deny(...))]` attribute blocks from those source files.
- **Keep `make lint` green** at every step: `cargo clippy --workspace -- -D warnings` must continue to pass after the initial PR by virtue of the per-crate `allow` overrides.

## Capabilities

### New Capabilities

- `workspace-lint-policy`: defines the workspace-wide clippy lint table, the per-crate inheritance contract, and the ratchet protocol (allow → deny) for raising the panic-free baseline incrementally.

### Modified Capabilities

- `request-path-error-handling` (from `resolve-priority-tech-debt`): the per-file `deny(clippy::unwrap_used, ...)` attributes are replaced by crate-level overrides. The enforcement guarantee is preserved; only the mechanism changes.

## Impact

- **Code**: root `Cargo.toml` + 21 per-crate `Cargo.toml` files + 5 source files have their `cfg_attr` block removed.
- **Build**: minor — adds one `[lints]` resolution step. No runtime impact.
- **CI**: `make lint` continues to pass. New PRs that add `.unwrap()` to a deny-level crate fail clippy.
- **Future work**: each `level = "allow"` override is a tracked cleanup target. Ratcheting one crate from allow to deny is a self-contained PR.
- **Dependencies**: none. Cargo 1.74+ already required by the toolchain pin (1.95.0).

## Non-goals

- Cleaning up the ~2,600 existing `.unwrap()` calls. This proposal sets the _gate_; the cleanups are tracked follow-ups, one per crate.
- Splitting giant files (`web-ui/api/mod.rs`, `core/types.rs`, `interface-cli/main.rs`) — separate change.
- The `web-ui/lib.rs = include!("main.rs")` smell — separate change.
- Auditing `#[async_trait]` usages — separate change.
- Adding lints beyond the panic-vector set (e.g. `clippy::pedantic`) — out of scope.

## User-facing documentation

**Not required.** Internal build-policy change. No API surface, behavior, or UX changes. The relevant developer guidance in `AGENTS.md` already states "Default clippy with `-D warnings`"; one sentence will be added pointing contributors at the workspace lint table and the per-crate ratchet status.
