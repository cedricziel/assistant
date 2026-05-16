## MODIFIED Requirements

### Requirement: Flutter login screen uses OAuth2 PKCE

The Flutter `/login` screen SHALL display email and password input fields
only. The server URL SHALL be pre-filled from `Uri.base.origin` on web and
SHALL NOT be editable by the user. The legacy single-token input field
and the email-password vs token toggle SHALL be removed. On submission
the system SHALL run the OAuth2 authorization-code flow with PKCE against
`/oauth/authorize` and `/oauth/token` to obtain access and refresh
tokens; it SHALL NOT POST credentials to a server `/login` endpoint.

#### Scenario: Login screen renders with pre-filled URL and credential fields

- **WHEN** the user navigates to `/login` on the web platform
- **THEN** the screen SHALL display the server URL derived from `Uri.base.origin` as read-only text
- **THEN** the screen SHALL display a text field for `email` (`TextInputType.emailAddress`)
- **THEN** the screen SHALL display a password-style text field for `password`
- **THEN** the screen SHALL NOT display a token field or a credential-type toggle

#### Scenario: Empty fields rejected client-side

- **WHEN** the user submits the form with an empty email or password
- **THEN** the form SHALL display a validation error AND SHALL NOT issue a network request

#### Scenario: Successful login uses OAuth2 PKCE and stores credentials

- **WHEN** the user enters valid email + password and submits the login form
- **THEN** the system SHALL run the OAuth2 PKCE flow against `Uri.base.origin`
- **THEN** on success the system SHALL save (or update) an `AssistantContext`
  with `oauthCredentials` populated (access + refresh tokens)
- **THEN** the system SHALL activate that context
- **THEN** the router SHALL navigate to `/chat`

#### Scenario: Invalid credentials show inline error

- **WHEN** the OAuth flow returns an authentication failure
- **THEN** the system SHALL display a generic "Invalid email or password" error
- **THEN** the system SHALL NOT create or activate a context
- **THEN** the system SHALL keep the entered email and clear the password field

### Requirement: Setup screen uses email + password and OAuth2 PKCE

The `/setup` screen (`ConnectionScreen`) SHALL display a server URL field
together with email + password fields when running in remote mode. The
legacy single-token input field SHALL be removed. The `?_token=`
query-parameter handling and its auto-submit on first frame SHALL be
removed. The `?_url=` query-parameter SHALL continue to pre-fill the URL
field but SHALL NOT auto-submit the form. On submission the screen SHALL
run the OAuth2 PKCE flow against the entered URL.

#### Scenario: Setup renders remote-mode form

- **WHEN** the user navigates to `/setup` on web (or on macOS in remote mode)
- **THEN** the screen SHALL display a server URL field
- **THEN** the screen SHALL display an email field
- **THEN** the screen SHALL display a password-style text field
- **THEN** the screen SHALL NOT display a token field or credential-type toggle

#### Scenario: URL query parameter pre-fills URL only

- **WHEN** the user navigates to `/setup?_url=https://server.example.com`
- **THEN** the URL field SHALL be pre-filled with `https://server.example.com`
- **THEN** the form SHALL NOT auto-submit
- **THEN** the email and password fields SHALL be empty

#### Scenario: Token query parameter is ignored

- **WHEN** the user navigates to `/setup?_token=anything`
- **THEN** the system SHALL NOT pre-fill any field with that value
- **THEN** the form SHALL NOT auto-submit
- **THEN** no `AssistantContext` SHALL be created from the query parameter

#### Scenario: Setup submission creates a context via OAuth2 PKCE

- **WHEN** the user enters URL + valid email + password and submits
- **THEN** the system SHALL run the OAuth2 PKCE flow against `<url>`
- **THEN** on success the system SHALL save an `AssistantContext` with the
  URL and populated `oauthCredentials`
- **THEN** the system SHALL activate the context and navigate to `/chat`

#### Scenario: Embedded-server mode unaffected

- **WHEN** the user is on macOS with `EmbeddedServerService.isAvailable == true`
  AND selects "Embedded (local)" mode
- **THEN** the screen SHALL show the embedded-server startup progress
- **THEN** no credential fields SHALL be displayed (no auth needed for embedded)

### Requirement: Edit-context screen has no raw token input

The edit-context screen (`/contexts/{id}/edit`) SHALL NOT display a
free-form text field that writes credentials directly. Re-acquiring
credentials for an existing context SHALL go through the OAuth2 PKCE flow
targeted at the context's stored server URL.

