## ADDED Requirements

### Requirement: Flutter writes credentials to shared Keychain group

The Flutter app SHALL configure `flutter_secure_storage` with `IOSOptions` specifying a `groupId` matching the app's Keychain access group (`$(AppIdentifierPrefix)$(PRODUCT_BUNDLE_IDENTIFIER)`). Server URL and auth token SHALL be written to this shared group so they are accessible to native Swift code.

#### Scenario: Credentials written with group ID

- **WHEN** the Flutter app stores the server URL and auth token via `flutter_secure_storage`
- **THEN** the values are persisted in the Keychain under the shared access group

#### Scenario: Existing credentials remain accessible

- **WHEN** a user upgrades from a pre-Siri build that stored credentials without a group ID
- **THEN** the app detects missing credentials in the shared group and re-writes them on next launch

### Requirement: Swift reads credentials from shared Keychain group

The iOS Runner SHALL include a `KeychainHelper` (or equivalent) class that reads the server URL and auth token from the shared Keychain access group using `SecItemCopyMatching` with matching `kSecAttrAccessGroup` and `kSecAttrService` attributes.

#### Scenario: Successful credential read

- **WHEN** the Flutter app has stored credentials and the Swift `KeychainHelper` reads the Keychain
- **THEN** the helper returns the server URL and auth token as strings

#### Scenario: No credentials stored

- **WHEN** the Keychain contains no credentials for the shared access group
- **THEN** the helper returns nil for both server URL and auth token

### Requirement: Keychain service name consistency

The Flutter `flutter_secure_storage` service name and the Swift `KeychainHelper` SHALL use the same `kSecAttrService` value to ensure credential lookup matches. The service name SHALL be the app's bundle identifier.

#### Scenario: Service name matches between Flutter and Swift

- **WHEN** Flutter writes a credential with service name "com.example.assistantApp"
- **THEN** the Swift KeychainHelper queries with the same service name and retrieves the credential

### Requirement: Keychain access group in entitlements

The iOS `Release.entitlements` and `DebugProfile.entitlements` SHALL include a `keychain-access-groups` entry matching `$(AppIdentifierPrefix)$(PRODUCT_BUNDLE_IDENTIFIER)`. This is already present in the current entitlements and SHALL be preserved.

#### Scenario: Entitlements contain access group

- **WHEN** the iOS app is built
- **THEN** both `Release.entitlements` and `DebugProfile.entitlements` contain the `keychain-access-groups` array with `$(AppIdentifierPrefix)$(PRODUCT_BUNDLE_IDENTIFIER)`
