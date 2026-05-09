## 1. Failing tests first (TDD red)

- [x] 1.1 Add `app/test/unit/api/auth_interceptor_test.dart` covering: first 401 → refresh + retry succeeds; second 401 (already-retried) → deactivate; refresh-throws → deactivate; concurrent 401s share a single refresh future. Use Dio's `MockHttpAdapter` (or `package:nock`-style stubs) and inject mock `onAuthExpired` / `onTokenRefreshed` callbacks.
- [x] 1.2 Add `app/test/widget/space_selector_loading_state_test.dart`: when `apiClientProvider` is overridden to `null` and `serverProfileProvider` is in `AsyncLoading`, `_OrgList` renders a `CircularProgressIndicator` (NOT `_EmptyCard`).
- [x] 1.3 Add `app/test/widget/space_selector_revisit_test.dart`: with `spaceSelectionProvider` pre-seeded to `(orgId: o, spaceId: s)` and the spaces API returning a single space matching `s`, the screen renders the space tile (visually marked) and does NOT show an indefinite spinner. A separate test asserts the cold-cache one-space flow still auto-navigates to `/chat`.
- [x] 1.4 Add `app/test/unit/spaces/logout_resets_space_selection_test.dart`: assert `performWebLogout` clears `spaceSelectionProvider` AND deactivates `activeContextProvider`.
- [x] 1.5 Run `flutter test` — confirmed all four new tests fail for the expected reasons (red bar).

## 2. Implement the Dio 401 interceptor

- [x] 2.1 Defined `AuthRecoveryInterceptor` in new file `app/lib/api/auth_interceptor.dart` taking `refreshTokens: Future<String?> Function()` (returns new bearer or null) and `onAuthExpired: Future<void> Function()` callbacks.
- [x] 2.2 Single-flight via `Completer<String?>? _inFlight` — concurrent 401s share the same in-flight refresh future. Tested by `concurrent 401s share a single refresh attempt (single-flight)`.
- [x] 2.3 `onError`: skip non-401; on retried-true 401 → `onAuthExpired` + `handler.next`; else `_refreshOnce`; on null token → `onAuthExpired` + `handler.next`; on success → re-dispatch via the same Dio with new bearer + `extra['x-retried'] = true`. The retry runs through the interceptor chain again so a second 401 is handled by the inner invocation; the outer catch just propagates without double-firing `onAuthExpired`.
- [x] 2.4 `ApiClient` constructor extended with optional `refreshTokens` / `onAuthExpired` callbacks; registers the interceptor on `_dio.interceptors` when both are provided.
- [x] 2.5 `apiClientProvider` (`connection_provider.dart`) now constructs the closures: `_refreshAccessToken` calls `OAuthService.refresh`, persists via `contextsProvider.notifier.saveContext`, returns the new bearer; `_handleAuthExpired` clears `spaceSelectionProvider` then deactivates `activeContextProvider`.
- [x] 2.6 6 interceptor tests green; `flutter analyze --fatal-infos` clean.

## 3. Selector loading-vs-empty distinction

- [x] 3.1 `OrgsNotifier.build()` and `SpacesNotifier.build()` now `await ref.watch(serverProfileProvider.future)` first; only after the connection settles do they decide between `[]` (no profile) and the API call.
- [x] 3.2 `OrgList shows a spinner ... while serverProfileProvider is loading` and `OrgList renders empty card when serverProfileProvider settles with no profile` both green.

## 4. Selector revisit fix

- [x] 4.1 `_SpaceList` now only enters the spinner-with-postframe branch when `selection.spaceId == null`; revisits with `spaceId` already set fall through to the list-rendering branch.
- [x] 4.2 List-rendering Column highlights the active space with `colorScheme.primaryContainer` background and a leading check icon.
- [x] 4.3 Both revisit tests green (`first-time auto-select on single-space org navigates to /chat`, `revisit with single space + existing selection renders the list, not a stuck spinner`).

## 5. Logout resets selection

- [x] 5.1 Extracted `performWebLogout(ProviderContainer)` in new file `app/lib/shared/auth_actions.dart`. `_LogoutButton.onLogout` in `nav_shell.dart` now calls it via `ProviderScope.containerOf(context, listen: false)` then navigates to `/login`.
- [x] 5.2 The interceptor's `_handleAuthExpired` (in `connection_provider.dart`) mirrors `performWebLogout`: clear selection then deactivate. Both paths produce the same end state.
- [x] 5.3 Both logout tests green (`performWebLogout clears space selection AND deactivates context`, `performWebLogout is idempotent on already-empty state`).

## 6. Manual smoke + regenerate web bundle

- [x] 6.1 `flutter analyze --fatal-infos` → 0 issues.
- [x] 6.2 `flutter test` → 782 tests all green.
- [ ] 6.3 `flutter run -d chrome` against a local `assistant webui serve`. Walk through: fresh login → land on `/chat`, no flicker; click space switcher → see the single space, no spinner; click logout → land on `/login`; revoke the JWT in DB, hit a refresh on `/chat` → 401 handled, redirected to `/login` after one refresh attempt (or instantly if no refresh token). **(Manual — operator action)**
- [x] 6.4 `flutter build web --release` succeeds (38.3s); the embed-into-Rust step (`cargo build -p assistant-cli` via `make build`) is left for release time.

## 7. Ship

- [ ] 7.1 Open PR with title `fix(app): handle 401s and space-selector race conditions`. Body links the four scenarios from the spec deltas.
- [ ] 7.2 Merge and cut a release.
- [ ] 7.3 Deploy to schorschvm; have the operator clear browser storage once and verify the four behaviors on the live host.
- [ ] 7.4 Archive this change with `openspec archive flutter-401-and-space-selector-fixes` after deploy verification.
