## ADDED Requirements

### Requirement: API key creation accepts an expiry in days

The `POST /api/users/me/api-keys` endpoint SHALL accept an optional
`expires_in_days` field of type unsigned integer. When set the server
SHALL compute `expires_at = now() + days` and persist it on the new
record. When absent or null the new key SHALL have no expiry.

#### Scenario: Key created with a 90-day expiry

- **WHEN** a client submits `POST /api/users/me/api-keys` with body
  `{"name": "ci", "expires_in_days": 90}`
- **THEN** the response SHALL include an `expires_at` ISO-8601 timestamp
  approximately 90 days in the future (±1 minute)
- **THEN** the persisted record's `expires_at` SHALL match the response
- **THEN** subsequent `GET /api/users/me/api-keys` SHALL list the key with
  the same `expires_at`

#### Scenario: Key created without expiry

- **WHEN** a client submits `POST /api/users/me/api-keys` with body
  `{"name": "ci"}` (no `expires_in_days` field)
- **THEN** the response SHALL have `expires_at: null`
- **THEN** the persisted record's `expires_at` SHALL be `None`

#### Scenario: Zero or negative expiry rejected

- **WHEN** a client submits `POST /api/users/me/api-keys` with
  `expires_in_days: 0`
- **THEN** the server SHALL respond with `400 Bad Request` and a clear
  error message
- **THEN** no record SHALL be persisted

#### Scenario: Expiry above ceiling rejected

- **WHEN** a client submits `POST /api/users/me/api-keys` with
  `expires_in_days` greater than `400`
- **THEN** the server SHALL respond with `400 Bad Request`
- **THEN** no record SHALL be persisted

### Requirement: API keys screen is reachable from Settings

The Flutter app SHALL surface a navigation entry to the API keys
management screen from a Settings landing screen. The API keys screen
SHALL NOT occupy a top-level nav shell destination.

#### Scenario: Settings entry navigates to API keys

- **WHEN** the user opens the Settings landing screen AND taps the
  "API keys" entry
- **THEN** the router SHALL navigate to `/api-keys`
- **THEN** the existing `ApiKeysScreen` SHALL render

#### Scenario: API keys screen not in top-level nav

- **WHEN** the nav shell renders on any breakpoint (desktop rail,
  tablet hamburger, mobile bottom bar)
- **THEN** "API keys" SHALL NOT appear as a primary or overflow nav
  destination
- **THEN** the only path to `/api-keys` SHALL be via Settings or a
  direct URL

### Requirement: API key create dialog supports scope and expiry selection

The Flutter API key create dialog SHALL allow the user to pick a set of
scopes and an expiry duration in addition to a name.

#### Scenario: Dialog renders all three inputs

- **WHEN** the user taps the floating action button on `/api-keys`
- **THEN** the create dialog SHALL render a name text field, a scope
  picker, and an expiry chip row
- **THEN** the expiry chip row SHALL default to the `90 days` chip
  selected
- **THEN** the scope picker SHALL default to no scopes selected

#### Scenario: Submit with selected scopes

- **WHEN** the user enters a name, selects `personas:read` and
  `conversations:write` in the scope picker, and submits
- **THEN** the request body SHALL include
  `"scopes": ["personas:read", "conversations:write"]`

#### Scenario: Submit with selected expiry

- **WHEN** the user selects the `30 days` chip and submits
- **THEN** the request body SHALL include `"expires_in_days": 30`

#### Scenario: Submit with "No expiry"

- **WHEN** the user selects the `No expiry` chip and submits
- **THEN** the request body SHALL OMIT `expires_in_days` (or send it
  as `null`)
- **THEN** the chip SHALL be visually annotated with a subdued warning
  hint before submission

#### Scenario: "Read everything" preset

- **WHEN** the user taps the `Read everything` quick-fill button
- **THEN** all `<resource>:read` scopes SHALL become selected
- **THEN** all non-read scopes SHALL be deselected

#### Scenario: "Read + write everything" preset

- **WHEN** the user taps the `Read + write everything` quick-fill button
- **THEN** all `<resource>:read` and `<resource>:write` scopes SHALL
  become selected
- **THEN** all other actions (delete, execute, manage) SHALL be
  deselected

### Requirement: API key list tile shows human-readable metadata

Each tile in the API keys list SHALL render the key's creation date and
expiry as relative timestamps, and SHALL summarise scopes.

#### Scenario: Relative dates

- **WHEN** a key was created 3 days ago
- **THEN** the tile SHALL render "Created 3 days ago" (or the
  locale-appropriate equivalent)
- **WHEN** the user hovers (desktop) or long-presses (mobile) the date
- **THEN** the absolute ISO-8601 timestamp SHALL be revealed in a
  tooltip or bottom sheet

#### Scenario: Expiry rendering

- **WHEN** a key has `expires_at` 87 days in the future
- **THEN** the tile SHALL render "Expires in 87 days" with subdued
  styling
- **WHEN** a key has `expires_at: null`
- **THEN** the tile SHALL render "No expiry"

#### Scenario: Scope summary, few scopes

- **WHEN** a key has 1-3 scopes
- **THEN** the tile SHALL render each scope as a small chip with the
  `resource:action` label

#### Scenario: Scope summary, many scopes

- **WHEN** a key has more than 3 scopes
- **THEN** the tile SHALL render the count as "N scopes" with an affordance
  to expand the full list
