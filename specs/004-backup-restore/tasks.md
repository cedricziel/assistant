# Tasks: Backup and Restore Subcommands

**Input**: Design documents from `specs/004-backup-restore/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

**Tests**: Included — the spec explicitly requires a testable implementation (SC-004: all core logic covered by automated tests that run without a live installation).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)

## Path Conventions

New crate: `crates/backup/` (`assistant-backup`)
CLI adapter: `crates/interface-cli/src/`
Integration tests: `crates/backup/tests/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add new crate and workspace dependencies

- [x] T001 Add `tar = "0.4"`, `flate2 = "1"`, `sha2 = "0.10"`, `hex = "0.4"` to `[workspace.dependencies]` in `Cargo.toml` and add `"crates/backup"` to `[workspace.members]`
- [x] T002 Create `crates/backup/Cargo.toml` with package name `assistant-backup`, edition 2021, and inherit `serde`, `serde_json`, `tokio`, `anyhow`, `tracing`, `chrono`, `tar`, `flate2`, `sha2`, `hex` from workspace
- [x] T003 Create stub `crates/backup/src/lib.rs` exporting five modules: `manifest`, `archive`, `checksum`, `paths`, and `fs` (empty stubs that compile)
- [x] T004 [P] Create stub `crates/backup/src/manifest.rs` (empty module)
- [x] T005 [P] Create stub `crates/backup/src/archive.rs` (empty module)
- [x] T006 [P] Create stub `crates/backup/src/checksum.rs` (empty module)
- [x] T007 [P] Create stub `crates/backup/src/paths.rs` (empty module)
- [x] T008 Add `assistant-backup = { path = "../backup", version = "*" }` to `crates/interface-cli/Cargo.toml`
- [x] T009-setup Verify `cargo check -p assistant-backup` and `cargo check -p assistant-cli` both pass

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types and traits that every user story depends on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T009 Implement `BackupManifest` and `ManifestEntry` structs with `serde::{Serialize, Deserialize}` in `crates/backup/src/manifest.rs` (fields: version u32, app_version String, created_at String, install_dir String, entries Vec\<ManifestEntry\>; entry fields: archive_path, install_path, size_bytes u64, sha256 String)
- [x] T010 [P] Implement `sha256_hex(data: &[u8]) -> String` and `sha256_reader<R: Read>(r: R) -> Result<String>` in `crates/backup/src/checksum.rs` using the `sha2` and `hex` crates
- [x] T011 [P] Implement `InstallationPaths` struct and `discover(install_dir: &Path, db_path: Option<&Path>) -> Result<Vec<PathBuf>>` in `crates/backup/src/paths.rs` — walks `config.toml`, `assistant.db` (+wal/shm if present), `agents/`, `skills/`, `matrix-state/`, `signal-store/`
- [x] T012 Define `BackupFs` trait in `crates/backup/src/lib.rs` with methods: `read_file(&self, path: &Path) -> Result<Vec<u8>>`, `write_file(&self, path: &Path, data: &[u8]) -> Result<()>`, `list_dir(&self, path: &Path) -> Result<Vec<PathBuf>>`, `file_exists(&self, path: &Path) -> bool`, `file_size(&self, path: &Path) -> Result<u64>`
- [x] T013 Implement `RealFs` (production) and `FakeFs` (in-memory `HashMap<PathBuf, Vec<u8>>`) as concrete types implementing `BackupFs` in `crates/backup/src/lib.rs`
- [x] T014 Define `BackupOptions`, `RestoreOptions`, `BackupResult`, `RestoreResult` structs in `crates/backup/src/lib.rs` matching `data-model.md`
- [x] T015 Create `crates/interface-cli/src/cmd_backup.rs` with empty clap `BackupCommand` and `RestoreCommand` enum skeletons that compile
- [x] T016 Register `Backup` and `Restore` as variant stubs in the `Cli` enum in `crates/interface-cli/src/main.rs` (no-op match arms, just compiles)

**Checkpoint**: `cargo check --workspace` passes with no warnings

---

## Phase 3: User Story 1 — Back Up the Current Installation (Priority: P1) 🎯 MVP

**Goal**: `assistant backup [--output <path>]` creates a valid `.tar.gz` archive of the full installation and reports the output path, size, and file count.

**Independent Test**: `cargo test -p assistant-backup` passes; `assistant backup --output /tmp/test.tar.gz` creates a readable archive containing `manifest.json` and all installation files.

### Tests for User Story 1 ⚠️ Write first — verify they FAIL before implementing

