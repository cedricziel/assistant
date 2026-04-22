## Why

The iOS share extension always shows "Not Connected" even when the user is logged in and has an active context with valid credentials. The extension cannot access the shared Keychain credentials due to three compounding bugs in the credential sync pipeline:

1. **`hasCredentials` requires an auth token** — `KeychainHelper.hasCredentials` demands both a server URL _and_ a non-empty auth token. Servers configured without auth (token is nil) always fail this check, making the share extension permanently unusable.

2. **Fire-and-forget sync with silent error swallowing** — The `SharedCredentialsChannel.syncCredentials()` call in `context_repository.dart` is unawaited. If the method channel isn't registered yet (race with `didInitializeImplicitFlutterEngine`), or the Flutter engine tears down before the call completes, credentials never reach the shared Keychain group. Both the Dart and Swift sides silently swallow all errors.

3. **No recovery path** — There is no mechanism to re-sync or verify shared Keychain credentials. If the initial sync fails, the shared Keychain stays empty until the user deactivates and reactivates their context — and they have no way of knowing that's the fix.

## What Changes

- **Fix `hasCredentials` to accept URL-only** — A valid server URL is sufficient. The auth token should be optional (matching the `AssistantContext` model where `authToken: null` means "server requires no auth").
- **Eliminate method channel bridge on iOS** — Replace fire-and-forget `syncCredentials` method channel with direct `flutter_secure_storage` writes using `IOSOptions(groupId:)` which maps to `kSecAttrAccessGroup`. This removes the race condition and makes writes synchronous to the Keychain.
- **Retain method channel for macOS** — The `flutter_secure_storage_darwin` plugin guards `groupId` behind `#if os(iOS)`, so macOS continues using the native `syncCredentials` method channel as a fallback.
- **Resolve team prefix at startup** — Fetch the Apple Team ID prefix once via a lightweight `getTeamPrefix` method channel call, cache it, and inject into `ContextRepository` for constructing `IOSOptions(groupId:)`.
- **Cache `KeychainHelper.teamPrefix`** — Change from `static var` (recomputed on every access via Keychain probe) to `static let` (cached, thread-safe).

## Non-goals

- Adding UI in the main app for diagnosing share extension issues
- Offline queueing in the share extension

## Impact

- **Swift** (`AssistantIntents` package): `KeychainHelper.hasCredentials` accepts URL-only; `teamPrefix` cached via `static let`
- **Swift** (`SharedCredentialsChannel.swift`): iOS removes `syncCredentials`, adds `getTeamPrefix`; macOS retains both
- **Dart** (`context_repository.dart`): Direct `IOSOptions(groupId:)` writes on iOS; method channel fallback on macOS
- **Dart** (`shared_credentials_channel.dart`): `getTeamPrefix()` (cached) replaces `syncCredentials()` on iOS; `syncCredentialsMacOS()` retained
- **Dart** (`context_providers.dart`): Fetches team prefix at startup, injects into `ContextRepository`
