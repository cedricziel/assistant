## Purpose

Lets orgs, spaces and individual users each install remote MCP integrations
that co-exist, and defines which integrations a Persona can use in a given turn.

## ADDED Requirements

### Requirement: Integrations are installed at the org, space or user layer

An integration SHALL belong to exactly one layer: `org` (the whole org),
`space` (one space), or `user` (one user within the org). It SHALL come either
from a catalog entry or from a custom remote MCP URL. Each integration has a
`slug` that is unique within its layer and owner: kebab-case, 1–32 characters,
defaulting to the catalog slug. It also has a display name, an `enabled` flag, a
trust level (`confirm` by default, or `trust`), and a credential mode. Several
integrations, at the same layer or at different layers, MAY point at the same
catalog entry.

#### Scenario: Same service installed at every layer

- **GIVEN** org `acme` has an org-layer `linear` integration, space `eng` has a
  space-layer `linear` integration, and user `u1` has a user-layer `linear`
  integration
- **WHEN** `u1` runs a turn in `eng`
- **THEN** the turn has three distinct sets of Linear tools, one per integration

#### Scenario: Duplicate slug in one layer

- **GIVEN** space `eng` already has an integration with slug `linear`
- **WHEN** a space admin installs another space-layer integration with slug
  `linear` in `eng`
- **THEN** the response is `409`

### Requirement: Who may manage integrations at each layer

Org-layer integrations and org policy SHALL be managed only by org admins.
Space-layer integrations SHALL be managed by space admins of that space or by
org admins. User-layer integrations SHALL be managed only by the user who owns
them. Org admins MAY list and delete them, but MUST NOT read their credentials.
Listing follows visibility: members see org integrations enabled for their
spaces and the integrations of spaces they belong to. Users see only their own
user-layer integrations, and another user's integration returns `404`.

#### Scenario: Member tries to add a space integration

- **GIVEN** a user with role `member` in space `eng`
- **WHEN** they `POST` a space-layer integration for `eng`
- **THEN** the response is `403`

#### Scenario: Another user's personal integration

- **GIVEN** user `u2` owns user-layer integration `int_x`
- **WHEN** user `u1` calls `GET /api/integrations/{int_x}`
- **THEN** the response is `404`

#### Scenario: Org admin removes a departed user's integration

- **WHEN** an org admin deletes user `u2`'s integration `int_x`
- **THEN** it and its credentials are removed, and the response never contained
  credential material

### Requirement: Org policy limits what lower layers may install

Each org SHALL have a policy containing: `catalog_mode`, one of `all`
(default), `allowlist` or `none`; a list of allowed catalog entry ids;
`allow_custom_urls` (default `false`); and `allow_user_integrations` (default
`true`). Installs at the space and user layers MUST be rejected with `403` when
the policy forbids them. Org-layer installs by org admins are not restricted by
the policy. Tightening the policy SHALL disable, not delete, existing
integrations that no longer comply, and they SHALL report status
`blocked_by_policy`.

#### Scenario: Allowlist

- **GIVEN** a policy with `catalog_mode = allowlist` and allowed ids
  `[curated/linear-app-mcp]`
- **WHEN** a user installs catalog entry `curated/notion-so-mcp` at the user layer
- **THEN** the response is `403`

#### Scenario: Custom URL not allowed

- **GIVEN** `allow_custom_urls = false`
- **WHEN** a space admin installs a custom URL integration
- **THEN** the response is `403`

#### Scenario: Policy tightened after install

- **GIVEN** user `u1` has a user-layer integration from `curated/notion-so-mcp`
- **WHEN** an org admin sets `allow_user_integrations = false`
- **THEN** that integration reports status `blocked_by_policy` and its tools are
  absent from `u1`'s turns

### Requirement: Spaces choose which org integrations they use

An org-layer integration SHALL be available in every space of the org unless
its org admin restricts it to a list of spaces. A space admin MAY opt their space
out of an available org-layer integration, and MAY opt back in later.

#### Scenario: Space opts out

- **GIVEN** an org-layer `github` integration available to all spaces
- **WHEN** the admin of space `sales` opts out of it
- **THEN** turns in `sales` lack its tools, and turns in `eng` still have them

### Requirement: Credential mode is shared or per-user

Org-layer and space-layer integrations SHALL have a credential mode of `shared`
or `per_user`. A `shared` integration uses one credential, set by someone
allowed to manage that layer, for every caller. A `per_user` integration uses
the calling user's own credential for that integration and contributes no tools
for a user who has not connected one. User-layer integrations are always
`per_user` and use the owner's credential.

#### Scenario: Per-user org integration, user not connected

- **GIVEN** an org-layer `linear` integration with mode `per_user`
- **AND** user `u1` has connected their Linear account but `u2` has not
- **WHEN** `u1` and `u2` each run a turn in the same space
- **THEN** `u1`'s turn has the Linear tools acting as `u1`, and `u2`'s turn has
  no Linear tools and the integration shows as `needs_connection` for `u2`

#### Scenario: Shared space integration

- **GIVEN** a space-layer `sentry` integration with mode `shared`, connected by
  the space admin
- **WHEN** any member runs a turn in that space
- **THEN** the turn has the Sentry tools, and the calls use the admin-provided
  credential

### Requirement: A turn's effective integrations

For a turn by user U in space S, the effective set SHALL be every integration
that is enabled, permitted by policy, and has a usable credential for U, drawn
from: the org-layer integrations available in S and not opted out, the
space-layer integrations of S, and U's user-layer integrations. Turns with no
user, such as scheduled workflow turns run with a system context, SHALL include
only `shared` integrations. MCP servers configured in the TOML
`[[mcp.servers]]` section SHALL continue to apply to every turn.

#### Scenario: Scheduled turn

- **GIVEN** space `eng` with a shared `sentry` integration and a per-user org
  `linear` integration
- **WHEN** a scheduled workflow runs a turn in `eng`
- **THEN** the turn has Sentry tools and no Linear tools

#### Scenario: Disabled integration

- **WHEN** a space admin sets a space-layer integration's `enabled` to `false`
- **THEN** the next turn in that space no longer has its tools

### Requirement: Integration tools are namespaced by layer

Each tool from an integration SHALL be exposed to the model as
`mcp__{layer}_{slug}__{tool}`, where `{layer}` is `org`, `space` or `user`, so
tools from integrations at different layers never collide. Tool calls SHALL
follow the integration's trust level: `confirm` integrations require the same
user confirmation as other gated tools.

#### Scenario: Names across layers

- **GIVEN** org-layer and user-layer integrations both with slug `linear`, both
  exposing a tool `create_issue`
- **WHEN** a turn lists its tools
- **THEN** it contains `mcp__org_linear__create_issue` and
  `mcp__user_linear__create_issue`

### Requirement: Integration status is observable

Each integration's API representation SHALL include a `status` for the caller:
`ready`, `needs_connection`, `auth_expired`, `unreachable`, `blocked_by_policy`
or `disabled`. It SHALL also include the time of the last successful
connection and the number of tools discovered.
`POST /api/integrations/{id}/test` SHALL connect with the caller's effective
credential and report the discovered tool names, or the error.

#### Scenario: Revoked token

- **GIVEN** a user-layer OAuth integration whose refresh token the provider has
  revoked
- **WHEN** the user lists their integrations
- **THEN** that integration's status is `auth_expired`
