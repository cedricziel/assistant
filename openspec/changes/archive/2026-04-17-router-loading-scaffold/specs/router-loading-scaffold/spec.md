## ADDED Requirements

### Requirement: App shows loading scaffold while active context is resolving

While `activeContextProvider` is in `AsyncLoading` state the app SHALL display a full-screen loading scaffold and SHALL NOT render any navigation chrome (rail, tab bar, top bar) or attempt to access protected routes.

#### Scenario: Cold start with existing context

- **WHEN** the app launches and `activeContextProvider` is `AsyncLoading`
- **THEN** the router navigates to `/loading` and the loading scaffold is visible

#### Scenario: Provider settles with an active context

- **WHEN** `activeContextProvider` transitions from `AsyncLoading` to `AsyncData` with a non-null context
- **THEN** the router redirects the user to `/chat` automatically without any user interaction

#### Scenario: Provider settles with no context

- **WHEN** `activeContextProvider` transitions from `AsyncLoading` to `AsyncData` with a null value
- **THEN** the router redirects the user to `/contexts` automatically

#### Scenario: Loading scaffold is excluded from navigation chrome

- **WHEN** the loading scaffold is displayed
- **THEN** the icon rail, bottom tab bar, and top app bar are NOT rendered

### Requirement: Loading route is exempt from redirect guards

The `/loading` route SHALL be exempt from all go_router redirect guards so that navigating to it while loading does not produce a redirect loop.

#### Scenario: Redirect does not fire for /loading while loading

- **WHEN** the active context is still loading AND the current route is `/loading`
- **THEN** the redirect returns `null` (no further redirect)

### Requirement: Loading screen renders a spinner and label

The loading scaffold SHALL render a centred `CircularProgressIndicator` and a short status label.

#### Scenario: Loading screen visual content

- **WHEN** the `/loading` route is active
- **THEN** a `CircularProgressIndicator` widget is present in the widget tree
- **THEN** a text label reading "Starting…" is present in the widget tree
