## 1. Audit hardcoded `assistant.db` references

- [x] 1.1 `grep -rn '"assistant.db"' crates/` and inventory every literal — split
      into "runtime path" (must move to `OrgPoolFactory`), "legacy fallback"
      (kept under deprecation), and "log/error message" (cosmetic).
- [ ] 1.2 Update the inventory comment block at the top of
      `crates/storage/src/pool_factory.rs` listing every production caller
      after this change lands.

## 2. Wire orchestrator to `OrgPoolFactory`

- [ ] 2.1 Write failing integration test in
      `crates/integration-tests/tests/smoke.rs` that boots the orchestrator
      against a tempdir, sends one message, and asserts `space.db` exists with
      a non-empty `messages` table while `assistant.db` does not exist.
- [ ] 2.2 Replace the hard-coded `assistant_dir.join("assistant.db")` in
      `crates/interface-cli/src/main.rs:1432` with a call to
      `OrgPoolFactory::space_db_path("default", "default")`.
- [ ] 2.3 Preserve `config.storage.db_path` as a deprecated dev-only override:
      log a `warn!` when set, and bypass the factory only in that case.
- [ ] 2.4 Confirm the integration test from 2.1 now passes.

## 3. Add legacy-layout migration to orchestrator startup

- [ ] 3.1 Extract the `is_legacy_layout` → backup → migrate-fs → migrate-db →
      bootstrap-admin block from `crates/web-ui/src/main.rs:194-255` into a
      shared helper `assistant_storage::migration::ensure_migrated(base_path)`.
- [ ] 3.2 Write failing unit test in `crates/storage/src/migration.rs` that
      asserts `ensure_migrated` is idempotent on an already-migrated layout.
- [ ] 3.3 Call `ensure_migrated` from the orchestrator startup path in
      `crates/interface-cli/src/main.rs` before `StorageLayer::new`.
- [ ] 3.4 Update `crates/web-ui/src/main.rs` to call the shared helper instead
      of inlining the steps.

## 4. Wire web-ui to `OrgPoolFactory`

- [ ] 4.1 Write failing test asserting `assistant webui serve` opens `space.db`
      not `assistant.db` (mirrors task 2.1 against the web-ui binary entry).
- [ ] 4.2 Replace `StorageLayer::new(&db_path)` at
      `crates/web-ui/src/main.rs:257` with the factory call.
- [ ] 4.3 Confirm 4.1 passes and existing web-ui auth/oauth tests still pass.

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

- [ ] 6.1 Add `Migrate { command: MigrateCommand }` to the top-level CLI enum
      in `crates/interface-cli/src/main.rs`, with a `Finalize { force: bool }`
      variant.
- [ ] 6.2 Create `crates/interface-cli/src/cmd_migrate.rs` with
      `cmd_migrate_finalize(force: bool, base_path: &Path)`.
- [ ] 6.3 Write failing unit test for the running-process detection helper:
      given a list of `/proc/*/cmdline` style strings, returns `Err(...)`
      when any contains `assistant orchestrator run` or `assistant webui`.
- [ ] 6.4 Implement detection using `sysinfo` (already in workspace deps;
      check before adding).
- [ ] 6.5 Implement WAL checkpoint via
      `sqlx::query("PRAGMA wal_checkpoint(TRUNCATE);")`.
- [ ] 6.6 Implement copy + rename + sidecar removal, mirroring task 5.2.
- [ ] 6.7 Implement no-op branch when `assistant.db` is already absent.
- [ ] 6.8 Write integration test covering: success path, services-running
      refusal, `--force` override, no-op path.

## 7. Doctor drift check

- [ ] 7.1 Write failing test in `crates/interface-cli/src/cmd_doctor.rs`
      asserting `check_database` returns `Warn` when `assistant.db.legacy`
      and `space.db` exist with different `messages` row counts.
- [ ] 7.2 Implement the row-count comparison in `check_database`. Use a
      read-only SQLite connection (`mode=ro`) to avoid creating WAL files
      against the legacy database.
- [ ] 7.3 Update the doctor output to suggest `assistant migrate finalize`
      when the warn condition fires.
- [ ] 7.4 Confirm test passes.

## 8. Operator documentation

- [ ] 8.1 Add `docs/operations/multi-org-cutover.md` covering: what changed,
      how to verify with `assistant doctor`, when to run `assistant migrate
finalize`, the meaning of `assistant.db.legacy`, and a rollback
      procedure.
- [ ] 8.2 Update the runtime-data section in `CLAUDE.md` to mention that
      `assistant.db.legacy` is the post-cutover artifact and may be removed
      after a successful run.
- [ ] 8.3 Cross-link the new doc from `docs/adr/` if a new ADR captures the
      cutover decision (per the AGENTS.md ADR convention for architectural
      changes).

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
