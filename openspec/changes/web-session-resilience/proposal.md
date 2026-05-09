## Why

After #685 landed, the dominant remaining failure on web is no longer "stuck on a broken /chat" — the 401 interceptor recovers. But the _cause_ is web-specific and silent: `flutter_secure_storage` on web stores OAuth credentials as AES-GCM ciphertext in `localStorage` with the key material in `IndexedDB`, and that key gets wiped/corrupted by routine browser actions (clear site data, profile sync glitches, browser updates, private mode tabs closing). When decrypt fails, `context_repository.dart:126` silently drops the token but keeps the context metadata, leaving the router with `hasContext == true` and an empty bearer — every request 401s. The user is then bounced to /login by the interceptor with no explanation. Native platforms don't see this because they go through the OS Keychain, which is durable. Two compounding issues make it worse on web: `spaceSelectionProvider` is in-memory only (lost on every hard reload), and `app/web/sw.js` ships without a version bump (`__APP_VERSION__` placeholder is silently skipped at build time), so old buggy JS keeps serving for up to 24 hours after a deploy.

## What Changes

- `context_repository.loadContexts()` MUST distinguish "secure-storage read failed" from "no token" and mark the context as session-ended. A new public boolean (`hasCredentialsCorrupted`) is exposed for the router and login screen to consume.
- The router (`app_router.dart`) MUST treat an active context with corrupted credentials as `!hasContext` for redirect purposes — same effect as `deactivate()`, but without losing the context metadata so re-login can update it in place.
- The login screen MUST surface a `SessionEndedBanner` when arriving with the corruption flag set: "Your session was reset by the browser; please log in again."
- `spaceSelectionProvider` MUST persist `(orgId, spaceId)` to `localStorage` on web only (key: `assistant.spaceSelection`). Restored on `Notifier.build()`. Native platforms keep the in-memory behavior.
- `crates/web-ui/build.rs` MUST inject the package version into `app/build/web/sw.js` at the `__APP_VERSION__` placeholder before embedding. The warning is currently swallowed; this change makes it loud (build error if placeholder is missing) so the SW version actually bumps on release.

## Capabilities

### New Capabilities

- `web-session-resilience`: How the Flutter web app detects and recovers from `flutter_secure_storage` decrypt failures, persists the active space selection, and ensures service-worker cache busts on every release.

### Modified Capabilities

(none — `web-401-recovery` and `space-selector-resilience` from #685 stay; this change adds a new layer in front of them.)

## Impact

- **Code touched**: `app/lib/features/contexts/data/context_repository.dart`, `app/lib/features/contexts/providers/context_providers.dart`, `app/lib/router/app_router.dart`, `app/lib/features/login/login_screen.dart`, `app/lib/features/spaces/space_provider.dart`, `crates/web-ui/build.rs`, `app/web/sw.js`.
- **Tests**: new unit tests for the corruption detection in `context_repository`, widget test for the login banner, unit test for `spaceSelectionProvider` persistence round-trip on web.
- **Behavior change**: web users with broken secure storage now land on `/login` with a banner instead of staring at a broken `/chat` or being silently logged out. Selection persists across refresh on web.
- **Non-goals**:
  - Replacing the OAuth bearer flow with cookie auth on web. The HttpOnly `assistant_session` cookie already exists; switching the SPA to use `withCredentials: true` would eliminate the secure-storage class of bug entirely. Tracked as `web-cookie-auth` for a follow-up — bigger change with CORS implications.
  - Migrating off `flutter_secure_storage` for non-OAuth credentials.
  - Cross-tab synchronization of session state via `BroadcastChannel`.
  - Any backend changes (the OAuth refresh endpoint and the cookie are already in place).
- **User-facing documentation needed**: No. The behavior change is fail-friendly, not a new feature.