- [x] T017 [P] [US1] Unit test `test_manifest_round_trip`: serialise a `BackupManifest` to JSON and deserialise back; assert all fields equal — in `crates/backup/src/manifest.rs` `#[cfg(test)] mod tests`
- [x] T018 [P] [US1] Unit test `test_sha256_empty` and `test_sha256_known_vector`: verify checksum output matches known SHA-256 values — in `crates/backup/src/checksum.rs` `#[cfg(test)] mod tests`
- [x] T019 [P] [US1] Unit test `test_backup_engine_creates_archive`: use `FakeFs` with three synthetic files; run `BackupEngine::run()`; assert archive bytes are non-empty, manifest is first entry, all three files present — in `crates/backup/src/lib.rs` `#[cfg(test)] mod tests`
- [x] T020 [P] [US1] Unit test `test_backup_engine_cleans_up_on_failure`: inject a `FakeFs` that fails mid-write; assert `BackupEngine::run()` returns `Err` and no partial archive file exists — in `crates/backup/src/lib.rs` `#[cfg(test)] mod tests`
- [x] T021 [US1] Integration test `test_backup_round_trip_tempdir`: create a populated `TempDir` mimicking `~/.assistant/`; run `BackupEngine::run()` with `RealFs`; assert archive exists, is readable with `tar`, and `manifest.json` deserialises correctly — in `crates/backup/tests/backup_integration.rs`

### Implementation for User Story 1

- [x] T022 [US1] Implement `write_tar_gz(manifest: &BackupManifest, files: &[(archive_path, data)], output: &Path) -> Result<u64>` in `crates/backup/src/archive.rs` — writes manifest first, then file entries; returns compressed byte count
- [x] T023 [US1] Implement WAL checkpoint helper `checkpoint_sqlite(db_path: &Path) -> Result<()>` in `crates/backup/src/paths.rs` — opens a read-only SQLite connection via `sqlx`, issues `PRAGMA wal_checkpoint(TRUNCATE)`, closes connection
- [x] T024 [US1] Implement `BackupEngine::new(fs: Arc<dyn BackupFs>) -> Self` and `BackupEngine::run(opts: BackupOptions) -> Result<BackupResult>` in `crates/backup/src/lib.rs` — orchestrates discovery → checkpoint → archive write → result
- [x] T025 [US1] Add cleanup-on-failure guard in `BackupEngine::run()`: write to a `.tmp` path, rename atomically on success, delete `.tmp` on any error — in `crates/backup/src/lib.rs`
- [x] T026 [US1] Implement default output path logic (`~/.assistant/backups/assistant-backup-<YYYYMMDD-HHMMSS>.tar.gz`) in `crates/backup/src/paths.rs`
- [x] T027 [US1] Fill out `BackupCommand` clap struct in `crates/interface-cli/src/cmd_backup.rs` with `--output` and `--install-dir` flags; implement `cmd_backup()` thin adapter that calls `BackupEngine::run()` and prints success/error summary to stdout
- [x] T028 [US1] Wire `Cli::Backup` match arm in `crates/interface-cli/src/main.rs` to call `cmd_backup()`; set process exit code on error

**Checkpoint**: `cargo test -p assistant-backup` passes; `cargo run -p assistant-cli -- backup --output /tmp/smoke.tar.gz` exits 0 and produces a readable archive

---

## Phase 4: User Story 2 — Restore from a Backup Archive (Priority: P2)

**Goal**: `assistant restore <path> [--force]` extracts a valid archive to `~/.assistant/`, verifying integrity and prompting for confirmation when the directory is non-empty.

**Independent Test**: `cargo test -p assistant-backup` still passes; `assistant restore /tmp/smoke.tar.gz --force` exits 0 and restores all files byte-identically; a corrupted archive exits 3.

### Tests for User Story 2 ⚠️ Write first — verify they FAIL before implementing

- [x] T029 [P] [US2] Unit test `test_restore_engine_restores_files`: build a valid in-memory tar.gz with `FakeFs`; run `RestoreEngine::run(force: true)`; assert all files written with correct content — in `crates/backup/src/lib.rs` `#[cfg(test)] mod tests`
- [x] T030 [P] [US2] Unit test `test_restore_aborts_on_corrupted_archive`: pass a truncated byte slice as archive; assert `RestoreEngine::run()` returns `Err` and no files are written — in `crates/backup/src/lib.rs` `#[cfg(test)] mod tests`
- [x] T031 [P] [US2] Unit test `test_path_traversal_guard`: create archive with entry `archive_path: "../../etc/passwd"`; assert extraction returns `Err` with path-traversal message — in `crates/backup/src/archive.rs` `#[cfg(test)] mod tests`
- [x] T032 [P] [US2] Unit test `test_checksum_mismatch_rejected`: create manifest with wrong SHA-256; assert `RestoreEngine::run()` returns `Err` with integrity message — in `crates/backup/src/lib.rs` `#[cfg(test)] mod tests`
- [x] T033 [US2] Integration test `test_restore_round_trip`: backup a populated `TempDir`, corrupt a byte in the archive copy, assert restore of corrupt copy fails; restore from original succeeds with byte-identical files — in `crates/backup/tests/restore_integration.rs`

