## ADDED Requirements

### Requirement: AskAssistant App Intent

The iOS app SHALL define an `AskAssistantIntent` conforming to `AppIntent` (iOS 16+) that accepts a `question` string parameter, calls the server's `POST /api/quick-message` endpoint, and returns the assistant's answer as a spoken dialog result.

#### Scenario: Successful Siri interaction

- **WHEN** the user invokes the Intent via Siri with the question "What should I cook for dinner?"
- **THEN** Siri sends the question to the server via `POST /api/quick-message` and speaks the returned `answer` aloud

#### Scenario: Siri prompts for question

- **WHEN** the user invokes the Intent without providing a question (e.g., "Ask Assistant")
- **THEN** Siri prompts "What would you like to ask?" via the `requestValueDialog` on the question parameter

#### Scenario: Server unreachable

- **WHEN** the server is unreachable or returns a network error
- **THEN** the Intent returns a spoken dialog: "I couldn't reach your assistant server. Please check your connection."

#### Scenario: Request times out

- **WHEN** the server does not respond within 25 seconds
- **THEN** the Intent returns a spoken dialog: "I'm still working on that. Check the app for the full answer."

### Requirement: App Shortcuts Provider registration

The iOS app SHALL define an `AppShortcutsProvider` that registers `AskAssistantIntent` with system-discoverable phrases including:

- "Ask {applicationName} about {question}"
- "Ask {applicationName} {question}"

This makes the shortcut discoverable in the Shortcuts app and configurable as an Action Button action.

#### Scenario: Shortcut appears in Shortcuts app

- **WHEN** the user opens the iOS Shortcuts app and searches for the assistant app
- **THEN** the "Ask Assistant" shortcut appears with the registered phrases

#### Scenario: Action Button configuration

- **WHEN** the user navigates to Settings > Action Button on a supported iPhone
- **THEN** the "Ask Assistant" shortcut is available as an Action Button target via Shortcuts

### Requirement: Native HTTP client for Intent

The iOS app SHALL include a native Swift `AssistantAPIClient` class that:

- Reads the server URL and auth token from the shared Keychain
- Sends `POST /api/quick-message` requests with `Authorization: Bearer <token>` header
- Parses the JSON response to extract the `answer` field
- Uses a configurable timeout (default 25 seconds)

The client SHALL NOT depend on the Flutter engine.

#### Scenario: Client reads credentials from Keychain

- **WHEN** the Intent performs and the Keychain contains a server URL and auth token
- **THEN** the client uses those credentials to call the API

#### Scenario: No credentials in Keychain

- **WHEN** the Intent performs but no server URL or auth token exists in the Keychain
- **THEN** the Intent returns a spoken dialog: "Please open the app and connect to your assistant server first."

### Requirement: Siri usage description in Info.plist

The iOS app's `Info.plist` SHALL include `NSSiriUsageDescription` with a user-facing explanation of why the app uses Siri.

#### Scenario: Info.plist contains Siri description

- **WHEN** the iOS app is built
- **THEN** `Info.plist` contains the key `NSSiriUsageDescription` with value "Siri is used to send voice questions to your assistant and hear responses."
