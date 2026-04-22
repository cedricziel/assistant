## Why

The iOS share extension always shows "Not Connected" even when the user is logged in and has an active context with valid credentials. The extension cannot access the shared Keychain credentials due to three compounding bugs in the credential sync pipeline:

1. **`hasCredentials` requires an auth token** — `KeychainHelper.hasCredentials` demands both a server URL _and_ a non-empty auth token. Servers configured without auth (token is nil) always fail this check, making the share extension permanently unusable.

2. **Fire-and-forget sync with silent error swallowing** — The `SharedCredentialsChannel.syncCredentials()` call in `context_repository.dart` is unawaited. If the method channel isn't registered yet (race with `didInitializeImplicitFlutterEngine`), or the Flutter engine tears down before the call completes, credentials never reach the shared Keychain group. Both the Dart and Swift sides silently swallow all errors.

3. **No recovery path** — There is no mechanism to re-sync or verify shared Keychain credentials. If the initial sync fails, the shared Keychain stays empty until the user deactivates and reactivates their context — and they have no way of knowing that's the fix.

## What Changes

- **Fix `hasCredentials` to accept URL-only** — A valid server URL is sufficient. The auth token should be optional (matching the `AssistantContext` model where `authToken: null` means "server requires no auth").
- **Await the shared Keychain sync** — Make the `SharedCredentialsChannel.syncCredentials()` call awaited so failures are observable. Log errors instead of swallowing them.
- **Add a verification round-trip** — After writing credentials via the method channel, read them back from the shared Keychain to confirm they landed. If verification fails, retry once and log an error.
- **Sync credentials on app foreground** — Re-sync shared Keychain credentials when the app returns to foreground, covering the case where the initial sync failed due to a race condition.

## Non-goals

- Changing the overall credential sharing architecture (shared Keychain group via method channel is correct)
- Adding UI in the main app for diagnosing share extension issues
- Offline queueing in the share extension
- Changing how `flutter_secure_storage` itself is configured

## Impact

- **Swift** (`AssistantIntents` package): `KeychainHelper.hasCredentials` logic change
- **Dart** (`context_repository.dart`): Await the sync call, add logging on failure
- **Dart** (`shared_credentials_channel.dart`): Stop swallowing errors, propagate failures
- **Swift** (`SharedCredentialsChannel.swift`): Add verification read-back after write
- **Dart** (app lifecycle): Add foreground re-sync hook
