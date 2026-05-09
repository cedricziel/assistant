## ADDED Requirements

### Requirement: Decrypt failure during context load is detected and surfaced

`ContextRepository.loadContexts()` SHALL distinguish "secure-storage read threw" from "no token stored". A new boolean field `AssistantContext.credentialsCorrupted` MUST be set to `true` whenever any secure-storage read for that context throws an exception. The flag is in-memory only (never serialized).

#### Scenario: Healthy load — flag stays false

- **WHEN** `loadContexts()` runs AND every secure-storage read succeeds
- **THEN** every returned context SHALL have `credentialsCorrupted == false`

#### Scenario: Decrypt failure — flag flipped, metadata preserved

- **WHEN** `loadContexts()` reads the OAuth credentials for a context AND the read throws
- **THEN** the returned context SHALL have `credentialsCorrupted == true` AND `oauthCredentials == null` AND `authToken == null` AND all metadata fields (`id`, `name`, `serverUrl`, `authMode`, `createdAt`) SHALL match what was persisted in `SharedPreferences`

#### Scenario: Token decrypt OK but OAuth decrypt fails

- **WHEN** the legacy `authToken` reads successfully AND the OAuth credentials read throws
- **THEN** the context SHALL still be flagged `credentialsCorrupted == true` (any partial credential corruption taints the whole session)

### Requirement: Router treats corrupted contexts as unauthenticated

The router (`app_router.dart`) SHALL evaluate redirects against `hasUsableActiveContextProvider`, which returns `true` only when an active context is present AND `credentialsCorrupted == false`. A corrupted active context MUST trigger the same redirect as a missing one — but the context metadata SHALL NOT be deleted; re-login uses `upsertContextByUrl` to refresh credentials in-place.

#### Scenario: Cold start with corrupted credentials → /login

- **WHEN** the app boots AND `activeContextProvider` resolves to a context with `credentialsCorrupted == true`
- **THEN** the router SHALL redirect to `/login?reason=session-ended` AND SHALL NOT call `deactivate()` on the active context

#### Scenario: Re-login refreshes credentials in place

- **WHEN** the user re-authenticates from the session-ended login screen
- **THEN** `upsertContextByUrl` SHALL find the existing context (by `serverUrl`) AND update its credentials AND clear `credentialsCorrupted` AND preserve `id`, `createdAt`, and `name`

### Requirement: Login screen surfaces the session-ended banner

When the login route is opened with the `reason=session-ended` query parameter, the screen SHALL render a dismissible banner with a clear, human-readable explanation. Without that parameter the banner SHALL NOT render.

#### Scenario: Reason param present — banner shown

- **WHEN** the user navigates to `/login?reason=session-ended`
- **THEN** a banner SHALL render at the top of the login screen with a message like "Your session was reset by the browser. Please log in again." AND a dismiss button

#### Scenario: Direct /login navigation — no banner

- **WHEN** the user navigates to `/login` (no query parameters)
- **THEN** no banner SHALL render

#### Scenario: User dismisses the banner

- **WHEN** the user taps dismiss
- **THEN** the banner SHALL hide AND SHALL NOT reappear during the same login attempt (until the next decrypt failure)

### Requirement: Space selection persists across hard reload on web

On the web platform, `spaceSelectionProvider` SHALL persist its state to `localStorage` under key `assistant.spaceSelection` and rehydrate from that key on `Notifier.build()`. Native platforms SHALL retain the existing in-memory-only behavior.

#### Scenario: Web — selection persists across reload

- **GIVEN** the user is on web AND has selected `(orgId, spaceId)` AND triggers a hard reload
- **WHEN** the app rebuilds
- **THEN** `spaceSelectionProvider` SHALL hydrate to the same `(orgId, spaceId)` AND no auto-select flow SHALL run

#### Scenario: Native — no persistence change

- **WHEN** running on macOS or iOS
- **THEN** `spaceSelectionProvider` SHALL behave exactly as before this change (in-memory only)

#### Scenario: Logout clears persisted selection

- **WHEN** `performWebLogout` runs
- **THEN** the `assistant.spaceSelection` localStorage key SHALL be removed AND `spaceSelectionProvider` SHALL be reset to its initial empty state

#### Scenario: localStorage write failure is non-fatal

- **WHEN** the browser rejects a localStorage write (quota, private mode)
- **THEN** the provider SHALL continue with in-memory state AND SHALL NOT throw

### Requirement: Service-worker version is injected on every release build

`crates/web-ui/build.rs` SHALL replace the `__APP_VERSION__` placeholder in `app/build/web/sw.js` with `CARGO_PKG_VERSION` before `rust-embed` snapshots the asset. If the placeholder is missing from `sw.js`, the build SHALL fail with a clear error pointing to the load-bearing comment.

#### Scenario: Successful injection

- **GIVEN** `sw.js` contains the literal `__APP_VERSION__` token
- **WHEN** `cargo build -p assistant-cli` runs
- **THEN** the embedded `sw.js` SHALL have `__APP_VERSION__` replaced by the package version (e.g. `0.1.146`) AND no warning SHALL be printed

#### Scenario: Missing placeholder fails the build

- **WHEN** `sw.js` does not contain `__APP_VERSION__`
- **THEN** `cargo build -p assistant-cli` SHALL fail with a non-zero exit code AND the error message SHALL reference the placeholder and the contract documented in `app/web/sw.js`

#### Scenario: Service worker re-installs on version change

- **WHEN** a new release embeds an updated `sw.js` (different version comment)
- **THEN** browsers performing the byte-diff check SHALL detect the change AND queue the new SW for installation on next page load
