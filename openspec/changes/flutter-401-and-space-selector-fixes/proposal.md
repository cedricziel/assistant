## Why

The Flutter web app silently fails when a session goes stale or when the space selector is opened in transient states. Concretely: 401 responses leave the user stranded on `/chat` with broken panels because there is no Dio interceptor that maps "auth gone" to "log out and redirect". And `SpaceSelectorScreen` shows misleading empty/loading states during the post-login race and gets stuck on an infinite spinner when revisited after a single-space org has already been auto-selected. Both turn into "the app looks broken" reports — even though the backend is healthy.

## What Changes

- Add a global Dio interceptor in `ApiClient` (`app/lib/api/api_client.dart`) that: (a) attempts a single OAuth refresh on the first 401, retries the request, and (b) on persistent 401 deactivates the active context so the router redirects to `/login`.
- In `OrgsNotifier` / `SpacesNotifier` (`app/lib/features/spaces/space_provider.dart`), distinguish "API client not yet available" from "API returned empty". When `apiClientProvider` is null, surface `AsyncLoading`, not an empty `data: []`.
- In `_SpaceList` (`app/lib/features/spaces/space_selector_screen.dart`), fix the stuck-spinner branch: when revisiting with `spaceId` already set, render the list of spaces (or unconditionally bounce to `/chat` for one-space orgs) instead of an unconditional spinner with a guarded callback that no-ops.
- In the logout handler (`app/lib/shared/nav_shell.dart`), also reset `spaceSelectionProvider` so a new sign-in starts from a clean selection.
- Cover all four behaviors with widget/unit tests under `app/test/`.

## Capabilities

### New Capabilities

- `web-401-recovery`: How the Flutter app handles 401 responses from the assistant API — refresh-once-then-deactivate, with redirect-to-login as the terminal state.
- `space-selector-resilience`: Required behavior of the space selector against transient API states (loading), revisit with prior selection, and reset on logout.

### Modified Capabilities

(none — `web-login` and `router-loading-scaffold` keep their existing requirements; the new capabilities sit alongside them.)

## Impact

- **Code touched**: `app/lib/api/api_client.dart`, `app/lib/features/spaces/space_provider.dart`, `app/lib/features/spaces/space_selector_screen.dart`, `app/lib/shared/nav_shell.dart`, `app/lib/features/auth/auth_provider.dart` (reuse `refreshIfNeeded`).
- **Tests**: new widget tests for the selector states and a unit test for the interceptor's refresh+retry+deactivate flow.
- **APIs**: no backend changes — the Rust web-ui already returns 401 correctly; this is a pure client behavior fix.
- **Dependencies**: none added.
- **Breaking changes**: none.
- **Non-goals**:
  - Implementing token rotation or proactive refresh based on `expires_at`. Refresh is reactive (on 401 only).
  - Reworking the OAuth login UI or PKCE flow.
  - Changing the backend `/oauth/token` or membership endpoints.
  - A multi-space picker UX overhaul — only the broken auto-select / revisit paths are in scope.
- **User-facing documentation needed**: No. These are bug fixes that restore expected behavior; no new user-visible features or workflows.
