## Tasks

- [x] **T1: Fix `hasCredentials` to accept URL-only** — `app/packages/AssistantIntents/Sources/AssistantIntents/API/KeychainHelper.swift` — Change `hasCredentials` to only require a non-empty `serverURL`. Remove the `authToken` guard from the check. (Design: D1)

- [x] **T5: Add unit test for `hasCredentials` without token** — Write a Swift test confirming that `hasCredentials` returns `true` when `serverURL` is set but `authToken` is nil/empty.

- [x] **T7: Add `getTeamPrefix` in SharedCredentialsChannel (Swift)** — iOS removes the `syncCredentials` case and uses direct `IOSOptions(groupId:)` writes; macOS retains `syncCredentials` as the fallback and also adds `getTeamPrefix`. (Design: D6)

- [x] **T8: Replace `syncCredentials` with `getTeamPrefix` in SharedCredentialsChannel (Dart)** — `shared_credentials_channel.dart`: remove `syncCredentials`, add `getTeamPrefix()` that calls the native side and caches the result. Log errors instead of swallowing. (Design: D6)

- [x] **T9: Write directly to shared Keychain via IOSOptions in syncSiriCredentials** — `context_repository.dart`: use `flutter_secure_storage` with `IOSOptions(accountName: 'com.cedricziel.assistant', groupId: '$teamPrefix...')` to write Siri credentials directly. Keep method channel fallback for macOS where `groupId` is ignored by the plugin. (Design: D6)

- [x] **T10: Fetch team prefix at startup and inject into ContextRepository** — `context_providers.dart` + `main.dart`: call `SharedCredentialsChannel.getTeamPrefix()` during init, pass to `ContextRepository` constructor so it has the prefix for `IOSOptions`. (Design: D6)

- [x] **T11: Remove foreground re-sync from main.dart** — The direct-write approach eliminates the race condition, so the foreground re-sync (T4) is no longer needed. Revert the `_resyncSharedCredentials` addition.

- [ ] **T12: Manual verification on device** — Build and run on a physical iPhone. Connect to a server. Open share sheet from Safari or Files. Confirm the share extension shows the form instead of "Not Connected." Share a file and confirm it arrives in the conversation.
