## Context

See `proposal.md` for motivation and `specs/` for the behavior contract.

Current state that shapes the approach:

- MCP servers are read from the TOML `[[mcp.servers]]` section
  (`crates/core/src/types/skills_mcp.rs`). `McpClientManager::start` connects
  them once per process (`crates/web-ui/src/lib.rs:505`), and their tools are
  registered as _ambient_ tools on one `ToolExecutor` that every turn shares.
  Remote servers use rmcp's Streamable HTTP client with static headers. There
  is no outbound OAuth.
- Turns run in the orchestrator service (`assistant orchestrator run`), not
  in the web-ui process. After `multi-tenant-data-plane`, both processes run a
  `SpaceSupervisor` and build per-space orchestrators through `SpaceRuntime`.
  Web-ui owns the HTTP API and the browser-facing OAuth callback. The
  orchestrator owns tool execution. Both share `org.db` and `~/.assistant/`.
- `org.db` already holds org-scoped resources with an optional `space_id`
  (`catalog_items`, `catalog_subscriptions`, `interface_instances`).
- No credential store exists. `aes-gcm` is already a dependency (web-ui).
- rmcp 3.x ships an `auth` feature with `AuthorizationManager`, metadata
  discovery, DCR, PKCE, and pluggable `CredentialStore` and `StateStore`
  traits.
- Upstream snapshot (2026-11-25): 5,674 entries, of which 1,360 are `mcp`
  (1,351 with `connectUrl`). Upstream auth kinds are `none`, `oauth`,
  `api_key`, `token` and `mixed`; 943 MCP entries have no hint. The upstream
  README marks the schema as unstable (v0.0.1).

## Goals / Non-Goals

**Goals:**

- One data model that serves all three layers, so the effective set for a
  turn is a single query.
- Credentials are never decrypted in the web-ui process except when they are
  written, or exchanged during OAuth.
- Turn-scoped tools: integration tools must not leak into another user's or
  another space's turn through the shared ambient registry.
- Isolate upstream schema churn behind a normalizer with fixture tests.

**Non-Goals:**

- Live tool-list streaming to the UI.
- Key rotation tooling. The schema stores a `key_id` so rotation can be added
  later.
- Multi-instance (horizontally scaled) token-refresh coordination beyond what a
  SQLite row lock gives us.

## Decisions

### D1. All integration state lives in `org.db`

New tables (migration `org/006_integrations.sql`):

- `integrations`: `id` (`int_<uuid>`), `org_id`, `layer`
  (`org|space|user`), `space_id` (nullable; set for the space layer),
  `owner_user_id` (nullable; set for the user layer), `catalog_entry_id`
  (nullable), `connect_url`, `slug`, `name`, `auth_kind`, `header_name`,
  `header_value_prefix`, `credential_mode` (`shared|per_user`), `trust`,
  `enabled`, `oauth_client_id_override`, timestamps. Unique on
  `(org_id, layer, coalesce(space_id,''), coalesce(owner_user_id,''), slug)`.
  A `CHECK` constraint ties `layer` to which owner column is set.
- `integration_space_access`: `(integration_id, space_id, mode)`, where `mode`
  is `allow` or `opt_out`. An org-layer integration with no `allow` rows is
  available in all spaces.
- `integration_credentials`: `(integration_id, subject_user_id NULL, kind,
ciphertext, nonce, key_id, expires_at, status, updated_at)`, unique per
  `(integration_id, subject)`. A `NULL` subject means the shared credential.
- `integration_oauth_clients`: `(org_id, issuer, redirect_uri, client_id,
client_secret_ct, registration_json)`. One DCR client per org per
  authorization server.
- `integration_oauth_states`: `(state_hash, integration_id, subject_user_id,
pkce_verifier_ct, created_at)`. Rows are deleted when used, and swept after
  10 minutes.
- `integration_policy`: one row per org.

_Why not space-layer rows in `space.db`?_ The effective-set query would join
across two databases, and credentials would be split across many files that are
backed up separately. Users and policy are org-level already. _Why not reuse
`catalog_items`?_ Those rows are type-tagged metadata for skills, templates and
interfaces, with subscription semantics. Integrations need credentials, three
owner kinds and per-user state. Overloading the table would turn every column
into a nullable union.

Stores follow the ADR-0009 trait-pair pattern (`IntegrationStore` with
`SqliteIntegrationStore` and `InMemoryIntegrationStore`, and likewise for the
credential store).

### D2. Catalog cache is a global file plus an in-memory index

