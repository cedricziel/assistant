# Backup & Restore

The assistant ships two CLI subcommands for creating and restoring point-in-time snapshots of your installation (`~/.assistant/`).

## What gets backed up

| Path                             | Description                                                                                  |
| -------------------------------- | -------------------------------------------------------------------------------------------- |
| `server.toml`                    | Process-wide configuration                                                                   |
| `secrets.key`                    | Key that encrypts stored integration credentials                                             |
| `jwt_secret`                     | Signing key for issued tokens                                                                |
| `org.db`                         | Org database: users, spaces, memberships, API keys, stored credentials                       |
| `orgs/`                          | Every org's `org.toml` and `catalog/`, and every space's `space.db`, `agents/` and `skills/` |
| `agents/`, `skills/`             | Persona workspaces and user-defined skills at the install root                               |
| `matrix-state/`, `signal-store/` | Interface session state (if present)                                                         |

Not backed up: `backups/` (earlier archives), `cache/` (the integrations catalog cache, which the server downloads again), `*.legacy` snapshots and any other file at the install root.

`secrets.key` is part of the archive. Without it, the credentials stored in `org.db` cannot be decrypted. The archive file is created with mode `0600`; treat it as a secret and store it accordingly.

## Commands

### `assistant backup`

Create a backup of the current installation.

```text
assistant backup [OPTIONS]
assistant backup list [--dir <path>]
```

**Options**

| Flag                   | Default                                                        | Description                  |
| ---------------------- | -------------------------------------------------------------- | ---------------------------- |
| `--output <path>`      | `~/.assistant/backups/assistant-backup-YYYYMMDD-HHMMSS.tar.gz` | Destination archive path     |
| `--install-dir <path>` | `~/.assistant/`                                                | Installation root to back up |

**Example**

```sh
# Default: timestamped archive in ~/.assistant/backups/
assistant backup

# Custom output path
assistant backup --output /mnt/nas/my-backup.tar.gz
```

**Output**

```text
Backup created: /home/alice/.assistant/backups/assistant-backup-20260329-120000.tar.gz
  Size:    2.4 MB
  Files:   47
  Created: 2026-03-29T12:00:00+00:00
```

### `assistant backup list`

List available backups.

```sh
assistant backup list

# Scan a custom directory
assistant backup list --dir /mnt/nas/
```

**Output**

```text
Backups in /home/alice/.assistant/backups/

  assistant-backup-20260329-120000.tar.gz   2.4 MB   2026-03-29T12:00:00+00:00   47 files
  assistant-backup-20260328-090000.tar.gz   2.3 MB   2026-03-28T09:00:00+00:00   45 files

2 backup(s) found.
```

### `assistant restore`

Restore an installation from a backup archive.

```text
assistant restore <ARCHIVE> [OPTIONS]
```

**Arguments**

| Argument    | Description                          |
| ----------- | ------------------------------------ |
| `<ARCHIVE>` | Path to the `.tar.gz` backup archive |

**Options**

| Flag                   | Default         | Description                      |
| ---------------------- | --------------- | -------------------------------- |
| `--force`              | false           | Skip interactive confirmation    |
| `--install-dir <path>` | `~/.assistant/` | Target directory to restore into |

**Example**

```sh
# Interactive (prompts for confirmation if install dir is non-empty)
assistant restore ~/.assistant/backups/assistant-backup-20260329-120000.tar.gz

# Non-interactive (for scripts)
assistant restore backup.tar.gz --force

# Restore to a different location
assistant restore backup.tar.gz --install-dir /tmp/assistant-test --force
```

**Output**

```text
Restore complete: 47 files restored.
```

## Archive format

Archives are standard `.tar.gz` files and can be inspected with any tar-compatible tool:

```sh
# List contents
tar tzf assistant-backup-20260329-120000.tar.gz

# Extract manually
tar xzf assistant-backup-20260329-120000.tar.gz -C /tmp/inspect/
```

The first entry in every archive is `manifest.json`, which records the app version, creation timestamp, and a SHA-256 checksum for every file. The restore command verifies all checksums before writing anything to disk — if any entry fails, the entire restore is aborted.

## Safety guarantees

- **Integrity** — every file is verified against its SHA-256 checksum; a mismatch aborts the restore before any file is written.
- **Atomic restore** — all entries are verified in memory first; the live install tree is not touched until all checksums pass.
- **Path-traversal protection** — entries with `..` components or destinations outside the install directory are skipped with a warning.
- **Atomic write** — the archive is written to a `.tmp` file and renamed into place on success; the `.tmp` is deleted on any failure.
- **SQLite safety** — every `.db` file is archived as a `VACUUM INTO` snapshot taken in one read transaction, so a running server cannot produce a torn copy. The snapshot includes rows still in the `-wal` file. `-wal` and `-shm` files are not archived, and restore deletes any that sit next to a restored database.
- **Private keys** — restored `secrets.key` and `jwt_secret` are written with mode `0600`.

## Automating backups

Use cron or a systemd timer to schedule regular backups:

```sh
# crontab: daily backup at 02:00
0 2 * * * assistant backup >> ~/.assistant/backup.log 2>&1
```
