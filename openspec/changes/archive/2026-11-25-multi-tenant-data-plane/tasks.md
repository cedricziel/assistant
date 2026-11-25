## 1. PR 1 — Proposal and cross-tenant isolation harness

- [x] 1.1 Write `proposal.md`, `design.md`, and the spec deltas; run `openspec validate multi-tenant-data-plane`
- [x] 1.2 Add `TenantHarness` test fixture (two orgs, three spaces, users per role, seeded conversations/webhooks/workflows)
- [x] 1.3 Write cross-tenant negative tests for conversations, turns, attachments, audio, commands, capabilities, push (confirm RED)
- [x] 1.4 Write cross-tenant negative tests for webhooks and workflows (confirm RED)
- [x] 1.5 Write role-matrix tests (viewer cannot write; member cannot rotate secrets or delete) (confirm RED)

## 2. PR 2 — Per-request space resolution

- [x] 2.1 Failing test: `OrgPoolFactory::space_storage` returns one pool for concurrent first calls and rejects traversal slugs
- [x] 2.2 Implement `space_storage` with single-flight cache and slug validation in `crates/storage`
- [x] 2.3 Failing tests for `SpaceCtx`: selector header, query param, single-space default, ambiguous (400), foreign org (403), unknown (403)
- [x] 2.4 Implement `SpacePools` + `SpaceCtx` in `crates/web-ui/src/space.rs`
- [x] 2.5 Migrate conversations, turns, attachments, audio, commands, capabilities, push handlers to `SpaceCtx`; swap `ApiState.pool` for the resolver
- [x] 2.6 Scope conversation store to `auth.user_id` in every handler (owner-only; 404 for foreign ids)
- [x] 2.7 Update `openapi.json` (`make dump-openapi`) and regenerate the Dart client (`make generate-flutter-client`)
- [x] 2.8 Failing Flutter test: data-plane Dio calls carry `X-Space-Id`; implement interceptor; EventSource URL carries `space_id`
- [x] 2.9 Harness for these modules goes GREEN

## 3. PR 3 — Per-route role enforcement

- [x] 3.1 Failing tests for `Access` mapping (`Viewer→Read`, `Member→Write`, `SpaceAdmin`/`OrgAdmin`→`Admin`)
- [x] 3.2 Implement `SpaceCtx::require(Access)`; delete unused `require_scope_layer`
- [x] 3.3 Migrate webhooks and workflows to `SpaceCtx` with the levels in the design table
- [x] 3.4 Redact webhook `secret` for non-admin callers (optional field); update OpenAPI and regenerate the Dart client
- [x] 3.5 Move the public workflow trigger to `/workflow-hooks/{org_slug}/{space_slug}/{id}/{token}`; update the URL returned by `webhook-secrets`
- [x] 3.6 Harness for these modules goes GREEN

## 4. PR 4 — Remove the single pool

- [x] 4.1 Failing test: `webui serve --db-path` is rejected; startup creates no `default/default` pool
- [x] 4.2 Delete `Args::db_path`, `ASSISTANT_DB_PATH` handling, and the startup `StorageLayer`; derive the JWT key path from the install root
- [x] 4.3 Build the legacy-token `AuthContext` from the real seeded org and space rows
- [x] 4.4 Rebase onto the telemetry-removal change so `init_tracing` no longer needs a startup pool
- [x] 4.5 Point `/ready` and `AppState` at the org database
- [x] 4.6 Update `docs/operations/`, `docs/web-ui.md`, and the `pool_factory.rs` module docs that mention `--db-path`

## 5. PR 5 — Space-aware background components

- [x] 5.1 Failing test: a schedule-triggered workflow in space B1 writes its run only to B1's database
- [x] 5.2 Implement `SpaceSupervisor` (list spaces, reconcile every 30 s, stop on delete) and `SpaceRuntime` factory
- [x] 5.3 Make `OrchestratorTurnClient` take the run's `(org, space)` and a scoped `AuthContext`
- [x] 5.4 Adopt the supervisor in `assistant orchestrator run` so it works every space's turn queue and title generation

## 6. PR 6 — Documentation

- [x] 6.1 Add an ADR in `docs/adr/` for data-plane tenancy and update `docs/glossary.md` if terms change
- [x] 6.2 Document the space selector, role table, webhook ingress URL, and flag removal in `docs/web-ui.md` and `docs/multi-user.md`
- [x] 6.3 Run `make lint && make format && make test && make lint-flutter && make test-flutter`
