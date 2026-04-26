## Context

The Apr 2026 multi-org change (`archived/2026-04-25-multi-user-orgs`) introduced
the directory layout and `org.db` (auth, users, memberships, OAuth clients).
Both the orchestrator binary (`crates/interface-cli/src/main.rs`) and the
web-ui binary (`crates/web-ui/src/main.rs`) still resolve their primary
`StorageLayer` to `~/.assistant/assistant.db`:

- `interface-cli/src/main.rs:1432` — hard-coded fallback `assistant_dir.join("assistant.db")`.
- `web-ui/src/main.rs:257` — same `StorageLayer::new(&db_path)` call after the
  legacy-layout migration block.
- `OrgPoolFactory::space_db_path()` (`crates/storage/src/pool_factory.rs:49`) is
  defined and unit-tested but has **no production caller** — `grep` only finds
  it referenced from `migration.rs` itself.

`migrate_database` copies `assistant.db` → `space.db` once and leaves
`assistant.db` in place; the runtime keeps writing to the legacy file. On
schorschvm the divergence is now 567+ messages and growing (Apr 26: live
`assistant.db` 22,145 messages vs frozen `space.db` 21,578).

A second issue: only the web-ui binary calls `is_legacy_layout` →
`migrate_database`. An orchestrator-only deployment never auto-migrates,
so the half-wired state is hidden until web-ui happens to run.

## Goals / Non-Goals

**Goals:**

- Orchestrator and web-ui open the per-space DB via `OrgPoolFactory`, never
  via a raw `assistant.db` path.
- Migration becomes a true cutover: the legacy file is renamed so a stale
  process cannot silently keep writing to it.
- Operators with a stuck install (legacy live, `space.db` snapshot stale)
  have an explicit one-shot recovery path.
- `assistant doctor` surfaces drift rather than letting it accumulate.

**Non-Goals:**

- Multi-org/multi-space routing at request time (per-user space resolution,
  org switcher). Cutover targets `default/default` only.
- Schema changes inside `space.db`.
- Re-sharding or splitting an existing single-space install across multiple
  spaces.

## Decisions

### 1. Single resolver: `OrgPoolFactory::space_db_path("default", "default")`

Both binaries call the factory rather than constructing the path themselves.
The legacy `db_path` config knob (`config.storage.db_path`) is preserved as
a deprecated override for tests and dev — when set, it skips the factory
and prints a deprecation warning.

**Alternatives considered:**

- _Keep `assistant.db` as the runtime path and treat `space.db` as a future
  copy._ Rejected: the migration's explicit purpose is to land on the
  multi-org layout. Indefinite legacy preservation invalidates the migration
  contract and blocks any future per-space work.
- _Add a config flag to opt into the new path._ Rejected: the migration
  already happened automatically; making the cutover opt-in produces three
  states (legacy, migrated-but-not-cut-over, cut-over) instead of two.

### 2. Atomic cutover: rename `assistant.db` → `assistant.db.legacy`

After `tokio::fs::copy(assistant.db → space.db)` succeeds, `migrate_database`
issues `tokio::fs::rename(assistant.db, assistant.db.legacy)` and removes
the legacy `*-shm` / `*-wal` sidecars. A process still holding the old fd
keeps reading/writing through the deleted inode (Linux semantics) but its
writes are invisible to anything that re-resolves the path — making the
divergence loud (process crash on next open) instead of silent.

**Alternatives considered:**

- _Symlink `assistant.db` → `space.db`._ Rejected: SQLite locking against a
  symlinked WAL is fragile across platforms; a leftover symlink masks the
  cutover state in `ls`.
- _Hard-link._ Same locking concerns as symlink, plus `tokio::fs::copy`
  semantics already preserve content but not inode identity.

### 3. Add legacy-layout check to orchestrator startup

The orchestrator binary gains the same `is_legacy_layout` → `migrate_database`
sequence currently in `crates/web-ui/src/main.rs:194-255`. Both call sites
share a helper in `assistant_storage::migration` so the bootstrap-admin step
runs in exactly one place.

**Alternatives considered:**

- _Run migration from a third place (e.g. a `pre-start` hook in systemd)._
  Rejected: increases packaging surface and would not help dev-mode
  (`make run`) deployments.

### 4. `assistant migrate finalize` for stuck installs

For installs already in the half-wired state, a one-shot CLI does:

1. Stop services (operator pre-condition; CLI checks for running pids and
   refuses unless `--force`).
2. WAL-checkpoint the live `assistant.db`.
3. `tokio::fs::copy` it over the stale `space.db`.
4. Rename `assistant.db` → `assistant.db.legacy`, drop sidecars.
5. Print restart instructions.

**Alternatives considered:**

- _Auto-finalize on next boot if drift is detected._ Rejected: the cutover
  closes the legacy file; doing it under live services risks losing in-flight
  writes. Operator gating is safer.

### 5. Doctor drift check

`cmd_doctor::check_database` is split into a per-space iteration. For each
space, compare `messages` row count between `space.db` and (if present)
the sibling `assistant.db.legacy`; report `Warn` when delta > 0. The check
is read-only and never modifies either file.

## Risks / Trade-offs

- **Risk:** A non-default org/space materializes on disk before cutover lands
  in this change. → **Mitigation:** scope the cutover to `default/default`
  explicitly and assert in `migrate_database` that the seeded slugs match
  the constants. Future per-space resolution lands in a separate change.

- **Risk:** Renaming `assistant.db` while a sibling crate (e.g. `assistant-backup`)
  hardcodes the path breaks backup/restore. → **Mitigation:** grep the
  workspace for `"assistant.db"` literals before merging; route every reader
  through `OrgPoolFactory` or a documented legacy-fallback helper.

- **Risk:** `assistant migrate finalize` runs against services that are still
  alive, corrupting WAL. → **Mitigation:** the subcommand reads
  `/proc/*/cmdline` (or the platform-equivalent `sysinfo` lookup) to detect
  running orchestrator/web-ui processes and refuses unless `--force`.

- **Trade-off:** Renaming the legacy file means the operator's familiar
  `~/.assistant/assistant.db` disappears. → Documented in the operator
  migration note; doctor explicitly mentions the renamed file.

## Migration Plan

1. Land code change with new path resolution gated on a build-time feature
   flag `runtime-multi-org` so CI can run both modes during the rollout.
2. Flip the flag default to on; run integration smoke + manual schorschvm
   dry-run on a copy of the production install (snapshot the host first).
3. Release. On schorschvm: snapshot, then `assistant migrate finalize`,
   then restart services, then run `assistant doctor`.
4. After 1 week of stable operation, drop the feature flag and the legacy
   `db_path` config override.

**Rollback:** restore `assistant.db.legacy → assistant.db`, downgrade the
binary, restart services. The space.db copy is left untouched and can be
deleted by the operator.

## Open Questions

- Does `assistant-backup::backup_legacy_install` need a sibling
  `backup_multi_org_install` for the post-cutover layout, or is the existing
  per-file enumeration sufficient? (Likely already covered, but verify
  before flipping the default.)
- Should the deprecation warning for `config.storage.db_path` be a hard
  error in the next release, or stay as a warning indefinitely for tests?
