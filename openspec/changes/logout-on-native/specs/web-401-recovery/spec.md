## MODIFIED Requirements

### Requirement: Router redirects to login after deactivation

The router (`app_router.dart`) SHALL redirect to `/login` whenever the active context becomes null while the user is on a protected route. This requirement is satisfied by the existing redirect rule at `app_router.dart:128-130`; this spec ratifies that the interceptor's `deactivate()` call (via `performLogout`) MUST be the trigger for that redirect when 401 recovery fails.

#### Scenario: Interceptor deactivation triggers login redirect

- **WHEN** the interceptor's auth-expired path runs `performLogout` (which calls `activeContextProvider.notifier.deactivate()`) while the user is on `/chat` or any other authenticated route
- **THEN** the next router redirect evaluation SHALL produce `/login` AND the user SHALL be navigated there without manual interaction
