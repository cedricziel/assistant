# Research: Backup and Restore Subcommands

**Feature**: 004-backup-restore
**Phase**: 0 — Research
**Date**: 2026-03-29

---

## Decision 1: Archive Format

**Decision**: `.tar.gz` (tar with gzip compression) via the `tar` + `flate2` crates.

**Rationale**: Standard Unix archive format; universally inspectable (`tar tf backup.tar.gz`); cross-platform via pure-Rust crates; deterministic entry ordering when sorted; `.tar.gz` extension communicates intent clearly. The `tar` crate provides a builder API well-suited to adding entries from arbitrary `Read` sources without materializing the full archive in memory.

**Alternatives considered**:

- `zip` — more familiar on Windows but less idiomatic for Rust CLI tools; `zip` crate is less ergonomic for streaming writes.
- Raw directory copy (no compression) — simple but large; not atomic and harder to transfer across machines.
- `zstd` — better compression ratio than gzip but adds a build dependency that is less universally available; gzip is sufficient for the expected archive sizes.

---

## Decision 2: Crate Placement

**Decision**: New crate `assistant-backup` at `crates/backup/`, depended on by `assistant-cli`.

**Rationale**: Constitution Principle I (Crate-First Modularity) requires every feature to live in its own independently-compilable, independently-testable crate. Placing backup/restore logic in the CLI crate would conflate argument parsing with business logic and prevent unit-testing the core engine without instantiating the full CLI. A separate crate keeps the CLI as a thin adapter.

**Alternatives considered**:

- Embedding in `assistant-cli` — violates Constitution I; also makes the logic untestable in isolation.
- Embedding in `assistant-storage` — storage crate is already responsible for DB access; adding archive I/O would violate single-responsibility.

---

## Decision 3: Backup Scope

**Decision**: Full-installation backup by default, covering:

1. `~/.assistant/config.toml`
2. `~/.assistant/assistant.db` (WAL-checkpointed before copy)
3. `~/.assistant/agents/` tree (all persona memory files and workspace files)
4. `~/.assistant/skills/` (user-installed/overridden skills only — built-in skills are re-seeded on startup so they are lower priority, but included for completeness)
5. `~/.assistant/matrix-state/` and `~/.assistant/signal-store/` if present

A `--persona <id>` filter (P3) is deferred to a future iteration.

**Rationale**: A full backup is the safest default; partial backups introduce restore complexity (which files to overwrite vs preserve). For typical installations (spec assumption: ≤50 MB DB, ≤100 personas) a full backup completes well within the 30 s success criterion.

**Alternatives considered**:

- Per-persona backup — reduces archive size but complicates the restore path; deferred to v2.
- Excluding `signal-store` and `matrix-state` — these contain auth state that is hard to regenerate; including them avoids the user having to re-link devices after a restore.

---

## Decision 4: Checksum Algorithm

**Decision**: SHA-256 for file-level checksums in the manifest, computed via the `sha2` crate.

**Rationale**: `sha2` is a pure-Rust crate, widely used in the Rust ecosystem, and produces a 256-bit digest that is collision-resistant for integrity verification purposes. SHA-256 output can be represented as a 64-char hex string without additional dependencies.

**Alternatives considered**:

- `blake3` — faster but adds a dependency not already in the workspace; overkill for once-per-session verification.
- `md5` / `sha1` — deprecated for integrity use.
- No checksum — violates FR-010.

---

## Decision 5: Manifest Format

**Decision**: `manifest.json` embedded as the first entry in the archive. Structure: a JSON object with top-level fields `version`, `created_at` (RFC 3339), `app_version`, `install_dir`, and an `entries` array (each entry: `archive_path` — path inside the archive, `install_path` — intended extraction path, `size_bytes`, `sha256`).

**Rationale**: `serde_json` is already a workspace dependency. A machine-readable manifest enables: (a) integrity verification without full extraction; (b) version-mismatch detection (FR-011); (c) `backup list` display of per-archive metadata without extracting archives. Placing the manifest first in the archive allows streaming verification.

**Alternatives considered**:

- TOML manifest — workspace already uses TOML for config, but `serde_json` is more common for structured, programmatically-consumed manifests with arrays.
- No manifest — blocks integrity verification and version detection.

---

## Decision 6: SQLite Safety During Backup

**Decision**: Issue `PRAGMA wal_checkpoint(TRUNCATE)` on the open connection before closing it, then copy the `.db` file. If the assistant is not running (normal backup scenario), open a read-only connection solely for the checkpoint, then close it before copying.

**Rationale**: SQLite WAL mode produces two additional files (`.db-wal`, `.db-shm`). A simple file copy during active writes could produce an inconsistent snapshot. Checkpointing first ensures all WAL data is folded back into the main database file. The backup crate will copy `.db` and, if present, `.db-wal` and `.db-shm` files to guarantee the archive is self-consistent.

**Alternatives considered**:

- SQLite Online Backup API (via `rusqlite`) — most robust but requires adding `rusqlite` as a dep alongside `sqlx`; complexity outweighs benefit for the expected usage pattern (backup runs when assistant is idle).
- Skip WAL files — risks data loss if WAL contains uncommitted transactions.

---

## Decision 7: Dependency Additions

New workspace-level dependencies to add in root `Cargo.toml`:

- `tar = "0.4"` — archive creation/extraction
- `flate2 = "1"` — gzip compression
- `sha2 = "0.10"` — SHA-256 checksums
- `hex = "0.4"` — hex encoding of digests (tiny utility crate)

All four are pure-Rust, actively maintained, and have no transitive system dependencies.

---

## Decision 8: Testability Strategy

**Decision**: Abstract the filesystem via a `BackupFs` trait with two implementations: `RealFs` (production) and `FakeFs` (in-memory, test-only). The `BackupEngine` and `RestoreEngine` structs accept `Arc<dyn BackupFs>`. Test fixtures can construct archives entirely in memory using `std::io::Cursor`.

**Rationale**: Constitution Principles II (trait-based DI) and III (test discipline) both require that tests do not rely on real filesystem I/O. A trait abstraction allows the backup logic to be unit-tested with predictable in-memory content without `tempfile` or disk I/O. Integration tests use `tempfile::TempDir` for realistic path resolution.

**Alternatives considered**:

- `tempfile`-only tests — still require disk I/O; slower in CI; can leave artifacts on crash.
- No trait abstraction — violates Constitution II; makes mocking impossible.
