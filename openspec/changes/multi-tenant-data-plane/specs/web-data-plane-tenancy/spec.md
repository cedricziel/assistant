## ADDED Requirements

### Requirement: Data-plane requests resolve a space from the caller's identity

Every data-plane HTTP route (conversations, turns, attachments, audio, commands,
capabilities, push, webhooks, workflows) SHALL resolve an
`(org, space)` pair from the request's `AuthContext` and a space selector, and
SHALL read and write only that space's `StorageLayer`. The selector is the
`X-Space-Id` header or the `space_id` query parameter and carries a space id.
When no selector is sent and the caller holds exactly one space role, that space
SHALL be used; when the caller holds several, the request MUST fail with
`400 {"error":"space selector required"}`.

#### Scenario: Caller with one space and no selector

- **GIVEN** a user whose only role is `member` in space `A1`
- **WHEN** they call `GET /api/conversations` with no selector
- **THEN** the response lists conversations stored in `A1`'s `space.db`

#### Scenario: Caller with several spaces and no selector

- **GIVEN** a user with roles in `A1` and `A2`
- **WHEN** they call `GET /api/conversations` with no selector
- **THEN** the response is `400` with body `{"error":"space selector required"}`

#### Scenario: Selector supplied as a query parameter

- **WHEN** a browser opens `GET /api/conversations/stream?space_id={A1}` with a
  session cookie and no custom headers
- **THEN** the stream serves conversations from `A1`

### Requirement: A caller can only reach spaces they hold a role in

The server SHALL reject a data-plane request with `403 {"error":"access denied"}`
when the selected space does not exist, belongs to a different org than the
caller's `AuthContext.org_id`, or the caller holds no role in it and is not an org
admin of that org. The three cases MUST be indistinguishable in the response.

#### Scenario: Token from another org selects a foreign space

- **GIVEN** a member of org B and a space `A1` that belongs to org A
- **WHEN** they call `GET /api/conversations` with `X-Space-Id: {A1}`
- **THEN** the response is `403` and no data from `A1` is returned

#### Scenario: Member of one space selects a sibling space

- **GIVEN** a member of space `A1` and a sibling space `A2` in the same org
- **WHEN** they call `GET /api/webhooks` with `X-Space-Id: {A2}`
- **THEN** the response is `403`

#### Scenario: Unknown space id

- **WHEN** a caller selects a space id that does not exist
- **THEN** the response is `403`, identical to the foreign-space response

#### Scenario: No cross-tenant data without a selector

- **GIVEN** a member of org B with one space `B1`, and a conversation `C` in `A1`
- **WHEN** they call `GET /api/conversations/{C}` with no selector
- **THEN** the response is `404` because `C` is not in `B1`'s database

### Requirement: Routes enforce a minimum space role

Each data-plane route SHALL require a minimum role in the resolved space:
`viewer` for read access, `member` for write access, and `space_admin` or
`org_admin` for destructive and secret operations. A role below the minimum MUST
produce `403 {"error":"insufficient role"}`. Webhook and workflow deletion,
webhook secret rotation, and `GET /api/workflows/{id}/webhook-secrets` SHALL
require the admin level. `GET /api/webhooks`
and `GET /api/webhooks/{id}` SHALL include `secret` only for admin callers.

#### Scenario: Viewer cannot write

- **GIVEN** a `viewer` in `A1`
- **WHEN** they call `POST /api/conversations`
- **THEN** the response is `403`

#### Scenario: Member cannot rotate a webhook secret

- **GIVEN** a `member` in `A1` and a webhook `W` in `A1`
- **WHEN** they call `POST /api/webhooks/{W}/rotate-secret`
- **THEN** the response is `403` and `W`'s secret is unchanged

#### Scenario: Space admin can rotate a webhook secret

- **GIVEN** a `space_admin` in `A1` and a webhook `W` in `A1`
- **WHEN** they call `POST /api/webhooks/{W}/rotate-secret`
- **THEN** the response is `200` with the new secret

#### Scenario: Member does not receive webhook secrets

- **WHEN** a `member` calls `GET /api/webhooks/{W}`
- **THEN** the response is `200` and has no `secret` field

### Requirement: Conversations are scoped to their owner

Conversation reads and writes SHALL be filtered by `AuthContext.user_id`. A caller
MUST NOT list, read, modify, delete, stream, cancel a turn on, or fetch
attachments or audio for a conversation owned by another user, even within the
same space. Access to another user's conversation by id SHALL return `404`.

#### Scenario: List shows only own conversations

- **GIVEN** users Alice and Bob, both members of `A1`, each with a conversation
- **WHEN** Alice calls `GET /api/conversations`
- **THEN** only Alice's conversation is returned

#### Scenario: Reading another user's conversation

- **WHEN** Bob calls `GET /api/conversations/{alice_conversation}`
- **THEN** the response is `404`

#### Scenario: Children of a foreign conversation are hidden

- **WHEN** Bob calls `GET /api/attachments/{id}` for an attachment on Alice's
  conversation, or `POST /api/conversations/{alice_conversation}/turns/{t}/cancel`
- **THEN** the response is `404` and nothing is served or cancelled

### Requirement: Public workflow webhooks resolve their space from the URL

The public workflow trigger SHALL be served at
`POST /workflow-hooks/{org_slug}/{space_slug}/{id}/{token}`. The handler SHALL
open the named space through the pool cache and validate the token against that
space's workflows only. The previous path without slugs MUST NOT be routed.

#### Scenario: Token valid only in its own space

- **GIVEN** workflow `F` with hook token `T` in space `A1`
- **WHEN** `POST /workflow-hooks/{org_a}/{A2_slug}/{F}/{T}` is called
- **THEN** the response is `404` and no run is created

#### Scenario: Old ingress path

- **WHEN** `POST /workflow-hooks/{id}/{token}` is called
- **THEN** the response is `404`

### Requirement: Background workers run per space

The web-ui process SHALL run workflow, schedule and event adapters once per
known `(org, space)` and MUST NOT bind them to a single startup pool. A workflow
run SHALL execute turns with an `AuthContext` scoped to the run's own org and
space.

#### Scenario: Scheduled workflow in a non-default space

- **GIVEN** a schedule-triggered workflow in space `B1`
- **WHEN** its schedule fires
- **THEN** the run record is written to `B1`'s database and no row is written to
  any other space's database

#### Scenario: Space created after startup

- **WHEN** an org admin creates a space while the server is running
- **THEN** within one reconcile interval its adapters are running
