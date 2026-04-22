## Context

The share extension credential pipeline had three layers:

```text
Flutter ContextRepository
  → flutter_secure_storage (app-only keychain, Siri backward compat)
  → SharedCredentialsChannel (Dart method channel → Swift)
      → KeychainHelper.write() (shared keychain group)

Share Extension
  → KeychainHelper.read() (shared keychain group)
  → KeychainHelper.hasCredentials (gate for showing UI vs "Not Connected")
```

The shared Keychain group (`TEAMID.com.cedricziel.assistant.shared`) and App Group (`group.com.cedricziel.assistant`) entitlements are correctly configured on both the Runner and ShareExtension targets. The architecture was sound but the method channel bridge introduced race conditions and silent failures.

## Decisions

### D1: `hasCredentials` requires only a server URL

Current code demanded both URL and non-empty auth token. Changed to require only a non-empty server URL.

**Why:** The `AssistantContext` model in Dart explicitly documents `authToken: null` as "server requires no auth." The Keychain helper should match this contract.

### D6: Direct Keychain writes via `flutter_secure_storage` IOSOptions (replaces D2–D5)

`flutter_secure_storage` v10 supports `IOSOptions(groupId:)` which maps directly to `kSecAttrAccessGroup`. This eliminates the need for a Swift method channel bridge to write shared Keychain items.

**Old architecture (method channel bridge):**

```text
Dart syncSiriCredentials()
  → flutter_secure_storage (default scope, Siri compat)
  → MethodChannel "syncCredentials" → Swift → KeychainHelper.write()
     to shared access group
```

**New architecture (direct write):**

```text
Dart syncSiriCredentials()
  → flutter_secure_storage (default scope, Siri compat)
  → flutter_secure_storage with IOSOptions(
      accountName: "com.cedricziel.assistant",  // kSecAttrService
      groupId: "TEAMID.com.cedricziel.assistant.shared"  // kSecAttrAccessGroup
    )
```

**Why:** The method channel bridge was a workaround for something `flutter_secure_storage` supports natively. The bridge introduced:

- A race condition (fire-and-forget, unawaited call)
- Silent error swallowing (all exceptions caught and ignored)
- No recovery path (failed sync stayed failed)
- Extra Swift code to maintain

Direct writes through `flutter_secure_storage` are synchronous to the Keychain — no race condition, no fire-and-forget.

**Team prefix resolution:** `IOSOptions(groupId:)` requires the full access group including the Apple Team ID prefix (e.g. `ABCDE12345.com.cedricziel.assistant.shared`). The team prefix is resolved once at startup via a lightweight `getTeamPrefix` method channel call and cached for the app's lifetime.

**macOS limitation:** The `flutter_secure_storage_darwin` plugin wraps `groupId` in `#if os(iOS)` — it has no effect on macOS. For macOS, the method channel `syncCredentials` call is retained as a fallback.

**Key mapping:**

| Dart                       | Native Keychain       | Value                                                          |
| -------------------------- | --------------------- | -------------------------------------------------------------- |
| `IOSOptions(accountName:)` | `kSecAttrService`     | `"com.cedricziel.assistant"`                                   |
| `IOSOptions(groupId:)`     | `kSecAttrAccessGroup` | `"TEAMID.com.cedricziel.assistant.shared"`                     |
| `write(key:)`              | `kSecAttrAccount`     | `"assistant_siri_server_url"` or `"assistant_siri_auth_token"` |

These match what the Swift `KeychainHelper` reads with in the share extension.

### SharedCredentialsChannel changes

| Method            | iOS                                       | macOS                                    |
| ----------------- | ----------------------------------------- | ---------------------------------------- |
| `getTeamPrefix`   | Returns `KeychainHelper.teamPrefix`       | Returns `KeychainHelper.teamPrefix`      |
| `syncCredentials` | **Removed** (direct write via IOSOptions) | **Retained** (groupId ignored by plugin) |

## Risks / Trade-offs

**[Risk] Team prefix unavailable at startup** — If the native channel isn't registered when `getTeamPrefix` is called, the prefix is `null` and shared Keychain writes are skipped. Mitigated: the channel is registered in `didInitializeImplicitFlutterEngine` which fires before `main()` calls `createContextRepository()`.

**[Risk] macOS still uses method channel** — Accepted. The `flutter_secure_storage_darwin` plugin's `#if os(iOS)` guard means we can't eliminate the bridge on macOS. When/if the plugin adds macOS support for `groupId`, the method channel can be removed there too.

**[Eliminated risk] Fire-and-forget race condition** — Direct `flutter_secure_storage` writes complete synchronously. No method channel timing dependency.

**[Eliminated risk] Silent error swallowing** — `flutter_secure_storage` throws on failure. Errors are caught and logged via `debugPrint`.
