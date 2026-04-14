## ADDED Requirements

### Requirement: Web login screen shown when unauthenticated

On the web platform, when no active context exists, the system SHALL redirect to `/login` instead of `/contexts`.

#### Scenario: Unauthenticated web user lands on login

- **WHEN** the web app loads with no active context in localStorage
- **THEN** the router SHALL redirect to `/login`

#### Scenario: Authenticated web user bypasses login

- **WHEN** the web app loads with a persisted active context in localStorage
- **THEN** the router SHALL redirect to `/chat` without visiting `/login`

#### Scenario: Non-web platform unaffected

- **WHEN** the app runs on macOS or mobile
- **THEN** the router SHALL redirect unauthenticated users to `/contexts` (existing behavior)

### Requirement: Token-only login form with auto-filled server URL

The `/login` screen SHALL display a single token input field. The server URL SHALL be pre-filled from `Uri.base.origin` and SHALL NOT be editable by the user on web.

#### Scenario: Login screen renders with pre-filled URL

- **WHEN** the user navigates to `/login` on the web platform
- **THEN** the screen SHALL display the server URL derived from `Uri.base.origin` as read-only text
- **THEN** the screen SHALL display a password-style text field for the auth token

#### Scenario: Token field accepts empty value

- **WHEN** the user submits the login form with an empty token field
- **THEN** the system SHALL create a context with `authToken: null` and activate it

### Requirement: Login creates and activates a local context automatically

On form submission the system SHALL create (or update) a context with `name = Uri.base.host`, `serverUrl = Uri.base.origin`, and the entered token, then activate it without user confirmation.

#### Scenario: First-time login creates a new context

- **WHEN** the user enters a token and submits the login form
- **THEN** the system SHALL save a context named after `Uri.base.host`
- **THEN** the system SHALL activate that context
- **THEN** the router SHALL navigate to `/chat`

#### Scenario: Repeated login updates existing local context

- **WHEN** a context with the same `serverUrl` already exists and the user submits the login form
- **THEN** the system SHALL update the existing context's token rather than creating a duplicate

### Requirement: Active context persists across hard refresh on web

Because `shared_preferences` uses `localStorage` on web, the active context SHALL survive a browser hard refresh without re-prompting the user.

#### Scenario: Hard refresh with active context

- **WHEN** the user refreshes the browser while an active context is stored in localStorage
- **THEN** the app SHALL restore the active context automatically
- **THEN** the user SHALL land on `/chat` without seeing `/login`

### Requirement: Logout button in nav rail on web

On the web platform, the nav rail trailing section SHALL display a logout icon button. Pressing it SHALL deactivate the current context and navigate to `/login`.

#### Scenario: Logout from nav rail

- **WHEN** the user taps the logout button in the nav rail on web
- **THEN** the system SHALL call `activeContextProvider.notifier.deactivate()`
- **THEN** the router SHALL navigate to `/login`

#### Scenario: Logout button not shown on native

- **WHEN** the app runs on macOS or mobile
- **THEN** the nav rail trailing section SHALL NOT show a logout button (existing context switcher button remains)
