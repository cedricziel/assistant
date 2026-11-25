Prerequisite: the `multi-tenant-data-plane` stack has merged. Rebase onto `main` before starting PR 1.

## 1. PR 1 — Catalog normalizer, sync and API

- [ ] 1.1 Add a trimmed upstream fixture (`crates/core/tests/fixtures/integrations-catalog.json`, ~30 entries covering every `kind` and `auth.kind`, a missing-field entry, an `http://` entry) and failing tests for `normalize`: non-MCP skipped, header template parsed (`x-api-key: {api_key}`, `Authorization: Bearer {pat}`), missing auth → `unknown`, bad entry skipped (confirm RED)
- [ ] 1.2 Implement `CatalogEntry` + `normalize` in `crates/core/src/integrations/catalog.rs`; verify 1.1 GREEN
- [ ] 1.3 Failing `wiremock` tests for `CatalogSync`: first fetch persists to disk, `304` keeps cache, garbage `200` keeps cache, `enabled = false` makes zero requests, restart loads from disk (confirm RED)
- [ ] 1.4 Implement `[integrations.catalog]` config (`enabled`, `source_url`) and `CatalogSync` (startup + 24 h, ETag, atomic temp-file rename) in `crates/web-ui`; verify 1.3 GREEN
- [ ] 1.5 Failing route tests for `GET /api/integrations/catalog` (`q`, `category`, `limit`/`cursor`, 401 unauthenticated) and `GET /api/integrations/catalog/{id}` (404) (confirm RED)
- [ ] 1.6 Implement the catalog routes with utoipa annotations; verify 1.5 GREEN; `make dump-openapi` updates `openapi.json`

## 2. PR 2 — Secret box and storage

- [ ] 2.1 Failing tests for `SecretBox`: round trip, AAD mismatch fails, wrong key detected via `key_id` without panicking, key file created `0600`, `ASSISTANT_SECRETS_KEY` overrides the file (confirm RED)
- [ ] 2.2 Implement `SecretBox` (AES-256-GCM, `core::Secret` plaintext) in `crates/core`; verify 2.1 GREEN
- [ ] 2.3 Write migration `migrations/org/006_integrations.sql` (D1 tables, `CHECK` on layer/owner columns, unique slug index); failing migration test asserts the constraints reject a space-layer row without `space_id` and a duplicate slug (confirm RED → GREEN)
- [ ] 2.4 Failing contract tests shared by `SqliteIntegrationStore` and `InMemoryIntegrationStore`: create/get/list by layer, owner filter, delete cascades credentials and space access (confirm RED)
- [ ] 2.5 Implement both integration store impls and expose them from `OrgStorage`; verify 2.4 GREEN
- [ ] 2.6 Failing contract tests for `IntegrationCredentialStore` (shared vs per-user subject, ciphertext never equals plaintext, `updated_at` CAS update) (confirm RED); implement Sqlite + InMemory; verify GREEN
- [ ] 2.7 Failing test then implement `EffectiveIntegrations::resolve(org, space, user)` as a store query covering: allow-list rows, opt-out, disabled, `blocked_by_policy`, per-user without credential excluded, system turn → shared only; verify GREEN

## 3. PR 3 — Integration CRUD, policy and authorization

