# Data Model: Backup and Restore

**Feature**: 004-backup-restore
**Phase**: 1 — Design
**Date**: 2026-03-29

---

## Entities

### BackupManifest

Serialized as `manifest.json` and embedded as the first entry in the `.tar.gz` archive. Acts as the authoritative index of the backup's contents.

| Field         | Type                 | Description                                                          |
| ------------- | -------------------- | -------------------------------------------------------------------- |
| `version`     | `u32`                | Manifest schema version (currently `1`). Used for forward-compat.    |
| `app_version` | `String`             | Semver string of the `assistant-cli` binary that created the backup. |
| `created_at`  | `String` (RFC 3339)  | UTC timestamp when the backup was initiated.                         |
| `install_dir` | `String`             | Absolute path of the installation directory at backup time.          |
| `entries`     | `Vec<ManifestEntry>` | One entry per file included in the archive.                          |

**Validation rules**:

- `version` MUST be `1` (current schema); values > 1 trigger a version-mismatch warning on restore.
- `created_at` MUST parse as a valid RFC 3339 datetime.
- `entries` MUST be non-empty.

---

### ManifestEntry

One record per file in the backup archive.

| Field          | Type     | Description                                                            |
| -------------- | -------- | ---------------------------------------------------------------------- |
| `archive_path` | `String` | Path within the archive (relative, e.g., `agents/default/SOUL.md`).    |
| `install_path` | `String` | Original absolute path on disk (used during restore).                  |
| `size_bytes`   | `u64`    | Uncompressed file size in bytes.                                       |
| `sha256`       | `String` | Lowercase hex-encoded SHA-256 digest of the uncompressed file content. |

**Validation rules**:

- `archive_path` MUST NOT contain `..` components (path-traversal guard on restore).
- `sha256` MUST be exactly 64 lowercase hex characters.
- `size_bytes` MUST match the actual byte count of the extracted file (verified post-extract).

---

### BackupOptions

Passed to `BackupEngine::run()` by the CLI adapter. Not serialized.

| Field         | Type              | Description                                                         |
| ------------- | ----------------- | ------------------------------------------------------------------- |
| `install_dir` | `PathBuf`         | Root of the installation to back up (default: `~/.assistant/`).     |
| `output_path` | `PathBuf`         | Destination path for the `.tar.gz` archive.                         |
| `db_path`     | `Option<PathBuf>` | Override for the SQLite database path (read from config if `None`). |

---

### RestoreOptions

Passed to `RestoreEngine::run()` by the CLI adapter. Not serialized.

| Field          | Type      | Description                                               |
| -------------- | --------- | --------------------------------------------------------- |
| `archive_path` | `PathBuf` | Path to the `.tar.gz` archive to restore from.            |
| `install_dir`  | `PathBuf` | Target installation directory (default: `~/.assistant/`). |
| `force`        | `bool`    | Skip interactive confirmation prompt when `true`.         |

---

### BackupResult

Returned by `BackupEngine::run()`.

| Field          | Type             | Description                                     |
| -------------- | ---------------- | ----------------------------------------------- |
| `output_path`  | `PathBuf`        | Final path of the created archive.              |
| `archive_size` | `u64`            | Compressed size of the archive in bytes.        |
| `entry_count`  | `usize`          | Number of files included.                       |
| `manifest`     | `BackupManifest` | The manifest that was written into the archive. |

---

### RestoreResult

Returned by `RestoreEngine::run()`.

| Field            | Type             | Description                                                 |
| ---------------- | ---------------- | ----------------------------------------------------------- |
| `restored_count` | `usize`          | Number of files written to disk.                            |
| `warnings`       | `Vec<String>`    | Non-fatal issues (e.g., skipped files with path-traversal). |
| `manifest`       | `BackupManifest` | The manifest read from the restored archive.                |

---

## State Transitions

### Backup operation

```
[Idle]
  → validate output path writable
  → discover installation files
  → WAL checkpoint (SQLite)
  → write archive (manifest first, then files)
  → verify archive integrity (re-read manifest, spot-check checksums)
  → report BackupResult
[Complete | Failed → cleanup temp archive]
```

### Restore operation

```
[Idle]
  → open and validate archive (manifest present, schema version compatible)
  → verify checksums for all entries
  → if existing install dir non-empty and !force → prompt user
  → extract entries to install_dir (atomic: write to temp, rename)
  → report RestoreResult
[Complete | Failed → leave existing install intact, cleanup any partial writes]
```

---

## Archive Layout

```
backup-20260329-143022.tar.gz
├── manifest.json                  # BackupManifest (first entry, always)
├── config.toml                    # ~/.assistant/config.toml
├── assistant.db                   # SQLite database
├── assistant.db-wal               # WAL file (if present)
├── agents/
│   ├── default/
│   │   ├── SOUL.md
│   │   ├── IDENTITY.md
│   │   ├── USER.md
│   │   ├── TOOLS.md
│   │   ├── MEMORY.md
│   │   ├── AGENTS.md
│   │   ├── HEARTBEAT.md
│   │   ├── BOOT.md
│   │   └── memory/
│   │       └── 2026-03-29.md
│   └── work/
│       └── ...
├── skills/
│   └── (user-installed skills)
├── matrix-state/                  # Only if present
│   └── ...
└── signal-store/                  # Only if present
    └── ...
```

**Default archive filename**: `assistant-backup-<YYYYMMDD-HHMMSS>.tar.gz`
**Default output directory**: `~/.assistant/backups/` (created if absent)
