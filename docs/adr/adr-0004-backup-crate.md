# ADR-0004: Backup and Restore Crate

**Status**: Accepted
**Date**: 2026-03-29
**Feature branch**: `004-backup-restore`

---

## Context

The assistant application stores user-critical data in `~/.assistant/`: agent personas, skill definitions, memory files, configuration, and a SQLite database. Users need a reliable way to create point-in-time backups and restore from them, with strong integrity guarantees and no dependency on the production installation being healthy.

The spec identified testability as the primary constraint: all core backup/restore logic must be exercisable via automated tests without a live installation or a real filesystem.

---

## Decision

### 1. New Crate: `assistant-backup`

A new `crates/backup/` crate isolates all backup/restore concerns from the CLI. This follows **Constitution Principle I (Crate-First Modularity)**: the crate has a single clear responsibility and no circular dependencies. The CLI crate (`assistant-cli`) depends on `assistant-backup` as a thin adapter.

Rationale:

- Keeps `BackupEngine` and `RestoreEngine` independently testable without pulling in clap or other CLI machinery.
- Allows future consumers (web-ui, A2A agents) to call backup/restore programmatically.

### 2. `BackupFs` Trait for Testability

A `BackupFs` trait abstracts all filesystem operations:

```rust
pub trait BackupFs: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;
    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    fn file_exists(&self, path: &Path) -> bool;
    fn file_size(&self, path: &Path) -> Result<u64>;
}
```

Two implementations:

- **`RealFs`**: wraps `std::fs`, used in production.
- **`FakeFs`**: in-memory `HashMap<PathBuf, Vec<u8>>` with `RwLock`, used in unit tests.

This follows **Constitution Principle II (Trait-Based DI)** and **Principle III (Test Discipline)**: 18 of 21 tests run with zero disk I/O.

### 3. Archive Format: `.tar.gz` with Manifest-First Ordering

Archives are POSIX tar streams compressed with gzip (deflate). The `manifest.json` entry is always written first so that integrity pre-checks and `backup list` can read metadata without decompressing the full archive.

`manifest.json` schema (v1):

```json
{
  "version": 1,
  "app_version": "0.1.64",
  "created_at": "2026-03-29T12:00:00Z",
  "install_dir": "/Users/alice/.assistant",
  "entries": [
    {
      "archive_path": "config.toml",
      "install_path": "/Users/alice/.assistant/config.toml",
      "size_bytes": 128,
      "sha256": "abc123..."
    }
  ]
}
```

Rationale for tar.gz over zip:

- Native Rust ecosystem support (`tar` + `flate2` crates, already in workspace).
- Streaming write without needing random access (important for large installations).
- Standard format trivially inspectable with `tar tf` / `tar xf`.

### 4. Atomic Write with `.tmp` Rename Pattern

Archives are written to `<output>.tar.gz.tmp`, then atomically renamed to the final path on success. On any failure (write error or rename error), the `.tmp` file is deleted. This prevents partial archives from being visible to `backup list` or used in a restore.

### 5. SQLite WAL Checkpoint Before Copy

Before reading `assistant.db`, the engine issues `PRAGMA wal_checkpoint(TRUNCATE)` via a read-only sqlx connection. This flushes pending WAL frames into the main database file and truncates the WAL, ensuring the copied `.db` is consistent. The checkpoint is best-effort; `.db-wal` and `.db-shm` sidecar files are included in the archive as insurance.

### 6. Path-Traversal Guard (Defence in Depth)

`extract_tar_gz` applies two independent guards:

1. **Entry-path guard**: rejects archive entries whose path string contains `..`.
2. **Dest-escape guard**: after computing `dest`, rejects any path whose `Path::components()` contain `ParentDir`, or that does not start with `install_dir`.

Guard 2 catches crafted manifests where `install_path` uses `..` to escape `install_dir` (e.g., `~/.assistant/../../etc/passwd`). This scenario is covered by `test_path_traversal_guard`.

### 7. SHA-256 Integrity Verification

Every extracted file is verified against its manifest SHA-256. Mismatch aborts the restore. The `sha2` and `hex` crates (already in workspace) are used.

---

## Consequences

- **Good**: Zero-disk-I/O unit tests via `FakeFs`; high confidence without a real installation.
- **Good**: Atomic archive writes prevent corrupt partial files.
- **Good**: Manifest-first design enables fast `backup list` (no full decompression).
- **Good**: Layered path-traversal guards protect against maliciously crafted archives.
- **Neutral**: Adds ~6 source files and ~1,700 LOC to the workspace.
- **Trade-off**: `BackupFs` wraps synchronous stdlib I/O; large files could block the async executor. Acceptable for MB-scale config files; a `tokio::fs` implementation can be added if needed.

---

## Alternatives Considered

| Alternative                      | Reason rejected                                                 |
| -------------------------------- | --------------------------------------------------------------- |
| Shell script wrapping `tar`      | Not cross-platform; untestable without a real installation      |
| Zip format                       | Requires random-access writes; less idiomatic in Rust ecosystem |
| Embed files in SQLite            | Couples backups to the database; bloats assistant.db            |
| Single module in `assistant-cli` | Violates crate-first modularity; harder to unit-test            |
