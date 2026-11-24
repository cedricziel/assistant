## Why

The web-ui control plane is multi-tenant, but its data plane is not. At
`1011f60a`, `crates/web-ui/src/lib.rs` opens one `StorageLayer` at
`orgs/default/spaces/default/space.db` (or the `--db-path` override) and shares
it with every handler. The conversations, turns, webhooks, workflows,
attachments, push, commands, audio and capabilities routes never read
`AuthContext`. Any valid token from any org, in any role, can read or delete
another tenant's conversations, rotate webhook secrets, and read attachments. `require_scope_layer` exists but is unused, and the
`conversations.user_id` column (migration 038) is never filtered on.

Only the org-DB routers (members, spaces, users, orgs, catalog, interfaces,
bindings, templates, api_keys) authorize.

## What Changes

- Every data-plane request resolves `(org, space)` from `AuthContext` plus an
  explicit space selector, and gets that space's `StorageLayer` from a pool cache
  keyed by `(org, space)` and built on `OrgPoolFactory`.
- Each route enforces a minimum space role: viewer reads, member writes, space
  admin (or org admin) performs destructive and secret operations.
- Conversations and their children are scoped to the calling user.
- **BREAKING**: the `--db-path` / `ASSISTANT_DB_PATH` override and the hard-wired
  `default/default` pool are removed. No compatibility path.
- **BREAKING**: the public workflow webhook ingress URL carries the org and space
  slugs, because it has no `AuthContext` to resolve them from.
- Background components (workflow runner, schedule and event adapters, title
  generation) become space-aware instead of bound to one pool.
- A cross-tenant isolation test harness fails today and is the acceptance gate.

## Capabilities

### New Capabilities

- `web-data-plane-tenancy`: space resolution, role enforcement and per-user
  scoping for every data-plane HTTP route.

### Modified Capabilities

- `runtime-space-storage`: the web-ui no longer opens a single startup
  `space.db`; it resolves one per request through the pool cache.

## Impact

- `crates/web-ui`: new `SpacePools` cache and `SpaceCtx` extractor; every data
  router swaps its `pool` field for the resolver; `lib.rs` startup drops the
  single pool.
- `crates/storage`: `OrgPoolFactory` gains a cached `space_storage(org, space)`.
- `openapi.json` and `app/packages/assistant_api` (generated): a `space_id`
  selector on data-plane operations and the new webhook ingress path.
- `app/`: the Flutter client sends the selected space on data-plane calls.
- Not touched: `crates/tool-executor`, `crates/auth/src/jwt.rs`, and the
  traces, logs and analytics APIs, which are being deleted by a separate change
  (the app will emit OTLP only).

## Non-goals

- Per-key restricted-scope narrowing for API keys (role gate only for now).
- Changing the org-DB routers or the OAuth flow.
- A new role model; the existing `org_admin`, `space_admin`, `member`, `viewer`
  roles are reused.

## Documentation

User-facing documentation is required: `docs/web-ui.md` and `docs/multi-user.md`
(space selector, roles per route, webhook ingress URL, `--db-path` removal), plus
an ADR under `docs/adr/`.

## UI impact

No visual change. The Flutter client gains a request header, so no screenshot
baselines move.
