## Why

The Apr 2026 multi-org migration (`archived/2026-04-25-multi-user-orgs`) shipped the
filesystem layout and `org.db`, but the orchestrator and web-ui runtimes still open
the legacy `~/.assistant/assistant.db` for conversations, messages, traces, and
scheduled tasks. The migrated `space.db` is created at first boot and then never
read again — on production hosts (e.g. schorschvm) it has been frozen since
Apr 25 while `assistant.db` keeps growing (567+ message delta and counting).
Until the runtime is wired to `space.db`, multi-tenancy at the data layer is
fictional and the migration's "default org/space" does not isolate any data.

## What Changes

- **BREAKING (operational)**: Orchestrator (`assistant orchestrator run`) and
  web-ui (`assistant webui serve`) open the per-space database via
  `OrgPoolFactory::space_db_path(org, space)` instead of the hard-coded
  `assistant_dir.join("assistant.db")` in `interface-cli/src/main.rs:1432`
  and `crates/web-ui/src/main.rs:257`.
- Add the legacy-layout migration check to the orchestrator binary (currently
  only web-ui runs it), so orchestrator-only deployments migrate on first boot.
- Replace the one-shot `tokio::fs::copy(assistant.db → space.db)` in
  `migrate_database` with a true cutover: after copy, atomically rename
  `assistant.db` to `assistant.db.legacy` so any process still pointing at the
  old path fails loudly instead of silently diverging.
- Add a `assistant doctor` check that flags drift between `assistant.db` and
  `space.db` when both exist with non-trivial divergence (e.g. row-count
  delta on `messages`).
- For installs already in the schorschvm-style stuck state (legacy is live,
  `space.db` is a stale snapshot), provide a `assistant migrate finalize`
  CLI subcommand that stops services, re-copies the live `assistant.db` over
  the stale `space.db`, then renames the legacy file.

## Capabilities

### New Capabilities

- `runtime-space-storage`: Defines how orchestrator and web-ui resolve the
  active per-space database, the cutover semantics (legacy rename, single
  authoritative writer), and the operator-facing finalize/doctor flows for
  installs that already migrated under the half-wired implementation.

### Modified Capabilities

<!-- None — there is no existing storage or multi-org runtime spec to amend. -->

## Non-goals

- Multiple non-default orgs/spaces at runtime. The cutover targets the
  `default/default` pair only; arbitrary org/space resolution (per-request
  routing, per-user space membership) is out of scope.
- Schema changes to `space.db`. This change moves the runtime onto the
  existing schema; new tables or migrations are separate work.
- Backfilling conversations created under one stuck install into multiple
  spaces. Stuck installs land entirely in `default/default`.
- Removing `assistant.db` from existing installs without operator opt-in.
  The finalize subcommand is explicit, not automatic.

## Impact

- **Code**: `crates/interface-cli/src/main.rs` (orchestrator startup),
  `crates/web-ui/src/main.rs` (web-ui startup), `crates/storage/src/migration.rs`
  (cutover semantics), `crates/storage/src/pool_factory.rs` (orchestrator-side
  callers), `crates/interface-cli/src/cmd_doctor.rs` (drift check), new
  `crates/interface-cli/src/cmd_migrate.rs` for the finalize subcommand.
- **Operations**: Existing stuck installs need one-shot operator action
  (run `assistant migrate finalize`). Fresh installs and clean migrations
  pick up the new path automatically.
- **Tests**: `crates/integration-tests/tests/smoke.rs` should grow a case
  that asserts the orchestrator opens `space.db` and not `assistant.db`.

## User-facing documentation

**Yes.** Add a short operator-facing migration note to `docs/` covering
the finalize subcommand, the rename of `assistant.db → assistant.db.legacy`,
and how to verify the cutover with `assistant doctor`.