### Implementation for User Story 2

- [x] T034 [US2] Implement `read_tar_gz_manifest(archive: &Path) -> Result<BackupManifest>` in `crates/backup/src/archive.rs` — reads only the first entry without extracting the rest; used for integrity pre-check and `backup list`
- [x] T035 [US2] Implement `extract_tar_gz(archive: &Path, install_dir: &Path, manifest: &BackupManifest) -> Result<Vec<String>>` in `crates/backup/src/archive.rs` — extracts entries, verifies per-file SHA-256 against manifest, enforces path-traversal guard; returns list of warnings
- [x] T036 [US2] Implement `RestoreEngine::new(fs: Arc<dyn BackupFs>) -> Self` and `RestoreEngine::run(opts: RestoreOptions) -> Result<RestoreResult>` in `crates/backup/src/lib.rs` — orchestrates: open archive → read manifest → integrity check → optional confirm → extract → report
- [x] T037 [US2] Implement interactive confirmation prompt in `crates/backup/src/lib.rs`: if `install_dir` is non-empty and `opts.force == false`, print warning with backup metadata and read `y/N` from stdin; return `Err` with exit code 5 if declined
- [x] T038 [US2] Fill out `RestoreCommand` clap struct in `crates/interface-cli/src/cmd_backup.rs` with positional `<PATH>`, `--force`, and `--install-dir` flags; implement `cmd_restore()` thin adapter calling `RestoreEngine::run()`; print warnings to stderr, result to stdout
- [x] T039 [US2] Wire `Cli::Restore` match arm in `crates/interface-cli/src/main.rs` to call `cmd_restore()`; map `RestoreEngine` error variants to documented exit codes (2 file-not-found, 3 corrupt, 4 version-mismatch, 5 declined)

**Checkpoint**: `cargo test -p assistant-backup` passes (all US1 + US2 tests); round-trip smoke test via `cargo run -p assistant-cli -- restore /tmp/smoke.tar.gz --force` exits 0

---

## Phase 5: User Story 3 — List and Inspect Available Backups (Priority: P3)

**Goal**: `assistant backup list` scans `~/.assistant/backups/` and prints each archive's timestamp, size, and file count without extracting.

**Independent Test**: Run `assistant backup list` after creating two archives; assert both appear with correct metadata.

### Tests for User Story 3 ⚠️ Write first — verify they FAIL before implementing

- [x] T040 [P] [US3] Unit test `test_list_backups_empty`: call `list_backups()` on an empty `FakeFs` directory; assert result is empty `Vec` — in `crates/backup/src/lib.rs` `#[cfg(test)] mod tests`
- [x] T041 [P] [US3] Unit test `test_list_backups_returns_metadata`: populate `FakeFs` with two valid archive bytes; call `list_backups()`; assert two entries with correct created_at and entry_count — in `crates/backup/src/lib.rs` `#[cfg(test)] mod tests`

### Implementation for User Story 3

- [x] T042 [US3] Implement `list_backups(backup_dir: &Path, fs: Arc<dyn BackupFs>) -> Result<Vec<BackupInfo>>` in `crates/backup/src/lib.rs` — scans directory for `*.tar.gz`, calls `read_tar_gz_manifest()` (T034) on each, collects `BackupInfo { path, archive_size, created_at, entry_count }`; skips unreadable files with a warning
- [x] T043 [US3] Add `BackupInfo` struct to `crates/backup/src/lib.rs` with fields: `path: PathBuf`, `archive_size: u64`, `created_at: String`, `entry_count: usize`
- [x] T044 [US3] Add `BackupList` subcommand variant to `BackupCommand` clap enum in `crates/interface-cli/src/cmd_backup.rs` with `--dir` flag; implement `cmd_backup_list()` adapter that calls `list_backups()` and formats the tabular output per contracts/cli-commands.md
- [x] T045 [US3] Wire `BackupCommand::List` match arm in `crates/interface-cli/src/cmd_backup.rs` and ensure the `Cli::Backup` dispatch in `crates/interface-cli/src/main.rs` routes to the list handler

**Checkpoint**: `cargo test -p assistant-backup` passes (all US1 + US2 + US3 tests); `assistant backup list` prints correct tabular output

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Observability, documentation, quality gates

