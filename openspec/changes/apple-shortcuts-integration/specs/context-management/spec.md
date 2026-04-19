## MODIFIED Requirements

### Requirement: Siri credentials sync works on macOS

The system SHALL sync the active context's server URL and auth token to well-known Keychain keys (`assistant_siri_server_url`, `assistant_siri_auth_token`) on macOS, using the same mechanism as iOS. The `syncSiriCredentials()` method in `ContextRepository` SHALL function identically on both platforms via `flutter_secure_storage_darwin`.

#### Scenario: Credentials synced on macOS context switch

- **WHEN** the user switches the active context on macOS
- **THEN** `assistant_siri_server_url` is updated in the macOS Keychain with the new context's server URL
- **THEN** `assistant_siri_auth_token` is updated in the macOS Keychain with the new context's auth token

#### Scenario: Credentials cleared on macOS when no active context

- **WHEN** the user deactivates all contexts on macOS (active context set to null)
- **THEN** `assistant_siri_server_url` is deleted from the macOS Keychain
- **THEN** `assistant_siri_auth_token` is deleted from the macOS Keychain

#### Scenario: Swift package reads macOS Keychain credentials

- **WHEN** the Flutter app has synced credentials on macOS
- **THEN** the `KeychainHelper` in the `AssistantIntents` Swift package reads the same values
- **THEN** the bundle identifier used as Keychain service name matches between Flutter and Swift