- [ ] 3.1 Failing route tests for the permission matrix (org admin / space admin / member / viewer / other user × org / space / user layer): create, list, get (404 for another user's), patch, delete; org admin may delete a user integration and never sees credentials (confirm RED)
- [ ] 3.2 Implement `POST/GET/PATCH/DELETE /api/integrations[/{id}]` with `IntegrationSummary`/`IntegrationDetail`/`CreateIntegrationRequest`; verify 3.1 GREEN
- [ ] 3.3 Failing tests for policy: `catalog_mode` all/allowlist/none, `allow_custom_urls`, `allow_user_integrations`, tightening flips status to `blocked_by_policy` without deleting (confirm RED)
- [ ] 3.4 Implement `GET/PUT /api/integrations/policy` and install-time enforcement; catalog items gain `installable_layers`; verify 3.3 GREEN
- [ ] 3.5 Failing tests then implement `PUT/DELETE /api/integrations/{id}/spaces/{space_id}` (allow list by org admin, opt-out by space admin); verify GREEN
- [ ] 3.6 `make dump-openapi`; verify `openapi.json` diff contains only the new operations and passes `openapi-spectral.yaml`

## 4. PR 4 — Header credentials, connections and turn overlay

- [ ] 4.1 Failing route tests for `PUT/DELETE /api/integrations/{id}/credential`: subject chosen by credential mode, member cannot set a shared credential, response only reports `has_credential` (confirm RED); implement; verify GREEN
- [ ] 4.2 Failing tests in `crates/tool-executor`: a turn-scoped overlay `ToolDispatcher` is consulted before the registry and its specs appear only in that turn (confirm RED); implement the overlay hook; verify GREEN
- [ ] 4.3 Failing tests for `IntegrationConnections` using an in-process rmcp Streamable HTTP test server: lazy connect, header injected from template, tools cached, eviction after idle, invalidation when `updated_at` changes, 5 s connect timeout marks `unreachable` (confirm RED)
- [ ] 4.4 Implement `IntegrationConnections` in `crates/mcp-client`; verify 4.3 GREEN
- [ ] 4.5 Failing orchestrator test with `ScriptedLlmProvider`: user U in space S sees `mcp__org_*`, `mcp__space_*`, `mcp__user_*` tools for U only; user V in the same space does not see U's tools; `confirm` trust triggers the confirmation callback (confirm RED)
- [ ] 4.6 Wire `EffectiveIntegrations` + `IntegrationConnections` into the `SpaceRuntime` turn path of the orchestrator service; verify 4.5 GREEN
- [ ] 4.7 Failing tests then implement `POST /api/integrations/{id}/test` and the `status` / `last_connected_at` / `tool_count` fields; verify GREEN

## 5. PR 5 — MCP OAuth 2.1 with DCR

- [ ] 5.1 Spike: drive rmcp `auth` (`AuthorizationManager`, `CredentialStore`, `StateStore`) against a `wiremock` authorization server with two subjects; record the outcome (rmcp vs `oauth2` crate fallback) in `design.md` D4
- [ ] 5.2 Add `server.public_url`; failing test: `oauth/start` without it returns `422` with a clear message (confirm RED → GREEN)
- [ ] 5.3 Failing `wiremock` tests for `oauth/start`: RFC 9728 discovery from `WWW-Authenticate`, RFC 8414 and OIDC fallback, DCR when `registration_endpoint` exists, reuse of the org's client for the same issuer, admin client-id override, `422` with neither; URL has `S256`, `state`, `resource` (confirm RED)
- [ ] 5.4 Implement DB-backed `CredentialStore`/`StateStore` and the start route; verify 5.3 GREEN
- [ ] 5.5 Failing tests for `GET /api/integrations/oauth/callback`: valid code stores tokens for the right subject and redirects; replayed, expired, unknown state stores nothing; member cannot start a shared flow (confirm RED); implement; verify GREEN
- [ ] 5.6 Failing tests for refresh: expiring token refreshed once under concurrency, rotated refresh token persisted, `invalid_grant` → `auth_expired` and tools omitted (confirm RED); implement; verify GREEN
- [ ] 5.7 Failing test then implement `DELETE /api/integrations/{id}/oauth` (deletes tokens, attempts RFC 7009 revocation when advertised); verify GREEN
- [ ] 5.8 `make dump-openapi`; verify the callback is documented as unauthenticated

## 6. PR 6 — Outbound URL guard

- [ ] 6.1 Failing tests for `OutboundUrlGuard`: rejects `http`, loopback, link-local, RFC 1918, CGNAT, ULA, `169.254.169.254`; allows hosts in `allow_private_hosts`; connects to the resolved address (confirm RED)
- [ ] 6.2 Move the block list from `crates/workflow-http` to `crates/core`, implement the guard, apply it to custom-URL install, connect and every OAuth hop; existing `workflow-http` tests stay GREEN; 6.1 GREEN

## 7. PR 7 — Flutter catalog and install flow

- [ ] 7.1 `make generate-flutter-client`; verify `app/packages/assistant_api` builds and `flutter analyze` is clean
- [ ] 7.2 Failing unit tests for `CatalogViewModel` (loading, success, empty, error, search debounce, category filter) (confirm RED); implement in `app/lib/features/integrations/`; verify GREEN
- [ ] 7.3 Add `CatalogEntryTile` and `AuthKindBadge` widgets in their own files with widget tests
- [ ] 7.4 Add `CatalogBrowserScreen` with widget test (search, chips, empty state, error retry)
- [ ] 7.5 Failing unit tests for `InstallIntegrationViewModel` (allowed layers from `installable_layers`, slug validation, header submit, OAuth start opens URL and polls status, 403/409/422 errors) (confirm RED); implement; verify GREEN
- [ ] 7.6 Add `InstallIntegrationSheet` with widget test covering header and OAuth paths

## 8. PR 8 — Flutter layer screens

- [ ] 8.1 Failing unit tests for `IntegrationListViewModel` per layer (loading, success, empty, error, enable toggle, opt-out toggle, connect/disconnect) (confirm RED); implement; verify GREEN
- [ ] 8.2 Extract a reusable `IntegrationListTile` (status chip, "acts as" label for shared OAuth) into its own file with widget test
- [ ] 8.3 Add the Integrations section to org settings, including a Policy tab, with widget tests
- [ ] 8.4 Add the Integrations section to space settings (own + org integrations with opt-out), with widget tests
- [ ] 8.5 Add Account → Connections (personal integrations + per-user integrations to connect), with widget tests
- [ ] 8.6 Handle the `#/integrations?connected={id}` return route on web; widget test verifies a success snackbar
- [ ] 8.7 Add Playwright screenshots for the new screens and update the moved settings baselines; document the visual diff in the PR description

## 9. PR 9 — Docs and doctor

- [ ] 9.1 Failing test then add an `assistant doctor` check: `secrets.key` present, readable and matching stored `key_id`s; catalog cache age; `public_url` set when OAuth integrations exist; verify GREEN
- [ ] 9.2 Write `docs/operations/integrations.md` (public URL, secrets key and backup, catalog source/mirror/opt-out, `allow_private_hosts`) via `oss:technical-writer`
- [ ] 9.3 Write `docs/integrations.md` (layers, policy, shared vs per-user, header vs OAuth, troubleshooting statuses) via `oss:technical-writer`
- [ ] 9.4 Add an ADR `docs/adr/adr-0013-scoped-integrations.md` capturing D1, D3, D5 and D6
- [ ] 9.5 Add `integration`, `integration layer` and `credential mode` to `docs/glossary.md`

## 10. Final verification

- [ ] 10.1 `make lint && make format && make test` pass
- [ ] 10.2 `make lint-flutter && make test-flutter` pass
- [ ] 10.3 `openspec validate integrations-catalog --strict` passes
