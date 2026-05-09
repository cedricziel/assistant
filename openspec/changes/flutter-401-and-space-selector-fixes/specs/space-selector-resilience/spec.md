## ADDED Requirements

### Requirement: Selector providers distinguish "API not ready" from "API returned empty"

`OrgsNotifier` and `SpacesNotifier` (`app/lib/features/spaces/space_provider.dart`) SHALL stay in `AsyncLoading` while the underlying API client is not yet available. They MUST NOT short-circuit to `AsyncData([])` solely because `apiClientProvider` is null.

#### Scenario: API client not yet available — providers loading

- **WHEN** `OrgsNotifier.build()` runs AND `apiClientProvider` returns `null` because `serverProfileProvider` has not yet settled
- **THEN** the provider SHALL await the connection (e.g., via `serverProfileProvider.future`) before issuing the network call AND the consumer SHALL observe `AsyncLoading`, not `AsyncData([])`

#### Scenario: Genuinely no profile — providers return empty

- **WHEN** `OrgsNotifier.build()` runs AND `serverProfileProvider` has settled with `profile == null` (no active context)
- **THEN** the provider SHALL return `AsyncData([])`

#### Scenario: API call returns empty list

- **WHEN** `OrgsNotifier.build()` runs AND the API call succeeds AND the server returns `[]`
- **THEN** the provider SHALL return `AsyncData([])` AND `_OrgList` SHALL render the "No organizations found" empty card

### Requirement: Single-space revisit does not show an infinite spinner

`_SpaceList` (`app/lib/features/spaces/space_selector_screen.dart`) MUST NOT render an unconditional `CircularProgressIndicator` when the user revisits the selector with a `spaceId` already set in `spaceSelectionProvider`. The list of spaces SHALL be rendered so the user can confirm or change the selection.

#### Scenario: First-time auto-select with one space

- **WHEN** the user reaches `/spaces` AND `spaceSelectionProvider.spaceId` is `null` AND the API returns exactly one space
- **THEN** the screen SHALL display a brief loading indicator AND the post-frame callback SHALL call `selectSpace(...)` AND navigate to `/chat`

#### Scenario: Revisit with one space and prior selection

- **WHEN** the user navigates to `/spaces` AND `spaceSelectionProvider.spaceId` is already set AND the API returns exactly one space
- **THEN** the screen SHALL render the list of spaces (a single tile) with the currently-selected space visually marked AND SHALL NOT show a loading spinner indefinitely AND SHALL NOT auto-navigate

#### Scenario: Multi-space revisit unaffected

- **WHEN** the user navigates to `/spaces` AND the API returns more than one space (regardless of prior selection)
- **THEN** the screen SHALL render the full list of spaces with the currently-selected one visually marked

### Requirement: Logout resets in-memory space selection

The web logout handler (`app/lib/shared/nav_shell.dart`) SHALL reset `spaceSelectionProvider` to its initial empty state before deactivating the active context. The interceptor-driven deactivation path (see `web-401-recovery`) SHALL also clear the selection.

#### Scenario: User clicks logout

- **WHEN** the user taps the logout button in the web nav shell
- **THEN** the handler SHALL call `spaceSelectionProvider.notifier.clear()` AND THEN `activeContextProvider.notifier.deactivate()` AND THEN navigate to `/login`

#### Scenario: 401 interceptor deactivates the session

- **WHEN** the 401 interceptor's refresh attempt fails AND it calls `deactivate()`
- **THEN** the same code path SHALL also clear `spaceSelectionProvider`

#### Scenario: Next login starts with empty selection

- **WHEN** the user re-authenticates after a logout (manual or interceptor-driven)
- **THEN** `spaceSelectionProvider` SHALL be in its initial state with `orgId == null` AND `spaceId == null` AND the auto-select flow on `/spaces` SHALL run from scratch
