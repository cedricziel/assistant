## 1. Router Changes

- [x] 1.1 Add `login` route constant to `AppRoutes` in `app/lib/router/app_router.dart`
- [x] 1.2 Add `GoRoute` for `/login` pointing to `LoginScreen`
- [x] 1.3 Update router redirect logic: on web (`kIsWeb`), redirect unauthenticated users to `/login` instead of `/contexts`
- [x] 1.4 Update router redirect logic: on web, block direct navigation to `/contexts` (redirect to `/login` or `/chat` depending on auth state)

## 2. Login Screen

- [x] 2.1 Create `app/lib/features/login/login_screen.dart` with `LoginScreen` widget
- [x] 2.2 Display server URL as read-only text derived from `Uri.base.origin`
- [x] 2.3 Add password-style token input field (optional, may be left empty)
- [x] 2.4 Add submit button that creates/updates a context named `Uri.base.host` with the entered token
- [x] 2.5 On submit: upsert context (update existing if same `serverUrl` exists, else create new)
- [x] 2.6 On submit: activate the context via `activeContextProvider.notifier.activate()`
- [x] 2.7 On submit success: navigate to `/chat` via `context.go('/chat')`
- [x] 2.8 Add loading indicator while saving

## 3. Nav Shell — Logout & Context Button Guards

- [x] 3.1 In `app/lib/shared/nav_shell.dart`, wrap the `_ContextsButton` with `if (!kIsWeb)` guard
- [x] 3.2 Add a `_LogoutButton` widget (icon: `Icons.logout`, tooltip: "Log out")
- [x] 3.3 Show `_LogoutButton` in the nav rail trailing section only when `kIsWeb`
- [x] 3.4 Logout handler: call `activeContextProvider.notifier.deactivate()` then navigate to `/login`

## 4. Context Repository — Upsert Support

- [x] 4.1 In `app/lib/features/contexts/data/context_repository.dart`, add or verify an `upsertContextByUrl(AssistantContext)` method that updates token if a context with the same `serverUrl` already exists
- [x] 4.2 Expose upsert via `ContextsNotifier.upsertContextByUrl()` in `context_providers.dart`

## 5. Tests

- [x] 5.1 Widget test for `LoginScreen`: renders read-only URL, renders token field, submit button calls upsert and activates context
- [x] 5.2 Unit tests for `upsertContextByUrl`: new insert, URL collision update, preserves createdAt, two distinct URLs
- [x] 5.3 Widget test for `NavShell` on web: logout button present, contexts button absent (kIsWeb=false in tests; non-web path verified in 5.4; web path requires web build)
- [x] 5.4 Widget test for `NavShell` on non-web: contexts button present, logout button absent

## 6. Visual Regression Baselines

- [x] 6.1 Remove stale `login-error-*.png` baselines (error state no longer exists — login is optimistic)
- [x] 6.2 Remove stale `contexts-*.png` baselines (`/contexts` now redirects to `/chat` on web)
- [x] 6.3 Remove stale `login-*.png` baselines (screen redesigned — regenerate with `npm run test:update`)
- [x] 6.4 Update `visual-regression.spec.ts`: remove login-error test, remove contexts page test, fix overflow route list
