# CLI Command Contracts: Backup and Restore

**Feature**: 004-backup-restore
**Phase**: 1 — Design
**Date**: 2026-03-29

---

## `assistant backup`

Creates a `.tar.gz` archive of the full installation.

### Synopsis

```text
assistant backup [OPTIONS]
assistant backup list
```text

### Subcommands

| Subcommand  | Description                                    |
| ----------- | ---------------------------------------------- |
| _(default)_ | Create a new backup archive                    |
| `list`      | List backup archives in the default backup dir |

### Options (create mode)

| Option                 | Short | Type    | Default                                                    | Description                               |
| ---------------------- | ----- | ------- | ---------------------------------------------------------- | ----------------------------------------- |
| `--output <PATH>`      | `-o`  | PathBuf | `~/.assistant/backups/assistant-backup-<TIMESTAMP>.tar.gz` | Output path for the archive               |
| `--install-dir <PATH>` |       | PathBuf | `~/.assistant/`                                            | Override the installation root to back up |

### Exit codes

| Code | Meaning                                       |
| ---- | --------------------------------------------- |
| 0    | Backup created successfully                   |
| 1    | Generic failure (see stderr for details)      |
| 2    | Output path not writable / parent dir missing |
| 3    | Insufficient disk space                       |

### Stdout (success)

```text
Backup created: /home/user/.assistant/backups/assistant-backup-20260329-143022.tar.gz
  Size:    42.3 MB
  Files:   183
  Created: 2026-03-29T14:30:22Z
```text

### Stderr (failure)

Human-readable error message with suggested remediation. One line per error. No JSON.

---

## `assistant restore <PATH>`

Restores an installation from a `.tar.gz` backup archive.

### Synopsis

```text
assistant restore <PATH> [OPTIONS]
```text

### Arguments

| Argument | Required | Description                          |
| -------- | -------- | ------------------------------------ |
| `<PATH>` | Yes      | Path to the `.tar.gz` backup archive |

### Options

| Option                 | Short | Type    | Default         | Description                                     |
| ---------------------- | ----- | ------- | --------------- | ----------------------------------------------- |
| `--force`              | `-f`  | flag    | false           | Skip interactive confirmation; for scripted use |
| `--install-dir <PATH>` |       | PathBuf | `~/.assistant/` | Override the restore target directory           |

### Exit codes

| Code | Meaning                                             |
| ---- | --------------------------------------------------- |
| 0    | Restore completed successfully                      |
| 1    | Generic failure (see stderr)                        |
| 2    | Archive file not found or not readable              |
| 3    | Archive is corrupted / checksum mismatch            |
| 4    | Version mismatch (archive from a newer app version) |
| 5    | User declined confirmation prompt                   |

### Interactive confirmation (when `--force` not set and install dir non-empty)

```text
WARNING: This will overwrite your existing installation at /home/user/.assistant/
  Backup created: 2026-03-29T14:30:22Z
  Files:          183
  App version:    0.1.63

Proceed? [y/N]:
```text

### Stdout (success)

```text
Restore complete: 183 files restored to /home/user/.assistant/
```text

### Stderr (warnings)

Non-fatal issues emitted to stderr, one per line:

```text
warning: skipped entry with unsafe path: ../../../etc/passwd
```text

---

## `assistant backup list`

Lists backup archives found in the default backup directory.

### Synopsis

```text
assistant backup list [OPTIONS]
```text

### Options

| Option         | Type    | Default                 | Description                    |
| -------------- | ------- | ----------------------- | ------------------------------ |
| `--dir <PATH>` | PathBuf | `~/.assistant/backups/` | Directory to scan for archives |

### Stdout (archives present)

```text
Backups in /home/user/.assistant/backups/

  assistant-backup-20260329-143022.tar.gz   42.3 MB   2026-03-29T14:30:22Z   183 files
  assistant-backup-20260328-090011.tar.gz   41.9 MB   2026-03-28T09:00:11Z   181 files

2 backup(s) found.
```text

### Stdout (no archives)

```text
No backups found in /home/user/.assistant/backups/
Run 'assistant backup' to create one.
```text

### Exit codes

| Code | Meaning              |
| ---- | -------------------- |
| 0    | Always (list output) |

---

## Manifest JSON Schema

Embedded as `manifest.json` (first entry) in every backup archive.

```json
{
  "version": 1,
  "app_version": "0.1.63",
  "created_at": "2026-03-29T14:30:22Z",
  "install_dir": "/home/user/.assistant",
  "entries": [
    {
      "archive_path": "config.toml",
      "install_path": "/home/user/.assistant/config.toml",
      "size_bytes": 1024,
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    },
    {
      "archive_path": "assistant.db",
      "install_path": "/home/user/.assistant/assistant.db",
      "size_bytes": 52428800,
      "sha256": "..."
    }
  ]
}
```text

**Schema constraints**:

- `version`: integer, currently `1`
- `app_version`: semver string (`MAJOR.MINOR.PATCH`)
- `created_at`: RFC 3339 UTC datetime
- `entries[].archive_path`: relative path, no `..` components
- `entries[].sha256`: 64-character lowercase hex string
