# Multi-Org Runtime Cutover

This guide is for operators upgrading an existing single-user install
(`~/.assistant/assistant.db`) to a release that runs the orchestrator and
web-ui directly against the multi-org per-space database
(`~/.assistant/orgs/default/spaces/default/space.db`).

If you have a fresh install — i.e. you have never run a release older than
this one — there is nothing to do. The first startup creates the multi-org
layout directly.

## What changed

Earlier releases shipped a transitional state in which:

- the orchestrator and web-ui wrote to the legacy file
  `~/.assistant/assistant.db`, and
- a separate `space.db` was created by some code paths but not actually used
  for runtime traffic.

Starting with this release:

1. **Runtime data lives at `space.db`.** Both the orchestrator
   (`assistant orchestrator run`) and the web-ui (`assistant webui serve`)
   open the per-space database resolved by
   `OrgPoolFactory::space_db_path("default", "default")`.
2. **The first startup atomically cuts over** legacy installs. The
   migration helper renames `assistant.db` to `assistant.db.legacy` and
   removes the `*-shm` / `*-wal` sidecars, so the two databases cannot
   diverge after a successful migration.
3. **`config.storage.db_path` is deprecated.** It still works for
   tests/dev, but the orchestrator emits a `warn!` when it is set in
   production config and the knob will be removed in a future release.

## Verifying with `assistant doctor`

```sh
assistant doctor
```

Among other checks, doctor compares the `messages` row count between
`assistant.db.legacy` (if present) and the runtime database. The states
to watch for:

| Status                                          | Meaning                                                                                  |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `[+] Ok`                                        | Counts match — the cutover is complete and the legacy file is a frozen snapshot.         |
| `[+] Ok` _("no `assistant.db.legacy` present")_ | Fresh install — no migration ever happened.                                              |
| `[!] Warning` _("row count drift")_             | The legacy file has continued to grow after migration. Run `assistant migrate finalize`. |

Doctor opens both databases with `mode=ro` so it never recreates a
`*-wal`/`*-shm` against the legacy file.

## When to run `assistant migrate finalize`

Run finalize **only when doctor reports drift** — that is, when an old
binary on the host migrated the install but kept writing to
`assistant.db` afterwards. Symptoms:

- doctor's `Migration Drift` check is `[!] Warning`,
- `~/.assistant/assistant.db` exists _and_ `~/.assistant/assistant.db.legacy`
  also exists, _or_
- `~/.assistant/assistant.db` exists _and_ the multi-org tree under
  `orgs/default/spaces/default/` already has its own `space.db`.

The command:

```sh
# 1. Stop every assistant service first.
systemctl --user stop assistant-orchestrator assistant-web-ui

# 2. Take a backup (recommended, but not required — finalize itself is
#    only a copy + rename, but a tarball is your seatbelt).
assistant backup

# 3. Run finalize.
assistant migrate finalize
```

What finalize does, in order:

1. Refuses to run if any process whose argv contains
   `assistant orchestrator run` or `assistant webui` is alive. Override
   with `--force` only when you know there are no live writers — for
   example, after a forced kill where stale pids may remain.
2. Issues `PRAGMA wal_checkpoint(TRUNCATE)` on `assistant.db` so any WAL
   pages are flushed before the copy.
3. Copies `assistant.db` over the existing `space.db` (creating its
   parent directory if needed).
4. Renames `assistant.db` → `assistant.db.legacy`.
5. Removes `assistant.db-shm` and `assistant.db-wal`.
6. Prints the restart instructions.

Finalize is **idempotent**: re-running it after a successful cutover
prints `already finalized — nothing to do` and exits 0.

## Meaning of `assistant.db.legacy`

`assistant.db.legacy` is the post-cutover artifact. It is a frozen copy
of the legacy database at the moment of the rename. Once the runtime is
back up and writing to `space.db`, the legacy file is **read-only by
convention** — nothing in the assistant binary opens it for writing.

You can:

- **Keep it indefinitely** as an extra safety net.
- **Archive it** to off-host storage.
- **Delete it** once `assistant doctor` reports no drift and you are
  satisfied the new layout works (typically after a few days of normal
  use).

If you delete it, the `Migration Drift` check transitions from `[+] Ok`
to `[-] Skipped`, which is fine.

## Rollback

If something goes wrong after finalize and you want to revert to the
legacy layout:

1. Stop all assistant services.
2. Restore the pre-finalize tarball with `assistant restore <archive>`.
3. Pin the binary to the previous release (binary archive or git tag).
4. Restart services.

The `assistant.db.legacy` file alone is **not** a complete rollback —
the multi-org tree under `orgs/default/spaces/default/` may have
diverged from the legacy snapshot. Always rely on a tarball backup for
rollback, not on the legacy file.

## See also

- `docs/adr/adr-0007-multi-user-orgs.md` — the org/space identity model.
- `docs/backup.md` — `assistant backup` / `assistant restore` reference.
- `openspec/changes/multi-org-runtime-cutover/` — the change proposal,
  design, and task list that drove this work.