`CatalogSync` in web-ui fetches with `ETag`/`If-Modified-Since`. It writes
`~/.assistant/cache/integrations/catalog.json` and `meta.json` through a temp
file and rename, then swaps an `Arc<CatalogIndex>` behind a `tokio::sync::RwLock`.
`CatalogIndex` holds normalized entries plus a lowercase search string per
entry. With about 1,350 entries, a linear scan is fast enough, so there is no
search engine.

The normalizer is a pure function `normalize(raw: &Value) -> Option<CatalogEntry>`.
It is tested against a trimmed real snapshot in
`crates/core/tests/fixtures/integrations-catalog.json`. Installed integrations
copy `connect_url`, auth kind and header template at install time, so an
upstream change or removal never alters a running integration. The UI shows
"catalog entry changed" when they diverge.

_Alternatives:_ a bundled snapshot (stale, and a release is needed to update),
or a live proxy (runtime dependency, and it leaks queries to a third party).
The user chose periodic fetch.

### D3. AES-256-GCM with a server key and row-bound AAD

The key comes from `ASSISTANT_SECRETS_KEY` (base64, 32 bytes) or from
`~/.assistant/secrets.key` (generated with `0600`, like `jwt_secret`). Each
encryption uses a random 96-bit nonce. The AAD is
`"{integration_id}|{subject or 'shared'}|{kind}"`. `key_id` holds the first 8
bytes of SHA-256(key), so a key mismatch is detected before decryption and
reported as `auth_expired` instead of crashing. Decryption is behind a
`SecretBox` service in a new `crates/core` module, and plaintext is returned as
`core::Secret` so it is always redacted in logs.

_Alternatives:_ OS keychain (unavailable on headless servers and in Docker), or
per-org keys derived with HKDF from the master key. HKDF derivation is cheap to
add later; the AAD already gives row binding.

### D4. OAuth via rmcp `auth` with DB-backed stores

We enable rmcp's `auth` feature in `assistant-mcp-client` and implement its
`CredentialStore` and `StateStore` on top of the D1 tables, keyed by
`(integration_id, subject)`. The flow is:

- `POST /api/integrations/{id}/oauth/start` (web-ui, authenticated)
  authorizes the caller for the subject (spec: _OAuth subject follows
  credential mode_). It then runs discovery and finds or creates the DCR client
  for `(org, issuer)`, persists the state and PKCE verifier, and returns
  `authorization_url`.
- `GET /api/integrations/oauth/callback` is unauthenticated, because the
  browser arrives from the provider without our bearer token. Its authority is
  the single-use, hashed, 10-minute `state`, which is bound to the subject. It
  exchanges the code, stores the tokens, and redirects to
  `{public_url}/#/integrations?connected={id}`. Native clients open
  `authorization_url` in the system browser and poll
  `GET /api/integrations/{id}` until the status becomes `ready`.

The DCR request uses `client_name = "Assistant ({org name})"`,
`redirect_uris = [callback]`, `grant_types = [authorization_code,
refresh_token]`, and `token_endpoint_auth_method = none` if supported,
otherwise `client_secret_post`.

_Alternative:_ the `oauth2` crate plus hand-written RFC 9728 and RFC 7591. We
keep it as the fallback if rmcp's per-session `AuthorizationManager` cannot be
shared across subjects cleanly. Task 3.1 is a spike that decides this before
the rest of section 3.

### D5. Connections are pooled in the orchestrator, keyed by integration and subject

`IntegrationConnections` (in `assistant-mcp-client`) maps
`(integration_id, subject)` to a connected rmcp client and its cached
`tools/list`. A connection is opened lazily on the first turn that needs it,
refreshed on `notifications/tools/list_changed`, and evicted after 10 minutes
idle. It is invalidated when the integration or credential `updated_at`
changes; each turn checks this with one cheap query. Token refresh happens in
the `CredentialStore` under a per-subject `tokio::sync::Mutex`, and is written
back with a compare-and-swap on `updated_at`, so two orchestrator workers do not
both rotate a refresh token.

The web-ui process holds only a short-lived client for the `test` endpoint.

_Why not per `SpaceRuntime`?_ Org-layer and user-layer integrations span
spaces. Per-space pools would open duplicate connections with the same
credential.

### D6. Turn-scoped tool overlay

