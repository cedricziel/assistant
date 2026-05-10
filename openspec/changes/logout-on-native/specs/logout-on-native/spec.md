## ADDED Requirements

### Requirement: Logout affordance is available on every platform

The nav shell SHALL render the logout button on every platform (web, macOS, iOS), not just web. There MUST NOT be a `kIsWeb` (or equivalent platform) gate around the logout entry point.

#### Scenario: Native nav shell shows logout

- **WHEN** the app runs on macOS or iOS AND the user is authenticated
- **THEN** the nav shell SHALL render a logout button in the same position and shape as on web

#### Scenario: Native logout calls performLogout

- **WHEN** the user taps the logout button on macOS or iOS
- **THEN** the handler SHALL call `performLogout(container)` AND THEN navigate the router to `/login`

#### Scenario: Web logout — no behavior change

- **WHEN** the user taps the logout button on web
- **THEN** the same `performLogout` flow SHALL run as before this change (no regression)

### Requirement: The shared logout helper is named platform-agnostically

The function previously named `performWebLogout` SHALL be renamed `performLogout`. The implementation SHALL be unchanged: clear `spaceSelectionProvider`, then call `activeContextProvider.notifier.deactivate()`.

#### Scenario: Function rename — call sites updated

- **WHEN** any caller (nav shell logout handler, 401 interceptor's `_handleAuthExpired`) invokes the helper
- **THEN** the function SHALL be `performLogout(ProviderContainer)` — no caller still references `performWebLogout`

#### Scenario: Existing tests still pass

- **WHEN** `flutter test test/unit/spaces/logout_resets_space_selection_test.dart` runs
- **THEN** all assertions SHALL still hold against `performLogout` (the test file is updated to use the new name; semantics unchanged)
