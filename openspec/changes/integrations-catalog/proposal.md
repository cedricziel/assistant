## Why

Connecting a Persona to an external service today means hand-editing
`[[mcp.servers]]` in the global TOML: one server list for the whole install, plain-text
headers, no OAuth, and no way for an org, a space or a single user to bring
their own integration. [integrations.sh](https://integrations.sh) publishes an
MIT-licensed catalog of about 1,350 remote MCP servers with auth hints, which
gives us a browsable source of integrations once tenancy
(`multi-tenant-data-plane`) makes scoped configuration possible.

## What Changes

- The server fetches the integrations.sh catalog on startup and daily,
  caches it on disk, and serves the remote-MCP entries through a search API.
  The source URL is configurable, and fetching can be turned off.
- Integrations can be installed at three layers that co-exist: **org**,
  **space** and **user**. A turn sees the union of the org integrations
  enabled for its space, the space's own integrations, and the calling user's
  personal integrations.
- An org or space integration uses either a **shared** credential (set by an
  admin) or **per-user** credentials (each member connects their own).
- Credentials are stored in `org.db`, encrypted with a server key.
- Auth: none, static header (API key or PAT, filled from the catalog's header
  template), and **OAuth 2.1 for MCP**: protected-resource discovery,
  authorization-server metadata, Dynamic Client Registration (RFC 7591),
  PKCE, resource indicators and refresh.
- Org admins set a policy: which catalog entries spaces and users may install,
  and whether custom (non-catalog) MCP URLs are allowed.
- MCP connections are opened per integration and credential owner. Their
  tools join the turn under a namespaced prefix, with confirmation on by default.
- Flutter: a catalog browser, an install flow, and integration lists at each layer.
- `[[mcp.servers]]` in TOML keeps working unchanged.

## Capabilities

### New Capabilities

- `integrations-catalog`: fetching, caching, normalizing and searching the
  upstream catalog.
- `scoped-integrations`: installing, listing, enabling and removing
  integrations at the org, space and user layers; policy; how a turn's
  effective set is resolved; authorization rules.
- `integration-credentials`: encrypted credential storage, header auth, and
  the outbound MCP OAuth 2.1 client with DCR.

### Modified Capabilities

None. No existing spec covers MCP client configuration.

## Impact

- `crates/storage`: new `org.db` tables. `crates/core`: types, secret box,
  `server.public_url`.
- `crates/mcp-client`: connection pool, rmcp `auth`. `crates/tool-executor`:
  turn-scoped tool overlay.
- `crates/web-ui`: `/api/integrations/...`, OAuth callback, catalog sync.
- `openapi.json`, the regenerated Dart client, and new `app/` screens.
- **Depends on** the `multi-tenant-data-plane` stack (`SpaceCtx`,
  `SpaceRuntime`); implementation starts after it merges.

## Non-goals

- OpenAPI, GraphQL and CLI catalog entries (follow-up changes).
- Stdio MCP servers outside the TOML config.
- OAuth Client ID Metadata Documents and pre-registered clients beyond a
  manual client-id field.
- External KMS or vault integration.
- Per-key API-key scope narrowing.

## Documentation

User-facing documentation is **required**: an operator guide (public URL,
secrets key, catalog source and opt-out) and a user guide (installing at each
layer, shared versus per-user credentials, connecting OAuth).

## UI changes

New screens and new navigation entries in org settings, space settings and the
account area. Playwright screenshot baselines for those settings screens will
move.
