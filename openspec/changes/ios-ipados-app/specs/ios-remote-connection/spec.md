## ADDED Requirements

### Requirement: iOS onboarding shows remote-only connection form

On iOS and iPadOS the connection screen SHALL display only the remote server form (URL + token). The embedded-server mode toggle SHALL NOT be shown, because the Rust binary cannot run on iOS.

#### Scenario: User opens app on iOS for the first time

- **WHEN** the user launches the app on an iOS or iPadOS device with no saved context
- **THEN** the connection screen is shown
- **THEN** only the remote server URL field and optional authentication token field are visible
- **THEN** no segmented button for "Embedded (local)" vs "Remote server" is shown

#### Scenario: User submits a valid server URL

- **WHEN** the user enters a valid `http://` or `https://` URL and taps "Connect"
- **THEN** the app creates and activates a new AssistantContext with that URL
- **THEN** the app navigates to the chat screen

#### Scenario: User submits an invalid URL

- **WHEN** the user enters a malformed URL and taps "Connect"
- **THEN** a validation error is shown inline
- **THEN** no network request is made

#### Scenario: User enters an optional authentication token

- **WHEN** the user enters a non-empty token alongside a valid URL and taps "Connect"
- **THEN** the token is stored securely in the iOS Keychain via flutter_secure_storage
- **THEN** subsequent API calls include the token as a Bearer header

### Requirement: Saved contexts persist across app restarts on iOS

Server URL and authentication token saved in the iOS Keychain SHALL survive app termination and device reboot. On next launch the app SHALL skip the connection screen and navigate directly to chat if a valid active context exists.

#### Scenario: User reopens the app after connecting

- **WHEN** the user previously connected and terminates the app
- **THEN** on next launch the connection screen is NOT shown
- **THEN** the chat screen is displayed immediately with the previously active context

#### Scenario: User deletes the active context

- **WHEN** the user removes the only active context from settings
- **THEN** the app navigates back to the connection screen on next navigation event