Ambient registration is process-wide, so it would leak tools between
tenants. We add a turn-scoped overlay instead: the turn context carries an
`Arc<dyn ToolDispatcher>` overlay built by `EffectiveIntegrations::resolve(org,
space, user)`, which runs the spec's effective-set query, then
`IntegrationConnections`, then wraps tools as `mcp__{layer}_{slug}__{tool}`.
`ToolExecutor` looks up the overlay first, then its registry. The overlay's
tool specs are appended to the turn's tool list. Trust maps onto the existing
`McpTrustLevel` and the safety gate, so `confirm` integrations need a
confirmation, and scheduled turns with `AutoDenyConfirmation` will therefore
deny `confirm` tools.

This touches `crates/tool-executor`, which `multi-tenant-data-plane` leaves
alone. It is a small, additive extension point. TOML servers stay ambient.

An integration that fails to connect during resolution is logged, marked
`unreachable`, and skipped. A broken integration never fails the turn.

### D7. HTTP surface

The integration routes are under `/api/integrations`. They use the org-DB
router style: an `AuthContext` plus an explicit `layer` and `space_id` in the
body or query for space-layer operations. They do not use `SpaceCtx`, because
org-layer and user-layer integrations are not space data. The routes:

- `GET /catalog`, `GET /catalog/{id}`
- `GET /` (filter: `layer`, `space_id`) and `POST /`
- `GET|PATCH|DELETE /{id}`
- `PUT|DELETE /{id}/credential` (header value; the subject comes from the mode)
- `POST /{id}/oauth/start`, `GET /oauth/callback`, `DELETE /{id}/oauth`
- `POST /{id}/test`
- `PUT|DELETE /{id}/spaces/{space_id}` (allow or opt-out)
- `GET|PUT /policy`

They follow the `*Summary`, `*Detail` and `*Request` naming and are added to
`openapi.json`. Responses report `has_credential` and never include a value.

### D8. SSRF guard

A shared `OutboundUrlGuard` resolves the host, rejects loopback, link-local,
RFC 1918, CGNAT, ULA and metadata IPs unless the host is listed in
`allow_private_hosts`, and is applied on every connect and OAuth hop. It reuses
the block list from `crates/workflow-http`, which is moved into `crates/core`.
DNS rebinding is mitigated by connecting to the resolved address through
reqwest's `resolve` override.

### D9. Flutter

New `features/integrations/`:

- A catalog browser with search, category chips and auth badges.
- An install sheet, where the user picks a layer they may write to, sets a
  slug and credential mode, enters a header value, or starts OAuth.
- An integration list widget, reused in org settings (with a Policy tab), in
  space settings (own integrations plus org integrations with opt-out toggles),
  and in Account → Connections (personal integrations plus per-user
  integrations to connect).

State follows the `ux-principles` Riverpod loading, error and empty patterns.

## Risks / Trade-offs

- [Upstream schema churn or disappearance] → Normalizer with fixture tests,
  install-time copy of the connect data, a configurable mirror URL, and
  last-good cache.
- [Shared OAuth credentials act as one person for everyone in the space] → The
  UI shows "acts as {connected account}" and a warning, `per_user` is the
  default for OAuth, and only admins can connect shared integrations.
- [Prompt-injection via third-party MCP tools] → `confirm` trust by default,
  `trust` only settable by layer managers, and tool output passes through the
  existing tool-result handling. This is no worse than TOML servers today.
- [Key loss makes every credential unreadable] → Documented backup of
  `secrets.key`, a clear `auth_expired` status, and `assistant doctor` checks
  the key against stored `key_id`s.
- [Turn latency when many integrations are cold] → Resolve and connect in
  parallel with a 5-second per-integration timeout. Slow integrations are
  skipped for that turn.
- [DCR client sprawl at providers] → One client per org per authorization
  server, reused across subjects.
- [The tool-executor change conflicts with the tenancy stack] → It starts after
  that stack merges, as the first PR of this change's stack.

## Migration Plan

Additive only: new `org.db` migration, new config keys with defaults, and new
routes. TOML `[[mcp.servers]]` keep working unchanged. Rollback means
reverting the binary; the unused tables are harmless. There is no data
migration from TOML to the database, and no compatibility layer is needed.

PR stack, each under about 500 lines where practical:

1. Catalog normalizer, sync and API.
2. Secret box and `org.db` schema with stores.
3. Integration CRUD, policy and authorization.
4. Header credentials, connection pool and turn overlay.
5. OAuth (spike, then implementation).
6. SSRF guard hardening.
7. Flutter catalog and install flow.
8. Flutter layer screens.
9. Docs.
