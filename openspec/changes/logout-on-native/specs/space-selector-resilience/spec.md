## MODIFIED Requirements

### Requirement: Logout resets in-memory space selection

The logout handler (`app/lib/shared/nav_shell.dart`) SHALL reset `spaceSelectionProvider` to its initial empty state before deactivating the active context, on every platform (web, macOS, iOS). The interceptor-driven deactivation path (see `web-401-recovery`) SHALL also clear the selection. Both paths route through the shared `performLogout` helper.

#### Scenario: User clicks logout

- **WHEN** the user taps the logout button in the nav shell
- **THEN** the handler SHALL call `performLogout(container)` (which calls `spaceSelectionProvider.notifier.clear()` AND THEN `activeContextProvider.notifier.deactivate()`) AND THEN navigate to `/login`

#### Scenario: 401 interceptor deactivates the session

- **WHEN** the 401 interceptor's refresh attempt fails AND it invokes the auth-expired callback
- **THEN** the same `performLogout` shape SHALL run — clear selection then deactivate

#### Scenario: Next login starts with empty selection

- **WHEN** the user re-authenticates after a logout (manual or interceptor-driven)
- **THEN** `spaceSelectionProvider` SHALL be in its initial state with `orgId == null` AND `spaceId == null` AND the auto-select flow on `/spaces` SHALL run from scratch
