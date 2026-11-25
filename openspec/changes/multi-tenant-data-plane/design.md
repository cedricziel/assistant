## Context

`run_with_args` in `crates/web-ui/src/lib.rs` builds one `StorageLayer`, one
`SkillRegistry`, one `Orchestrator` and one set of local telemetry backends,
then hands `storage.pool` to every router state. The `AuthContext` that
`require_auth` injects is read only by the org-DB routers. The org database
(`org.db`) already models orgs, spaces and memberships; `OrgPoolFactory` already
maps `(org slug, space slug)` to `space.db`. The data plane never uses either.

Two identity facts shape the design:

- `AuthContext.org_id` / `space_roles` keys are **ids** (`org_<uuid>`,
  `spc_<uuid>`) for JWT and API-key contexts, but the legacy
  `ASSISTANT_WEB_TOKEN` context hard-codes `"default"` for both. Space
  directories on disk are **slugs**. Resolution therefore needs the `org.db`
  lookup `id → slug`, and the legacy context must be built from the real seeded
  org and space rows at startup.
- `AuthContext.scopes` is empty for session JWTs and non-empty for OAuth/API-key
  tokens, so `AuthContext::can` cannot be the gate: it would deny every session
  member. The gate is the role.

## Goals / Non-Goals

**Goals:** no data-plane request can reach a space the caller has no role in;
role checks are uniform and testable in one place; the old single pool and its
CLI override disappear.

**Non-Goals:** API-key scope narrowing, a new role model, changing the org-DB
routers, per-space LLM configuration.

## Decisions

### 1. Space selector

A data-plane request names its space with the `X-Space-Id` header, or the
`space_id` query parameter. The query form exists only because the browser
`EventSource` (conversation list stream, run event stream) cannot set headers.
Both carry the space **id** returned by `GET /api/orgs/{org_id}/spaces`, the same
value the Flutter `SpaceSelection.spaceId` already holds.

When neither is present: if the caller has exactly one space role, that space is
used; otherwise the request fails with `400 {"error":"space selector required"}`.

### 2. `SpaceCtx` extractor

An axum `FromRequestParts` extractor, `SpaceCtx`, reads `AuthContext` and the
selector and returns:

```rust
pub struct SpaceCtx {
    pub auth: AuthContext,
    pub org_slug: String,
    pub space_slug: String,
    pub role: Role,                    // effective role in this space
    pub storage: Arc<StorageLayer>,    // from SpacePools
}
```

Resolution order, each step failing closed:

1. `space = SpaceStore::get_space(space_id)`; unknown id → `403`.
2. `space.org_id == auth.org_id`; mismatch → `403`.
3. `auth.role_in(space_id)` is `Some`, or the caller is an org admin of that org;
   otherwise `403`.
4. `SpacePools::get(org_slug, space_slug)` opens or reuses the pool.

Unknown space, foreign-org space and not-a-member all return the same `403
{"error":"access denied"}`, so the response cannot be used to enumerate spaces.

Role gating is a method, not a layer: `ctx.require(Access::Write)?` where
`Access` is `Read | Write | Admin` and maps `Viewer → Read`, `Member → Write`,
`SpaceAdmin` and `OrgAdmin → Admin`, each level including the ones below it.
`require_scope_layer` is not used: it binds a space at router-construction time,
but the space is a per-request value here. The unused layer is deleted.

### 3. `SpacePools`

`OrgPoolFactory` gains `space_storage(org_slug, space_slug) -> Arc<StorageLayer>`
backed by a `tokio::sync::RwLock<HashMap<(String, String), Arc<StorageLayer>>>`
with single-flight opening (a per-key `OnceCell`), so concurrent first requests
open one pool. Slugs are validated against `[a-z0-9][a-z0-9_-]*` before they are
joined into a path, so a hostile slug cannot traverse out of `orgs/`. The factory
creates the space directory and runs migrations on first open, exactly as
`StorageLayer::new` does today. Deleting a space evicts its entry.

Tests construct `OrgPoolFactory::new(tempdir)` and seed `org.db` with
`OrgStorageLayer::new_in_memory()`; `StorageLayer::new_in_memory()` remains the
seam for module-level unit tests that do not exercise tenancy.

### 4. Role per route

| Routes | Level | Per-user scope |
|---|---|---|
| `GET /conversations`, `/conversations/stream`, `/conversations/{id}` | Read | owner only |
| `POST /conversations`, `PATCH /conversations/{id}`, messages, voice, quick-message, attachments upload, turn cancel, `/command` | Write | owner only |
| `DELETE /conversations/{id}` | Write | owner only |
| `GET /conversations/{id}/runs/.../events/stream`, `/turns/.../status`, `/events`, `/attachments/{id}`, `/messages/{id}/audio`, `/audio/{id}` | Read | owner only |
| `GET /commands`, `GET /capabilities`, `GET /push/vapid-public-key` | Read | none |
| `POST/DELETE /push/subscribe` | Read | space-scoped store |
| `GET /webhooks`, `GET /webhooks/{id}` | Read; `secret` returned only to Admin | none |
| `POST /webhooks`, `PATCH`, `toggle`, `verify` | Write | none |
| `DELETE /webhooks/{id}`, `rotate-secret` | Admin | none |
| `GET /workflows`, `/workflows/{id}`, `/runs`, `/runs/{run_id}` | Read | none |
| `POST /workflows`, `PUT`, `activate`, `deactivate`, `test-run` | Write | none |
| `DELETE /workflows/{id}`, `GET /workflows/{id}/webhook-secrets` | Admin | none |
`DELETE /conversations/{id}` is Write rather than Admin because a conversation is
the owner's own data and the existing role matrix already lets members delete
conversations. The destructive operations that are shared space state (webhooks,
workflows) are Admin.

