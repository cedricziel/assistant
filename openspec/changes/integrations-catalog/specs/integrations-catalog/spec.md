## Purpose

Gives every org a searchable, locally cached copy of the integrations.sh catalog,
so users can choose remote MCP servers to install without the server calling a
third party on each request.

## ADDED Requirements

### Requirement: The server keeps a cached copy of the upstream catalog

The server SHALL fetch the catalog from the configured source URL (default
`https://integrations.sh/api.json`) at startup and then every 24 hours. It SHALL
send a conditional request when it holds an `ETag` or `Last-Modified` value from a
previous fetch. A successful fetch SHALL replace the cached copy atomically and
persist it to disk, so a restart serves the last good copy before the first fetch
completes. A failed, non-2xx or unparseable fetch MUST leave the cached copy in
place and be logged as a warning.

#### Scenario: First start with network access

- **GIVEN** no cached catalog on disk
- **WHEN** the server starts and the source URL returns a valid catalog
- **THEN** `GET /api/integrations/catalog` returns entries from that catalog

#### Scenario: Upstream unchanged

- **GIVEN** a cached catalog fetched with `ETag: "v1"`
- **WHEN** the next refresh receives `304 Not Modified`
- **THEN** the cached copy and its fetch timestamp of record are unchanged, and
  the last-checked timestamp is updated

#### Scenario: Upstream returns garbage

- **GIVEN** a cached catalog with 1,350 entries
- **WHEN** a refresh receives a `200` whose body is not a valid catalog
- **THEN** the API still serves the 1,350 cached entries

### Requirement: Catalog fetching can be disabled or redirected

`server.toml` SHALL accept `[integrations.catalog] enabled` (default `true`) and
`source_url`. When `enabled = false` the server MUST NOT make any request to the
source URL. The catalog API then returns an empty list, and custom integrations
remain available if org policy allows them.

#### Scenario: Air-gapped install

- **GIVEN** `[integrations.catalog] enabled = false`
- **WHEN** the server runs for 25 hours
- **THEN** no request is made to the source URL and
  `GET /api/integrations/catalog` returns `200` with an empty `items` array

#### Scenario: Mirrored catalog

- **GIVEN** `source_url = "https://mirror.internal/api.json"`
- **WHEN** the server refreshes the catalog
- **THEN** it fetches from the mirror and not from integrations.sh

### Requirement: Only installable remote MCP entries are exposed

The catalog API SHALL expose only entries with `kind = "mcp"` and an `https`
`connectUrl`. Each entry SHALL be normalized to a stable shape: `id`, `name`,
`description`, `domain`, `icon_url`, `categories`, `connect_url`, and `auth`, where
`auth.kind` is one of `none`, `header`, `oauth`, `mixed` or `unknown`. A header
template (for example `Authorization: Bearer {pat}`) SHALL be parsed into a header
name and a named placeholder. Unknown upstream fields MUST be ignored, and an
entry missing a required field MUST be skipped without failing the whole fetch.

#### Scenario: Non-MCP entries are hidden

- **GIVEN** an upstream catalog with `mcp`, `openapi`, `graphql` and `cli` entries
- **WHEN** a user lists the catalog
- **THEN** only `mcp` entries with an `https` connect URL are returned

#### Scenario: Header template is parsed

- **GIVEN** an upstream entry with
  `auth: {kind: "api_key", header: "x-api-key: {api_key}"}`
- **WHEN** it is listed
- **THEN** its `auth` is `{kind: "header", header_name: "x-api-key",
placeholder: "api_key", value_prefix: ""}`

#### Scenario: Entry without auth hint

- **GIVEN** an upstream MCP entry with no `auth` field
- **WHEN** it is listed
- **THEN** its `auth.kind` is `unknown`

### Requirement: Any authenticated org member can search the catalog

`GET /api/integrations/catalog` SHALL require an authenticated caller. It SHALL
support a case-insensitive `q` text filter over name, domain and description, a
`category` filter, and `limit`/`cursor` pagination (default 50, maximum 200). Each
item SHALL report whether the caller's org policy allows installing it, and at
which layers. `GET /api/integrations/catalog/{id}` SHALL return one entry or `404`.

#### Scenario: Text search

- **WHEN** a member calls `GET /api/integrations/catalog?q=linear`
- **THEN** every returned item contains `linear` in its name, domain or description

#### Scenario: Unauthenticated request

- **WHEN** a request without credentials calls `GET /api/integrations/catalog`
- **THEN** the response is `401`
