## 1. Audit hardcoded `assistant.db` references

- [x] 1.1 `grep -rn '"assistant.db"' crates/` and inventory every literal — split
      into "runtime path" (must move to `OrgPoolFactory`), "legacy fallback"
      (kept under deprecation), and "log/error message" (cosmetic).
- [x] 1.2 Updated the module-level doc-comment at the top of
      `crates/storage/src/pool_factory.rs` with a `## Production callers`
      section enumerating the four production call sites
      (interface-cli/main.rs, interface-cli/cmd_migrate.rs,
      web-ui/main.rs, storage/migration.rs).

## 2. Wire orchestrator to `OrgPoolFactory`

- [ ] 2.1 Write failing integration test in
      `crates/integration-tests/tests/smoke.rs` that boots the orchestrator
      against a tempdir, sends one message, and asserts `space.db` exists with
      a non-empty `messages` table while `assistant.db` does not exist.
      _(Deferred — existing smoke tests require Docker/Ollama; the path
      resolution change is covered by the install.rs unit tests, which assert
      that after migration `assistant.db` is renamed to `.legacy` and only
      `space.db` remains. A full orchestrator-boot smoke test belongs with
      task 9 dry-run.)_
- [x] 2.2 Replace the hard-coded `assistant_dir.join("assistant.db")` in
      `crates/interface-cli/src/main.rs` with a call to
      `OrgPoolFactory::space_db_path("default", "default")`.
- [x] 2.3 Preserve `config.storage.db_path` as a deprecated dev-only override:
      log a `warn!` when set, and bypass the factory only in that case.
- [ ] 2.4 Confirm the integration test from 2.1 now passes. _(Deferred with 2.1.)_

## 3. Add legacy-layout migration to orchestrator startup

- [x] 3.1 Extract the `is_legacy_layout` → backup → migrate-fs → migrate-db →
      bootstrap-admin block from `crates/web-ui/src/main.rs:194-255` into a
      shared helper. Lives at `assistant_web_ui::install::ensure_migrated`
      rather than `assistant_storage::migration::ensure_migrated` because
      `assistant-storage` keeps `assistant-auth` and `assistant-backup` as
      `dev-dependencies` only; web-ui is the lowest common point that already
      links all three crates and is consumed by interface-cli as a library.
- [x] 3.2 Write failing unit test in `crates/web-ui/src/install.rs` that
      asserts `ensure_migrated` is idempotent on an already-migrated layout.
- [x] 3.3 Call `ensure_migrated` from the orchestrator startup path in
      `crates/interface-cli/src/main.rs` before `StorageLayer::new`.
- [x] 3.4 Update `crates/web-ui/src/main.rs` to call the shared helper instead
      of inlining the steps.

## 4. Wire web-ui to `OrgPoolFactory`

- [ ] 4.1 Write failing test asserting `assistant webui serve` opens `space.db`
      not `assistant.db` (mirrors task 2.1 against the web-ui binary entry).
      _(Deferred with 2.1 — requires Docker/Ollama smoke harness.)_
- [x] 4.2 Replace `StorageLayer::new(&db_path)` in `crates/web-ui/src/main.rs`
      with the factory call. Production hosts now resolve runtime data via
      `OrgPoolFactory::space_db_path("default", "default")`; explicit
      `--db-path` is preserved as a deprecated dev/test override behind a
      `warn!`.
- [x] 4.3 Confirm existing web-ui auth/oauth tests still pass (228 tests
      green, including the new `install::tests::*`).

## 5. Atomic cutover in `migrate_database`

- [x] 5.1 Write failing unit test in `crates/storage/src/migration.rs` that
      runs `migrate_database` against a tempdir with `assistant.db`,
      `assistant.db-shm`, `assistant.db-wal` and asserts post-conditions:
      `space.db` exists, `assistant.db.legacy` exists, the three legacy paths
      no longer exist.
- [x] 5.2 After the existing copy step in `migrate_database`, add
      `tokio::fs::rename(assistant.db, assistant.db.legacy)` and remove the
      `*-shm` / `*-wal` sidecars.
- [x] 5.3 Add a failure-mode test: when the copy fails, neither the rename nor
      the sidecar removal happens.
- [x] 5.4 Confirm tests pass.

## 6. `assistant migrate finalize` subcommand

