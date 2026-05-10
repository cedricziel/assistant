## 1. Code edits

- [x] 1.1 In `app/lib/shared/nav_shell.dart`, remove the `if (kIsWeb)` gate around `_LogoutButton`. The button now renders unconditionally.
- [x] 1.2 In `app/lib/shared/auth_actions.dart`, rename `performWebLogout` → `performLogout`. Update the dartdoc to drop the "on the web" framing.
- [x] 1.3 In `app/lib/features/connection/connection_provider.dart`, update the call from `_handleAuthExpired` (the 401 interceptor's callback wiring) to use `performLogout`.
- [x] 1.4 In `app/lib/shared/nav_shell.dart`, update the logout handler's call site to use `performLogout`.
- [x] 1.5 Rename `app/test/unit/spaces/logout_resets_space_selection_test.dart`'s test description and call sites to reference `performLogout` (file path stays the same).

## 2. Smoke + ship

- [x] 2.1 `flutter analyze --fatal-infos` → 0 issues.
- [x] 2.2 `flutter test` → all green (existing tests; no new tests added).
- [x] 2.3 `flutter build macos` succeeds (sanity that native still compiles).
- [x] 2.4 Manual smoke: in the running mac app, confirm the logout button is now visible in the nav shell. Tap it → land on `/login`.
- [ ] 2.5 PR: `feat(app): expose logout on native (rename performWebLogout → performLogout)`. Body links the four scenarios.
- [ ] 2.6 Merge.
- [ ] 2.7 Archive: `openspec archive logout-on-native`.
