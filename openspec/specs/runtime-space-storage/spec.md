## ADDED Requirements

### Requirement: Orchestrator opens per-space database

The orchestrator binary (`assistant orchestrator run`) SHALL resolve its
primary `StorageLayer` path via `OrgPoolFactory::space_db_path("default",
"default")` and MUST NOT fall back to `~/.assistant/assistant.db` for
runtime data.

#### Scenario: Fresh install starts orchestrator

- **WHEN** an operator runs `assistant orchestrator run` on a host with no
  prior `~/.assistant/` directory
- **THEN** the orchestrator creates
  `~/.assistant/orgs/default/spaces/default/space.db` and opens it as the
  primary `StorageLayer`
- **AND** no file named `~/.assistant/assistant.db` is created at any point

#### Scenario: Migrated install starts orchestrator

- **WHEN** an operator runs `assistant orchestrator run` on a host where
  `migrate_database` has already produced `space.db` and renamed
  `assistant.db` to `assistant.db.legacy`
- **THEN** the orchestrator opens
  `~/.assistant/orgs/default/spaces/default/space.db` as the primary
  `StorageLayer`
- **AND** the orchestrator does not open or write to `assistant.db.legacy`

### Requirement: Web-ui opens per-space database

The web-ui binary (`assistant webui serve`) SHALL resolve its primary
`StorageLayer` path via the same `OrgPoolFactory::space_db_path` call as
the orchestrator and MUST NOT use a hard-coded `assistant.db` path for
runtime data (conversations, messages, traces, scheduler).

#### Scenario: Web-ui after fresh install

- **WHEN** `assistant webui serve` starts on a host with no prior install
- **THEN** the web-ui binary creates and opens
  `~/.assistant/orgs/default/spaces/default/space.db` as its primary
  `StorageLayer`

#### Scenario: Web-ui after migration

- **WHEN** `assistant webui serve` starts on a host that has been migrated
  via `migrate_database`
- **THEN** the web-ui opens `space.db` and ignores any sibling
  `assistant.db.legacy`

### Requirement: Orchestrator runs legacy-layout migration

The orchestrator binary SHALL invoke `assistant_storage::migration::is_legacy_layout`
on startup, and when it returns `true` MUST run the same backup →
filesystem migration → database migration → admin-bootstrap sequence
currently performed by the web-ui binary, before opening any
`StorageLayer`.

#### Scenario: Orchestrator-only host with legacy layout

- **WHEN** `assistant orchestrator run` starts on a host whose
  `~/.assistant/` directory contains `assistant.db` and no `orgs/`
  directory
- **THEN** the orchestrator runs the legacy-to-multi-org migration
- **AND** writes the pre-migration backup to `~/.assistant/backups/`
- **AND** writes initial admin credentials to
  `~/.assistant/admin-credentials.txt`
- **AND** then opens `space.db` (not `assistant.db`) for runtime data

#### Scenario: Orchestrator on already-migrated host

- **WHEN** `assistant orchestrator run` starts on a host where `orgs/`
  already exists
- **THEN** the orchestrator does not re-run the migration
- **AND** opens `space.db` directly

### Requirement: Migration cuts over atomically

The `migrate_database` function SHALL, after copying `assistant.db` to
`space.db`, rename `assistant.db` to `assistant.db.legacy` and remove the
legacy `assistant.db-shm` and `assistant.db-wal` sidecar files. Any
process still holding the old file descriptor MUST find no path-resolvable
`assistant.db` after the rename completes.

#### Scenario: Successful migration on idle host

- **WHEN** `migrate_database` runs against a host with `assistant.db`,
  `assistant.db-shm`, and `assistant.db-wal` present and no live writers
- **THEN** after the function returns, `space.db` exists with the same
  byte content as the pre-migration `assistant.db`
- **AND** `assistant.db.legacy` exists at `~/.assistant/`
- **AND** `assistant.db`, `assistant.db-shm`, and `assistant.db-wal` no
  longer exist
- **AND** opening the legacy path with `StorageLayer::new` returns a
  not-found error

#### Scenario: Migration fails before rename

- **WHEN** `migrate_database` fails to copy `assistant.db` to `space.db`
- **THEN** `assistant.db` MUST remain in place at its original path
- **AND** no `assistant.db.legacy` file is created

### Requirement: Doctor reports drift between legacy and migrated DBs

`assistant doctor` SHALL, when both `assistant.db` (or
`assistant.db.legacy`) and `space.db` exist on the host, compare the
`messages` row count of each and report a `Warn` result when the counts
differ.

#### Scenario: Stuck install with drift

- **WHEN** `assistant doctor` runs on a host where `assistant.db` has 100
  more rows in `messages` than `space.db`
- **THEN** the doctor output includes a `Warn` entry naming both files
  and the row delta
- **AND** suggests `assistant migrate finalize` as the remediation

#### Scenario: Clean post-cutover install

- **WHEN** `assistant doctor` runs on a host where only `space.db` and
  optionally `assistant.db.legacy` exist
- **AND** the row counts match (or `assistant.db.legacy` is absent)
- **THEN** the doctor reports `OK` for the database check

### Requirement: `assistant migrate finalize` recovers stuck installs

The CLI SHALL expose a `assistant migrate finalize` subcommand that
re-runs the cutover for installs already in the half-wired state (live
`assistant.db`, stale `space.db`).

The subcommand SHALL:

- Refuse to run when an orchestrator or web-ui process owned by the same
  user is detected, unless `--force` is passed.
- WAL-checkpoint the live `assistant.db` before copying.
- Overwrite `space.db` with the checkpointed `assistant.db` content.
- Rename `assistant.db` to `assistant.db.legacy` and remove the legacy
  `*-shm` / `*-wal` sidecars.
- Print restart instructions on success.

#### Scenario: Finalize succeeds on idle host

- **WHEN** an operator runs `assistant migrate finalize` after stopping
  all `assistant` services
- **THEN** the subcommand exits with status 0
- **AND** `space.db` byte-matches the pre-finalize `assistant.db` content
- **AND** `assistant.db.legacy` exists where `assistant.db` used to be
- **AND** the output instructs the operator to restart services

#### Scenario: Finalize refuses when services are running

- **WHEN** an operator runs `assistant migrate finalize` while
  `assistant orchestrator run` is still active
- **THEN** the subcommand exits with non-zero status
- **AND** prints a message naming the running pid and command line
- **AND** does not modify any database file

#### Scenario: Finalize on already-cut-over host is a no-op

- **WHEN** `assistant migrate finalize` runs on a host where `assistant.db`
  no longer exists (only `assistant.db.legacy` and `space.db`)
- **THEN** the subcommand exits with status 0
- **AND** prints "already finalized — nothing to do"
- **AND** does not modify `space.db` or `assistant.db.legacy`
