## ADDED Requirements

### Requirement: Caller can retrieve their own user record

The system SHALL expose `GET /api/users/me` returning a `UserDetail` for the authenticated caller. The endpoint MUST work regardless of the caller's organization auth mode.

#### Scenario: Authenticated user retrieves own profile

- **WHEN** a logged-in user issues `GET /api/users/me` with a valid bearer token
- **THEN** the response is `200 OK` with a `UserDetail` body containing `id`, `org_id`, `email`, `name`, `created_at`, `updated_at`

#### Scenario: Unauthenticated request is rejected

- **WHEN** a request to `GET /api/users/me` arrives without a valid bearer token or API key
- **THEN** the response is `401 Unauthorized`

#### Scenario: Endpoint works for OIDC-managed users

- **WHEN** a user whose org has `auth_mode = "oidc"` issues `GET /api/users/me`
- **THEN** the response is `200 OK` with the same `UserDetail` shape as a password-mode user

### Requirement: Caller can update own name and email

The system SHALL expose `PATCH /api/users/me` accepting an `UpdateCurrentUserRequest` of the shape `{ name?: string, email?: string }`. On success the system SHALL persist the new values and return the updated `UserDetail` plus a `previous_email` field when email changed.

#### Scenario: User updates own name

- **WHEN** an authenticated user submits `PATCH /api/users/me` with `{"name": "New Name"}`
- **THEN** the response is `200 OK`, the user's name is updated in storage, and the response body shows `name: "New Name"`

#### Scenario: User updates own email

- **WHEN** an authenticated user submits `PATCH /api/users/me` with `{"email": "new@example.com"}`
- **AND** no other user in the same org already has that email
- **THEN** the response is `200 OK`, the email is updated in storage, and the response body includes `email: "new@example.com"` and `previous_email: "<old>"`

#### Scenario: Email collides with existing user in same org

- **WHEN** a user submits `PATCH /api/users/me` with an email already used by another user in the same org
- **THEN** the response is `409 Conflict` with body `{"error": "email already exists in this org"}` and storage is unchanged

#### Scenario: Empty body is a no-op

- **WHEN** a user submits `PATCH /api/users/me` with `{}`
- **THEN** the response is `200 OK` with the unchanged `UserDetail` and no `previous_email` field

#### Scenario: OIDC org rejects the update

- **WHEN** a user whose org has `auth_mode = "oidc"` submits `PATCH /api/users/me` with any body
- **THEN** the response is `409 Conflict` with body `{"error": "account managed by identity provider <issuer>"}` and storage is unchanged

#### Scenario: Invalid email format is rejected

- **WHEN** a user submits `PATCH /api/users/me` with an email failing basic validation (no `@`, empty)
- **THEN** the response is `400 Bad Request` with body `{"error": "<reason>"}`

### Requirement: Caller can change own password

The system SHALL expose `POST /api/users/me/password` accepting a `ChangePasswordRequest` of the shape `{ current_password: string, new_password: string }`. The system MUST verify the current password against the stored argon2id hash, hash the new password, persist it, and revoke all of the user's refresh tokens **except** the one underlying the current request. API keys MUST NOT be affected.

#### Scenario: Successful password change

- **WHEN** an authenticated user submits a correct `current_password` and a non-empty `new_password`
- **THEN** the response is `204 No Content`, the stored hash is updated, the calling session's refresh token still works, and all of the user's other refresh tokens are revoked

#### Scenario: Wrong current password

- **WHEN** an authenticated user submits a `current_password` that does not match the stored hash
- **THEN** the response is `401 Unauthorized` with body `{"error": "current password is incorrect"}` and the stored hash is unchanged

#### Scenario: Empty new password

- **WHEN** an authenticated user submits a `new_password` of empty string
- **THEN** the response is `400 Bad Request` with body `{"error": "new password must not be empty"}`

#### Scenario: API keys survive password change

