## ADDED Requirements

### Requirement: Local Swift Package for shared Intents code

The system SHALL provide a local Swift Package at `app/packages/AssistantIntents/` that compiles for both iOS (16.0+) and macOS (13.0+). The package SHALL have no external dependencies beyond Foundation and Security frameworks.

#### Scenario: Package compiles for iOS target

- **WHEN** the iOS runner Xcode project resolves the local SPM dependency
- **THEN** the `AssistantIntents` module compiles without errors for iOS 16.0+

#### Scenario: Package compiles for macOS target

- **WHEN** the macOS runner Xcode project resolves the local SPM dependency
- **THEN** the `AssistantIntents` module compiles without errors for macOS 13.0+

#### Scenario: No external dependencies

- **WHEN** inspecting `Package.swift`
- **THEN** the package declares no package dependencies (only Foundation and Security system frameworks)

---

### Requirement: iOS runner uses package instead of local Intents files

The system SHALL remove the existing `app/ios/Runner/Intents/` directory. The iOS runner target SHALL import `AssistantIntents` from the local SPM package. All existing iOS Shortcuts functionality (AskAssistant, Siri phrases) SHALL continue to work.

#### Scenario: Existing iOS AskAssistant intent works after migration

- **WHEN** a user invokes "Ask Assistant a question" via Siri on iOS
- **THEN** the intent executes identically to the pre-migration behavior
- **THEN** the response is returned as a Siri dialog

#### Scenario: Old Intents directory removed

- **WHEN** inspecting the iOS runner directory
- **THEN** `app/ios/Runner/Intents/` does not exist
- **THEN** no duplicate Swift intent files exist outside the package

---

### Requirement: macOS runner imports the package

The macOS runner Xcode target SHALL add a local package dependency on `AssistantIntents`. After integration, all App Intents and App Entities defined in the package SHALL be available to macOS Shortcuts and Siri.

#### Scenario: macOS Shortcuts discovers assistant actions

- **WHEN** the user opens the Shortcuts app on macOS 13+
- **THEN** the assistant's actions appear in the action picker under the app name

#### Scenario: macOS Siri recognizes registered phrases

- **WHEN** the user says a registered Siri phrase (e.g., "Ask Assistant a question")
- **THEN** Siri invokes the corresponding App Intent from the package

---

### Requirement: Keychain helper reads credentials on both platforms

The `KeychainHelper` in the package SHALL read `assistant_siri_server_url` and `assistant_siri_auth_token` from the device Keychain using `kSecClassGenericPassword` with the app's bundle identifier as the service name. This SHALL work on both iOS and macOS.

#### Scenario: Credentials available on macOS

- **WHEN** the Flutter app has synced credentials to the Keychain on macOS
- **THEN** `KeychainHelper.serverURL` returns the stored server URL
- **THEN** `KeychainHelper.authToken` returns the stored auth token

#### Scenario: No credentials stored

- **WHEN** no credentials exist in the Keychain (fresh install, no context set)
- **THEN** `KeychainHelper.serverURL` returns nil
- **THEN** `KeychainHelper.authToken` returns nil

---

### Requirement: API client communicates with assistant server

The `AssistantAPIClient` in the package SHALL use `URLSession` to make HTTP requests to the assistant server. It SHALL read the server URL and auth token from `KeychainHelper`. All requests to authenticated endpoints SHALL include a `Bearer` token in the `Authorization` header.

#### Scenario: Successful API call with credentials

- **WHEN** credentials exist in the Keychain and the server is reachable
- **THEN** the API client sends requests with `Authorization: Bearer <token>` header
- **THEN** responses are decoded as expected

#### Scenario: No credentials available

- **WHEN** no credentials exist in the Keychain
- **THEN** the API client throws `APIError.noCredentials`

#### Scenario: Server unreachable

- **WHEN** credentials exist but the server does not respond within 25 seconds
- **THEN** the API client throws `APIError.timeout`

#### Scenario: Network error

- **WHEN** a network error occurs (DNS failure, connection refused, etc.)
- **THEN** the API client throws `APIError.networkError`
