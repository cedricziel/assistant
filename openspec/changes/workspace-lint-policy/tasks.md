## 1. Slice A — Workspace lint table + per-crate inheritance

- [x] 1.1 Failing scanner test at `tests/workspace_lint_policy.rs` asserts `[workspace.lints.clippy]` keys and that every workspace member's `Cargo.toml` declares a `[lints]` table. Confirmed RED.
- [x] 1.2 Added `[workspace.lints]` block to root `Cargo.toml` with `clippy::unwrap_used = "warn"`, `clippy::expect_used = "warn"`, `clippy::panic = "warn"`, `clippy::unimplemented = "warn"`, `clippy::todo = "warn"`, and `rust::unused_must_use = "deny"`.
- [x] 1.3 Added `[lints]\nworkspace = true` to every workspace member (21 crates + root crate).
- [x] 1.4 Scanner tests for slice A are GREEN.

## 2. Slice B — Per-crate ratchet overrides

- [x] 2.1 Ran `cargo clippy --workspace`, tallied violations per file. Eight crates contain non-test unwrap/expect/panic in production code: `core`, `tool-executor`, `web-ui`, `backup`, `auth`, `runtime`, `llm-provider`, `interfaces`.
- [x] 2.2 **Implementation note:** Cargo rejects combining `[lints]\nworkspace = true` with per-crate `[lints.clippy]` overrides in the same manifest (error: `cannot override workspace.lints in lints`). The override approach was changed: dirty crates _drop_ `workspace = true` and **manually replay** the workspace baseline plus their allow overrides. Scanner test `every_workspace_member_declares_a_lints_table` updated to accept either inheritance OR explicit replay (but not both simultaneously).
- [x] 2.3 Each of the 8 dirty crates received an explicit `[lints.clippy]` + `[lints.rust]` block: `unwrap_used = "allow"`, `expect_used = "allow"`, `panic = "allow"`, `unimplemented = "warn"`, `todo = "warn"`, `unused_must_use = "deny"`, with a `# TODO(workspace-lint-policy)` comment recording the ratchet target.
- [x] 2.4 `make lint` is GREEN.

## 3. Slice C — Migrate existing per-file deny attributes to crate-level

- [x] 3.1 Scanner test `deny_level_crates_enforce_panic_free_contract` asserts `crates/storage/Cargo.toml` declares deny-level for `unwrap_used`, `expect_used`, `panic`. Scanner test `deny_level_source_files_do_not_duplicate_panic_free_attribute` asserts `crates/storage/src/lib.rs` no longer contains a `deny(clippy::unwrap_used, ...)` block. Both confirmed RED.
- [x] 3.2 **Scope revised:** Only `crates/storage` migrates to crate-level deny in this change. `crates/web-ui` has 9 violations in non-deny files (`build.rs` + `src/a2a/handlers.rs`) so the file-level `#![cfg_attr(not(test), deny(...))]` blocks on its 4 hot paths remain intact. web-ui will ratchet to crate-level deny in a follow-up after those files are cleaned.
- [x] 3.3 Added `[lints.clippy]` deny overrides to `crates/storage/Cargo.toml` (manual replay shape: `unwrap_used = "deny"`, `expect_used = "deny"`, `panic = "deny"`, `unimplemented = "warn"`, `todo = "warn"`). Removed the `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]` block from `crates/storage/src/lib.rs:25` and added a one-sentence pointer in its module docs to the Cargo-level contract.
- [x] 3.4 Scanner tests GREEN; `make lint` GREEN; `cargo test -p assistant-storage --lib` passes (234 tests).

## 4. Slice D — Tests-can-still-use-unwrap guarantee

- [x] 4.1 **Resolved empirically.** `make lint` runs `cargo clippy --workspace -- -D warnings` without `--all-targets`, and `.github/workflows/ci.yml` matches. Default `cargo clippy` does not compile `#[cfg(test)]` modules, so `unwrap_used` in test code is invisible to the gate. No `cfg_attr` exemption is needed at the crate level.
- [x] 4.2 The 234 existing tests in `assistant-storage` (the only deny-level crate) continue to pass; none of them tripped the deny because they live under `#[cfg(test)] mod tests` blocks.
- [x] 4.3 Mechanism documented in `AGENTS.md` "Code Style → Lint policy".
- [x] 4.4 Verification 6.1 and 6.2 below confirm the deny fires on new prod unwraps while leaving test code untouched.

## 5. Slice E — Documentation + commit

- [x] 5.1 Added "Lint policy" subsection to `AGENTS.md` under "Code Style". Names workspace lint table, lists `assistant-storage` as the sole deny-level crate, explains the inherit-vs-manual-replay shapes, points at the scanner test path and the OpenSpec change.
- [x] 5.2 Added pointer in `crates/storage/src/lib.rs` module docs noting the Cargo-level enforcement.
- [x] 5.3 `make lint` GREEN. `cargo fmt --all` clean. Policy scanner tests GREEN (4/4). `cargo machete` clean.
- [ ] 5.4 Commit as `chore(workspace): introduce workspace-wide lint policy with per-crate ratchet`.

## 6. Verification

- [x] 6.1 Confirmed: inserting `fn _wlp_canary(r: Result<u8, String>) -> u8 { r.unwrap() }` into `crates/storage/src/conversations.rs` produces `error: used `unwrap()`on a`Result` value` and fails `cargo clippy -p assistant-storage`.
- [x] 6.2 Confirmed: inserting the same canary into `crates/runtime/src/orchestrator/dispatch.rs` produces no warning under `cargo clippy -p assistant-runtime -- -D warnings`; build is green (allow-level crate).
- [x] 6.3 `assistant-storage` test suite (234 tests) passes. `cargo build --workspace` is implicitly green via successful `cargo clippy --workspace`.
