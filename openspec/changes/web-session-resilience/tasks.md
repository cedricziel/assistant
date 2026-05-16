## 1. Failing tests first (TDD red)

- [x] 1.1 Add `app/test/unit/contexts/credentials_corrupted_test.dart`: with a `_ThrowingSecureStorage` that always throws on `read`, assert `loadContexts()` returns the persisted contexts with `credentialsCorrupted == true`. Round-trip another test where reads succeed → flag stays false.
- [x] 1.2 Add `app/test/widget/router/session_ended_redirect_test.dart`: override `activeContextProvider` to return a context with `credentialsCorrupted = true`; assert the router redirects to `/login?reason=session-ended` even though the context is non-null.
- [x] 1.3 Add `app/test/widget/login/session_ended_banner_test.dart`: pump `LoginScreen` with route state `?reason=session-ended` → banner visible with the expected text. Pump again with no query param → banner absent. Tap dismiss → banner hidden.
- [x] 1.4 Add `app/test/unit/spaces/space_selection_persistence_test.dart`: only runs `if (kIsWeb)` (use `@TestOn('browser')`). Set selection → simulate reload by disposing and rebuilding the container → selection rehydrates. Skip on VM.
- [x] 1.5 Add `crates/web-ui/build.rs`-adjacent unit test (or integration via a synthetic temp `sw.js`): given a fixture with `__APP_VERSION__`, the build step replaces it. Given a fixture without the placeholder, the build step returns `Err(_)`. Place under `crates/web-ui/tests/` if `build.rs` logic is extracted into a callable helper.
- [x] 1.6 Run `cd app && flutter test` — confirm the new dart tests fail for the expected reasons. Run `cargo test -p assistant-web-ui` — confirm the new build-script test fails too.

## 2. Service-worker version injection (smallest, most independent)

- [x] 2.1 Audit `crates/web-ui/build.rs` to find the existing version-injection step. Identify why the current placeholder check fails (likely a regex/literal mismatch between what the build expects and what's in `app/web/sw.js`).
- [x] 2.2 Verify `app/web/sw.js` has the literal `__APP_VERSION__` token. Update the comment block to make the placeholder load-bearing and document that removing it breaks the build.
- [x] 2.3 Refactor the version-injection step into a pure helper (e.g. `fn inject_sw_version(content: &str, version: &str) -> Result<String>`) so it's unit-testable from `tests/`.
- [x] 2.4 Replace the `cargo:warning=...` branch with a `panic!`/`return Err(...)` that fails the build when the placeholder is missing. Error message: "sw.js is missing the `__APP_VERSION__` placeholder — see app/web/sw.js header comment".
- [x] 2.5 Confirm test 1.5 turns green. Run `cargo build -p assistant-cli`; verify the warning is gone and the embedded `sw.js` contains the actual version number (`grep "v0.1.146" target/debug/build/assistant-web-ui-*/out/...` — or however the embedded asset is exposed).

## 3. Detect & expose `credentialsCorrupted`

- [x] 3.1 Add `bool credentialsCorrupted` field to `AssistantContext` (default `false`). Annotate it `@JsonKey(includeFromJson: false, includeToJson: false)` (or whatever the project's serializer uses) so it is never persisted. Update `copyWith` and `fromJson` accordingly. Update equality if applicable.
- [x] 3.2 In `ContextRepository.loadContexts()`, inside the existing per-context `try/catch`, change the catch block from `result.add(ctx);` to `result.add(ctx.copyWith(credentialsCorrupted: true));`.
- [x] 3.3 Add `final hasUsableActiveContextProvider = Provider<bool>((ref) { … })` in `app/lib/features/contexts/providers/context_providers.dart` next to the existing `hasActiveContextProvider`.
- [x] 3.4 Confirm test 1.1 turns green.

## 4. Router redirect on corruption

- [x] 4.1 Update the router redirect in `app/lib/router/app_router.dart`:
  - Import `hasUsableActiveContextProvider`.
  - Replace the `hasContext` check used for the redirect-to-login decision with `hasUsableContext = ref.read(hasUsableActiveContextProvider)`.
  - When the active context is corrupted, redirect to `/login?reason=session-ended` (preserve any other query params if present).
  - Keep `hasContext` (the original) for the "redirect away from login when authenticated" rule — corrupted contexts should not trip the authenticated-already path.
- [x] 4.2 Confirm test 1.2 turns green.

## 5. Login banner

- [x] 5.1 In `app/lib/features/login/login_screen.dart`, read `GoRouterState.uri.queryParameters['reason']` from the screen's build context (use `GoRouter.of(context).routerDelegate...` or pass via constructor — check what the screen already takes).
- [x] 5.2 Render a `MaterialBanner` (or inlined `Card` with `colorScheme.errorContainer`) above the login form when `reason == 'session-ended'`. Text: "Your session was reset by the browser. Please log in again." Dismiss button hides it (local state, no provider needed).
- [x] 5.3 Confirm test 1.3 turns green.

## 6. Persist `spaceSelectionProvider` on web

- [x] 6.1 In `app/lib/features/spaces/space_provider.dart`, extend `SpaceSelectionNotifier`:
  - On `build()`, if `kIsWeb`, attempt to read `localStorage['assistant.spaceSelection']`; if present and parses as JSON, return the hydrated `SpaceSelection`.
  - Wrap every `state = ...` mutation with a `_persistOnWeb()` call that writes the current state JSON to `localStorage`. Wrap in try/catch — write failures are non-fatal.
- [x] 6.2 Add `_clearOnWeb()` and call it from the new `clear()` method (the existing one already exists for the logout path).
- [x] 6.3 In `app/lib/shared/auth_actions.dart` `performWebLogout`, ensure the localStorage key is removed (the `clear()` notifier call should already do this via 6.2 — verify and add explicit removal if needed).
- [x] 6.4 Confirm test 1.4 turns green. Run `cd app && flutter run -d chrome` manually: select a space, hard-refresh, confirm selection persists.

## 7. Smoke + ship

- [x] 7.1 `cd app && flutter analyze --fatal-infos` → 0 issues.
- [x] 7.2 `cd app && flutter test` → all green (target: ≥ 786 passing, +4 new from 1.1–1.4).
- [x] 7.3 `cargo test -p assistant-web-ui` → all green (target: ≥ 232 passing, +1 new from 1.5).
- [x] 7.4 `make lint && make format && make precommit` — clean.
- [x] 7.5 `make build` — verify the new SW version injection works and no `cargo:warning=sw.js …` line appears.
- [x] 7.6 Open PR titled `fix(app): web session resilience — corruption detection, banner, persistence, SW versioning`. Body links the four scenarios.
- [x] 7.7 Merge after CI green.
- [ ] 7.8 Deploy to schorschvm. Have the operator: clear browser cache once → log in → hard-refresh → confirm space selection persists. Then simulate decrypt failure (manually delete the `IndexedDB` entry for `flutter_secure_storage` via DevTools) → reload → confirm landing on `/login?reason=session-ended` with the banner visible. Then re-login → confirm context is reused (same context id), credentials refreshed, selection re-discovered.
- [ ] 7.9 Archive: `openspec archive web-session-resilience`.
