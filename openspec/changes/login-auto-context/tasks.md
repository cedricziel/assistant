## 1. Router Changes

- [ ] 1.1 Add `login` route constant to `AppRoutes` in `app/lib/router/app_router.dart`
- [ ] 1.2 Add `GoRoute` for `/login` pointing to `LoginScreen`
- [ ] 1.3 Update router redirect logic: on web (`kIsWeb`), redirect unauthenticated users to `/login` instead of `/contexts`
- [ ] 1.4 Update router redirect logic: on web, block direct navigation to `/contexts` (redirect to `/login` or `/chat` depending on auth state)

## 2. Login Screen

- [ ] 2.1 Create `app/lib/features/login/login_screen.dart` with `LoginScreen` widget
- [ ] 2.2 Display server URL as read-only text derived from `Uri.base.origin`
- [ ] 2.3 Add password-style token input field (optional, may be left empty)
- [ ] 2.4 Add submit button that creates/updates a context named `Uri.base.host` with the entered token
- [ ] 2.5 On submit: upsert context (update existing if same `serverUrl` exists, else create new)
- [ ] 2.6 On submit: activate the context via `activeContextProvider.notifier.activate()`
- [ ] 2.7 On submit success: navigate to `/chat` via `context.go('/chat')`
- [ ] 2.8 Add loading indicator while saving

## 3. Nav Shell — Logout & Context Button Guards

- [ ] 3.1 In `app/lib/shared/nav_shell.dart`, wrap the `_ContextsButton` with `if (!kIsWeb)` guard
- [ ] 3.2 Add a `_LogoutButton` widget (icon: `Icons.logout`, tooltip: "Log out")
- [ ] 3.3 Show `_LogoutButton` in the nav rail trailing section only when `kIsWeb`
- [ ] 3.4 Logout handler: call `activeContextProvider.notifier.deactivate()` then navigate to `/login`

## 4. Context Repository — Upsert Support

- [ ] 4.1 In `app/lib/features/contexts/data/context_repository.dart`, add or verify an `upsertContextByUrl(AssistantContext)` method that updates token if a context with the same `serverUrl` already exists
- [ ] 4.2 Expose upsert via `ContextsNotifier.upsertContextByUrl()` in `context_providers.dart`

## 5. Tests

- [ ] 5.1 Widget test for `LoginScreen`: renders read-only URL, renders token field, submit button calls upsert and activates context
- [ ] 5.2 Router redirect test (unit): on web with no context → `/login`; on web with context → `/chat`; on non-web with no context → `/contexts`
- [ ] 5.3 Widget test for `NavShell` on web: logout button present, contexts button absent
- [ ] 5.4 Widget test for `NavShell` on non-web: contexts button present, logout button absent