- [x] T046 [P] Add `tracing::info!` spans to `BackupEngine::run()` and `RestoreEngine::run()` — log start, file count, archive path, and duration — in `crates/backup/src/lib.rs`
- [x] T047 [P] Write ADR `docs/adr/adr-0004-backup-crate.md` documenting: new `assistant-backup` crate, `BackupFs` trait rationale, archive format choice (tar.gz), SQLite WAL checkpoint strategy
- [x] T048 Run `make lint` (`cargo clippy --workspace -- -D warnings`) and fix any warnings introduced by the new crate and CLI changes
- [x] T049 Run `make format` (`cargo fmt --all`) and verify no diffs remain
- [x] T050 Run `cargo machete --with-metadata` and remove any unused dependencies
- [x] T051 Run the quickstart.md smoke test sequence: `cargo test -p assistant-backup`, `cargo run -p assistant-cli -- backup --output /tmp/final-smoke.tar.gz`, `cargo run -p assistant-cli -- backup list`, `cargo run -p assistant-cli -- restore /tmp/final-smoke.tar.gz --force`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 completion — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Phase 2 — no dependency on US2 or US3
- **User Story 2 (Phase 4)**: Depends on Phase 2 + T034 (manifest reader from US2 phase itself) — no dependency on US1 (independently testable)
- **User Story 3 (Phase 5)**: Depends on Phase 2 + T034 from Phase 4 — logically depends on T034 (manifest peek) being implemented
- **Polish (Phase 6)**: Depends on all desired user stories complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Phase 2. No dependency on US2 or US3.
- **User Story 2 (P2)**: Can start after Phase 2. Reuses `read_tar_gz_manifest()` from its own phase; no dependency on US1 being complete (can construct test archives directly).
- **User Story 3 (P3)**: Requires `read_tar_gz_manifest()` (T034) from Phase 4. Once T034 is done, US3 can proceed even if RestoreEngine is not finished.

### Within Each User Story

- Write tests first → verify they FAIL → implement → verify tests PASS
- Checksum helpers before archive write (T010 before T022)
- Archive write before engine (T022 before T024)
- Engine before CLI adapter (T024 before T027)
- CLI adapter before main.rs wiring (T027 before T028)

### Parallel Opportunities

- T004, T005, T006 (stub module files) — all parallel
- T010, T011 (checksum + path discovery) — parallel within Phase 2
- T017, T018, T019, T020 (US1 unit tests) — all parallel
- T029, T030, T031, T032 (US2 unit tests) — all parallel
- T040, T041 (US3 unit tests) — parallel
- T046, T047 (tracing + ADR) — parallel in Polish phase

---

## Parallel Example: User Story 1

```bash
# Launch all unit tests for US1 simultaneously (write first, watch fail):
Task T017: test_manifest_round_trip  (crates/backup/src/manifest.rs)
Task T018: test_sha256_known_vector  (crates/backup/src/checksum.rs)
Task T019: test_backup_engine_creates_archive  (crates/backup/src/lib.rs)
Task T020: test_backup_engine_cleans_up_on_failure  (crates/backup/src/lib.rs)

# Then implement in dependency order:
Task T022: write_tar_gz()          → unblocks T024
Task T023: checkpoint_sqlite()     → unblocks T024
Task T024: BackupEngine::run()     → unblocks T025-T028
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1 (backup create)
4. **STOP and VALIDATE**: `cargo test -p assistant-backup` passes; `assistant backup --output /tmp/test.tar.gz` works
5. Ship or demo the backup subcommand

### Incremental Delivery

1. Setup + Foundational → workspace compiles
2. User Story 1 → `assistant backup` works; independently testable ✅
3. User Story 2 → `assistant restore` works; independently testable ✅
4. User Story 3 → `assistant backup list` works; independently testable ✅
5. Polish → lint, format, ADR, tracing

### Parallel Team Strategy

With two developers after Phase 2 is complete:

- **Developer A**: User Story 1 (T017–T028)
- **Developer B**: User Story 2 (T029–T039) — can build `RestoreEngine` against the same archive format without waiting for `BackupEngine` by constructing test archives in tests

---

## Notes

- **Testability is the primary constraint** (spec: "paramount that we create a testable implementation", SC-004). `FakeFs` and in-memory archives enable zero-disk-I/O unit tests.
- `[P]` tasks operate on different files and have no in-flight dependencies.
- `[Story]` label maps each task to its user story for traceability.
- Each user story is independently completable and testable.
- Write tests first; verify they fail before implementing.
- Commit after each checkpoint (T008, T016, T028, T039, T045, T051).
- **Do not skip `make lint` / `make format`** — pre-commit hooks enforce `fmt`, `clippy -D warnings`, `machete`.