- **WHEN** a user with at least one active API key successfully changes their password
- **THEN** that API key still authenticates subsequent requests

#### Scenario: OIDC org rejects password change

- **WHEN** a user whose org has `auth_mode = "oidc"` submits `POST /api/users/me/password`
- **THEN** the response is `409 Conflict` with body `{"error": "account managed by identity provider <issuer>"}` and the stored hash is unchanged

### Requirement: Web UI exposes an Account section

The Web UI SHALL render an "Account" section on the Settings page containing three controls: edit name, edit email (with confirm-email field), change password (current + new + confirm new). On submit each control SHALL call the corresponding `/api/users/me` endpoint.

#### Scenario: Account section visible for password-mode org

- **WHEN** a user whose org has `auth_mode = "password"` opens the Settings page
- **THEN** the Account section is visible with name, email, and change-password controls enabled

#### Scenario: Account section read-only for OIDC org

- **WHEN** a user whose org has `auth_mode = "oidc"` opens the Settings page
- **THEN** the Account section displays the user's current name and email as read-only fields with a banner reading "Managed by your identity provider (<issuer>)" and the change-password control is hidden

#### Scenario: Email confirmation typo blocks submission

- **WHEN** a user enters a new email and a different value in the confirm-email field
- **THEN** the form prevents submission and shows an inline error

### Requirement: Flutter app exposes an Account screen

The Flutter app SHALL register a route at `/settings/account` reachable via a tile on the Settings screen. The screen SHALL contain the same three controls as the Web UI Account section and call the same endpoints via the generated `assistant_api` client.

#### Scenario: Account tile navigates to Account screen

- **WHEN** a user taps the "Account" tile on the Settings screen
- **THEN** the app pushes the Account screen using the platform-appropriate navigator (Cupertino on Apple Touch, Material elsewhere)

#### Scenario: Account screen forms call /api/users/me

- **WHEN** a user submits the email-change form on the Account screen with a new value
- **THEN** the app issues `PATCH /api/users/me` and on success updates the displayed value and shows a confirmation snackbar/banner naming the previous email

#### Scenario: OIDC org disables write controls

- **WHEN** the current user's org has `auth_mode = "oidc"`
- **THEN** the Account screen shows name and email as read-only and hides the change-password form, with a banner naming the IdP issuer

### Requirement: CLI exposes an `account` subcommand group

The `assistant` CLI SHALL expose a top-level subcommand `account` with children `show`, `set-email <email>`, `set-name <name>`, and `change-password`. Each subcommand SHALL use the existing authenticated-client helper (refreshes credentials, supports `--api-key`/`--server` overrides).

#### Scenario: `assistant account show` prints current profile

- **WHEN** a logged-in user runs `assistant account show`
- **THEN** the command prints email, name, org_id, and auth mode and exits `0`

#### Scenario: `assistant account set-email` updates email

- **WHEN** a logged-in user runs `assistant account set-email new@example.com`
- **THEN** the command issues `PATCH /api/users/me` and prints `Email changed from <old> to new@example.com.` on success

#### Scenario: `assistant account change-password` prompts interactively

- **WHEN** a logged-in user runs `assistant account change-password` from a TTY
- **THEN** the command prompts for current password, new password, and confirmation (all hidden), submits to `POST /api/users/me/password`, and prints `Password changed.` on success
- **AND** the prompt values never appear in shell history or process arguments

#### Scenario: CLI surfaces OIDC-managed error clearly

- **WHEN** a logged-in user in an OIDC-managed org runs `assistant account change-password`
- **THEN** the command exits non-zero and prints `Your org's accounts are managed by <issuer>. Change your password there instead.`

#### Scenario: Not-logged-in shows actionable message

- **WHEN** a user with no stored credentials and no `--api-key` runs `assistant account show`
- **THEN** the command exits non-zero and prints `Not logged in — run \`assistant login <server-url>\` first.`