- [x] 6.1 Add `Migrate { command: MigrateCommand }` to the top-level CLI enum
      in `crates/interface-cli/src/main.rs`, with a `Finalize { force: bool }`
      variant. Dispatched before `ensure_migrated` so operators can recover
      stuck installs without the migration helper short-circuiting first.
- [x] 6.2 Create `crates/interface-cli/src/cmd_migrate.rs` with
      `cmd_migrate_finalize(force: bool, base_path: &Path)`.
- [x] 6.3 Write failing unit tests for the running-process detection helper
      (`detects_orchestrator_in_cmdline`, `detects_webui_in_cmdline`,
      `ignores_unrelated_processes`, `detects_multiple_running_services`).
- [x] 6.4 Implement detection using `sysinfo` (added to workspace deps and to
      `crates/interface-cli/Cargo.toml`).
- [x] 6.5 Implement WAL checkpoint via
      `sqlx::query("PRAGMA wal_checkpoint(TRUNCATE);")`.
- [x] 6.6 Implement copy + rename + sidecar removal, mirroring task 5.2.
- [x] 6.7 Implement no-op branch when `assistant.db` is already absent —
      covered by `finalize_is_noop_when_legacy_db_absent`.
- [x] 6.8 Write integration tests covering: success path, services-running
      refusal, `--force` override, no-op path. Implemented by exposing
      `cmd_migrate_finalize_with_detector` so a fake process detector can
      simulate live orchestrator/webui processes without spawning them.
      Tests: `finalize_refuses_when_orchestrator_is_running`,
      `finalize_force_overrides_running_check`,
      `finalize_success_path_with_idle_detector`,
      `finalize_noop_path_does_not_invoke_detector`,
      `finalize_removes_legacy_sidecars`.

## 7. Doctor drift check

- [x] 7.1 Wrote failing tests in `crates/interface-cli/src/cmd_doctor.rs`:
      `check_drift_skipped_when_no_legacy`, `check_drift_ok_when_counts_match`,
      and `check_drift_warns_when_counts_differ`. The check lives in a new
      `check_drift` function rather than overloading `check_database`, since
      the two have orthogonal failure modes.
- [x] 7.2 Implemented `check_drift` + `count_messages_readonly` in
      `cmd_doctor.rs`. Uses `SqliteConnectOptions::read_only(true)` plus
      `create_if_missing(false)` so opening `assistant.db.legacy` does not
      create `*-wal`/`*-shm` sidecars or recreate a missing file.
- [x] 7.3 The warning detail explicitly says
      `Run \`assistant migrate finalize\` to overwrite the runtime DB with
      legacy content`, asserted by `check_drift_warns_when_counts_differ`.
- [x] 7.4 All three drift tests pass; `cmd_doctor` is now async and the call
      site in `main.rs` awaits it.

## 8. Operator documentation

- [x] 8.1 Added `docs/operations/multi-org-cutover.md` covering: what
      changed, how to verify with `assistant doctor`, when to run
      `assistant migrate finalize`, the meaning of `assistant.db.legacy`,
      and the rollback procedure.
- [x] 8.2 Updated the runtime-data section in `CLAUDE.md` to mention that
      `assistant.db.legacy` is the post-cutover artifact and may be deleted
      after a successful run; cross-linked the new operator doc.
- [x] 8.3 Cross-linked the new operator doc from
      `docs/adr/adr-0007-multi-user-orgs.md` (the existing multi-org ADR)
      under a new `## Operations` section. No new ADR was created — the
      cutover is operational follow-through to ADR-0007, not a separate
      architectural decision.

## 9. Schorschvm dry-run + release

- [ ] 9.1 Take a fresh tarball backup of `~/.assistant/` on schorschvm before
      any binary update.
- [ ] 9.2 On a copy of the install (different host or container), run
      `assistant migrate finalize` with the new binary and verify
      `assistant doctor` reports OK.
- [ ] 9.3 Apply on schorschvm: stop services, run finalize, restart services,
      verify with doctor.
- [ ] 9.4 Update the project memory note about schorschvm to reflect the
      post-cutover state.

## 10. Cleanup

- [ ] 10.1 Remove the deprecated `config.storage.db_path` knob entirely
      after the next release if no test depends on it (per AGENTS.md
      "no lingering backwards compatibility code").
- [ ] 10.2 Drop the `runtime-multi-org` feature flag (if introduced during
      rollout) once the default has been on for one release cycle.