### 5. Per-user scoping

The conversation store already filters by `user_id` when built with
`with_user_id`. Handlers build it from `auth.user_id` for every query, so
`list` returns only the caller's conversations and `get`, `patch`, `delete`
return `404` for another user's id (not `403`, to avoid confirming existence).
Children of a conversation (messages, attachments, audio, turns, command
events, run streams) authorize by first loading the parent conversation through
the same owner-filtered store. Conversations created without a user (system and
scheduler turns) have a `NULL` `user_id` and are not visible through the API.

### 6. Public workflow webhook ingress

`POST /workflow-hooks/{id}/{token}` has no `AuthContext`. The route becomes
`POST /workflow-hooks/{org_slug}/{space_slug}/{id}/{token}`; the handler resolves
the pool from the path slugs through `SpacePools` and then validates the token as
it does today. The URL returned by `GET /workflows/{id}/webhook-secrets` is
updated. Existing hook URLs stop working (no compatibility route); operators
re-copy the URL.

### 7. Background components

The workflow runner, schedule adapter, event adapter and title generator each
consume one pool today. A `SpaceSupervisor` in the web-ui process lists every
space in `org.db` at startup and on a 30 s reconcile tick, and runs one set of
adapters per `(org, space)` against that space's pool; it stops them when the
space is deleted. `OrchestratorTurnClient` takes the `(org, space)` of the
workflow run, so a scheduled turn is dispatched on that space's orchestrator
and `AuthContext` (`system()` is re-scoped to the run's org and space).

Per-space orchestrators are built by a `SpaceRuntime` factory
(`StorageLayer` + `SkillRegistry` + `ToolExecutor` + `Orchestrator`) and cached
beside the pools. Only the factory call sites change; `crates/tool-executor`
is not edited.

The separate orchestrator service (`assistant orchestrator run`) is bound to one
space DB today. Because the web-ui publishes turns into the caller's space DB,
the orchestrator must run a worker per space too; it adopts the same supervisor.
This is the last PR in the stack and is called out in the tasks.

### 8. Removing the single pool

`Args::db_path`, the `ASSISTANT_DB_PATH` handling, the `warn!` about the
override, and the `pool_factory.space_db_path("default","default")` call in
`run_with_args` are deleted. The install root is always `~/.assistant`. The
JWT key file moves from `db_path.with_file_name("jwt_secret")` to
`<base>/jwt_secret`, which is the same file in a default install. `AppState.pool`
and the `ready` handler's `SELECT 1` switch to the org database.

## Risks / Trade-offs

- **Cold-start latency**: the first request to a space opens and migrates its DB.
  Mitigated by single-flight opening and by the supervisor warming known spaces.
- **Pool growth**: one SQLite pool per active space. Bounded by space count; an
  idle-eviction pass is a follow-up if a deployment has thousands of spaces.
- **Startup pool and telemetry**: `init_tracing` still takes the startup pool
  for the SQLite OTel exporter. Removing the startup pool (PR 4) depends on the
  separate change that deletes the local telemetry stack and moves to OTLP only;
  PR 4 lands after it, or rebases onto it.
- **403 vs 404**: cross-tenant space access returns 403 while another user's
  conversation returns 404. This is deliberate: space ids are not secret
  (they are listed to members), conversation ids are.
- **Breaking changes**: the webhook ingress URL and the removed CLI flag. Both
  are intentional (owner policy: no backward compatibility).

## Migration Plan

Ships as a stack of PRs, each green on its own:

1. This proposal and the cross-tenant isolation harness (red).
2. `SpacePools`, `SpaceCtx`, and per-request resolution for conversations,
   turns, attachments, audio, commands, capabilities, push; `openapi.json`,
   regenerated Dart client, and the Flutter client sending the selector.
3. Per-route role enforcement and the webhooks and workflows modules; `secret`
   redaction.
4. Remove the single pool, `--db-path`, and the startup `default/default` wiring;
   legacy context built from real org/space rows.
5. Space-aware background components; orchestrator service.
6. Docs and ADR.

Rollback is a revert of the stack; there is no data migration. Space databases
are not touched.

## Open Questions

- Should `GET /api/capabilities` stay reachable before a space is selected? As
  written it needs a resolved space like every other data-plane route.