#### Scenario: Edit-context renders without token field

- **WHEN** the user opens the edit screen for an existing context
- **THEN** the screen SHALL display name and URL fields as editable
- **THEN** the screen SHALL NOT display a free-form token input
- **THEN** the screen SHALL display a "Re-authenticate" button

#### Scenario: Re-authenticate runs OAuth2 PKCE

- **WHEN** the user taps the "Re-authenticate" button
- **THEN** the system SHALL run the OAuth2 PKCE flow against
  `context.serverUrl`
- **THEN** on success the system SHALL replace the context's
  `oauthCredentials` with the freshly issued tokens

### Requirement: AssistantContext stores only OAuth2 credentials

The `AssistantContext` model SHALL carry exactly one credential slot,
`oauthCredentials` (access + refresh tokens). The legacy `authToken`
field, the `AuthMode` enum, and the `effectiveToken` branch-based getter
SHALL be removed. A `bearerToken` getter SHALL return
`oauthCredentials.bearerToken` directly.

#### Scenario: Constructor requires OAuth credentials

- **WHEN** code instantiates an `AssistantContext` for an authenticated server
- **THEN** the constructor SHALL accept `oauthCredentials` (non-null) and
  SHALL NOT accept any `authToken` parameter

#### Scenario: bearerToken is a direct field access

- **WHEN** code reads `context.bearerToken`
- **THEN** the getter SHALL return `oauthCredentials.bearerToken` without
  any mode branching

#### Scenario: JSON serialisation omits legacy fields

- **WHEN** `toJson` is called on any `AssistantContext`
- **THEN** the resulting map SHALL NOT include `authMode` or `authToken` keys

### Requirement: Existing legacy-token contexts migrate on first load

`ContextRepository.loadContexts` SHALL detect persisted rows that were
written under the legacy model (`authMode == "legacyToken"`) and surface
them in a degraded state requiring re-authentication. The next write of
each such row SHALL strip the legacy keys.

#### Scenario: Legacy row produces a context needing re-auth

- **WHEN** the repository reads a stored JSON payload containing
  `authMode == "legacyToken"`
- **THEN** the resulting `AssistantContext` SHALL have
  `oauthCredentials == null` AND `requiresReauth == true`

#### Scenario: Re-auth flow upgrades a legacy context

- **WHEN** the user signs in again on a legacy context via the OAuth2 PKCE flow
- **THEN** the context's `oauthCredentials` SHALL be populated
- **THEN** the context's `requiresReauth` SHALL become false
- **THEN** the next `saveContext` SHALL persist a JSON payload with no
  `authMode` key and no `authToken` key

#### Scenario: Switcher shows re-auth affordance

- **WHEN** the contexts list / switcher renders a context with
  `requiresReauth == true`
- **THEN** the row SHALL display a "Sign in again" affordance
- **THEN** the row SHALL NOT allow direct connection without re-auth

### Requirement: Server `/login` page is removed

The server SHALL NOT expose a `/login` GET or POST endpoint. Requests to
the path `/login` SHALL be handled by the SPA catch-all so the Flutter
app renders its own client-side login screen.

#### Scenario: GET /login returns the SPA

- **WHEN** an unauthenticated client requests `GET /login`
- **THEN** the server SHALL respond with `200 OK` and the SPA `index.html`
  payload (same as any other catch-all path)
- **THEN** the response SHALL NOT contain a server-rendered HTML login form

#### Scenario: POST /login is unrouted

- **WHEN** a client submits `POST /login`
- **THEN** the server SHALL respond with `404 Not Found` (the route is
  not registered)

### Requirement: Server login endpoint honours OIDC mode

The server SHALL detect OIDC configuration and route authentication
traffic through the OIDC authorization-code flow. When `auth_mode =
"oidc"` is configured, `/oauth/authorize` SHALL redirect to the upstream
IdP for credential collection. The Flutter SPA's login flow is unchanged
in OIDC mode — it kicks off the same OAuth2 PKCE handshake, which the
server then forwards to the IdP.

#### Scenario: OIDC-mode authorize redirects to IdP

- **WHEN** the server is in OIDC mode AND a browser hits `/oauth/authorize`
- **THEN** the server SHALL respond with `302 Found` to the configured
  upstream IdP authorize URL, carrying the original `client_id`, `state`,
  and `code_challenge` parameters
