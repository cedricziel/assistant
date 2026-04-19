## ADDED Requirements

### Requirement: Native share extension appears in system share sheet

The app SHALL register a share extension target on both iOS and macOS that appears in the system share sheet when the user shares content from any app. The extension SHALL accept URLs, images, PDFs, text files, markdown files, CSV files, and JSON files.

#### Scenario: Share extension appears for supported content

- **WHEN** a user taps the share button in any app (Safari, Files, Photos, Mail, etc.)
- **THEN** "Assistant" appears as an option in the share sheet
- **THEN** tapping it opens the share extension UI as a modal sheet

#### Scenario: Share extension accepts URLs

- **WHEN** the user shares a URL (e.g., from Safari)
- **THEN** the extension receives the URL via `NSItemProvider` with `kUTTypeURL`
- **THEN** the URL is included as text in the message body, not uploaded as a file

#### Scenario: Share extension accepts files

- **WHEN** the user shares a file (PDF, image, text, markdown, CSV, JSON) from Files.app or another app
- **THEN** the extension receives the file via `NSItemProvider` with the appropriate UTI
- **THEN** the file is uploaded to the server as an attachment via streaming multipart upload

#### Scenario: Share extension accepts images from Photos

- **WHEN** the user shares an image from the Photos app
- **THEN** the extension receives the image data
- **THEN** the image is uploaded as an attachment (HEIC images SHALL be converted to PNG before upload)

### Requirement: Share extension provides conversation picker

The share extension UI SHALL display a conversation picker that defaults to "New conversation" and lists recent conversations fetched from the server API.

#### Scenario: Conversation list loads successfully

- **WHEN** the share extension opens and the server is reachable
- **THEN** the extension fetches conversations from `GET /api/conversations`
- **THEN** the picker displays "New conversation" as the default selection
- **THEN** recent conversations are listed below with their title and relative timestamp

#### Scenario: User selects an existing conversation

- **WHEN** the user selects an existing conversation from the picker
- **THEN** the shared content is sent to that conversation (attachment upload + message)

#### Scenario: User creates a new conversation

- **WHEN** the user keeps the default "New conversation" selection and taps Send
- **THEN** the extension creates a new conversation via `POST /api/conversations`
- **THEN** the shared content is sent to the newly created conversation

#### Scenario: Server unreachable

- **WHEN** the share extension opens but the server is unreachable
- **THEN** only "New conversation" is shown in the picker
- **THEN** an error banner explains the connection issue
- **THEN** the user can still attempt to send (which will fail with a clear error)

### Requirement: Share extension provides persona selector

The share extension UI SHALL display a persona selector allowing the user to choose which persona receives the shared content.

#### Scenario: Persona list loads successfully

- **WHEN** the share extension opens and the server is reachable
- **THEN** the extension fetches personas from `GET /api/personas`
- **THEN** the persona selector displays all available personas
- **THEN** the currently active persona is pre-selected

#### Scenario: User selects a different persona

- **WHEN** the user selects a persona different from the default
- **THEN** the extension switches the active persona via `POST /api/personas/active` before sending the message

#### Scenario: Server unreachable for persona list

- **WHEN** the persona list cannot be fetched
- **THEN** the persona selector is hidden
- **THEN** the server's currently active persona is used implicitly

### Requirement: Share extension provides optional message field

The share extension UI SHALL include a text field where the user can type an optional message to accompany the shared content.

#### Scenario: User sends with a message

- **WHEN** the user types "Summarize the key findings" and taps Send
- **THEN** the message text is sent along with the attachment to the selected conversation

#### Scenario: User sends without a message

- **WHEN** the user taps Send without typing a message
- **THEN** the attachment is sent with an empty message body
- **THEN** the assistant receives and can respond to the file content alone

### Requirement: Share extension uploads files via streaming multipart

The share extension SHALL upload files using streaming multipart requests to avoid buffering the entire file in memory.

#### Scenario: Large file upload within memory limits

- **WHEN** the user shares a 20 MB PDF
- **THEN** the extension streams the file bytes to `POST /api/conversations/{id}/attachments` without loading the full file into memory at once
- **THEN** the upload completes successfully within the iOS 120 MB memory limit

#### Scenario: File exceeds server size limit

- **WHEN** the user shares a file larger than 25 MB
- **THEN** the extension displays an error: "File exceeds maximum size of 25 MB"
- **THEN** the extension does not attempt the upload

### Requirement: Share extension reads credentials from shared Keychain

The share extension process SHALL read server URL and auth token from a shared Keychain access group that is also writable by the main Flutter app.

#### Scenario: Credentials available

- **WHEN** the share extension opens and credentials exist in the shared Keychain
- **THEN** the extension reads `assistant_siri_server_url` and `assistant_siri_auth_token`
- **THEN** API requests use these credentials for authentication

#### Scenario: No credentials configured

- **WHEN** the share extension opens but no credentials are found in the shared Keychain
- **THEN** the extension displays: "No server credentials found. Open the app and connect to your assistant server first."
- **THEN** the Send button is disabled

### Requirement: Share extension works on both iOS and macOS with shared UI

The share extension SHALL use a single SwiftUI view that adapts to both iOS and macOS share sheet presentation styles.

#### Scenario: iOS presentation

- **WHEN** the share extension is invoked on iOS or iPadOS
- **THEN** the UI presents as a modal sheet within the share sheet

#### Scenario: macOS presentation

- **WHEN** the share extension is invoked on macOS
- **THEN** the UI presents as a popover or small window within the share sheet

### Requirement: Shared Swift package for common code

The `KeychainHelper` and `AssistantAPIClient` Swift code SHALL be extracted into a shared local Swift package that both the Siri Intents target and the share extension target link against.

#### Scenario: Both targets use shared package

- **WHEN** the Xcode project is built
- **THEN** both the Intents extension and the share extension link against the shared `AssistantShared` package
- **THEN** both targets use the same `KeychainHelper` and `AssistantAPIClient` implementations

### Requirement: App Group and shared Keychain entitlements

Both the main app and the share extension SHALL declare matching App Group and shared Keychain access group entitlements on iOS and macOS.

#### Scenario: Entitlements configured on iOS

- **WHEN** the iOS app and share extension are built
- **THEN** both targets have the App Group entitlement (e.g., `group.com.cedricziel.assistant`)
- **THEN** both targets have the shared Keychain access group (e.g., `$(AppIdentifierPrefix)com.cedricziel.assistant.shared`)

#### Scenario: Entitlements configured on macOS

- **WHEN** the macOS app and share extension are built
- **THEN** both targets have matching App Group and shared Keychain access group entitlements

### Requirement: Main app migrates Keychain credentials to shared access group

The Flutter app SHALL migrate existing Keychain credentials from the app-scoped access group to the shared access group on first launch after the update.

#### Scenario: Migration on first launch

- **WHEN** the app launches after the update and credentials exist in the old Keychain scope
- **THEN** the app reads credentials from the old scope
- **THEN** the app writes them to the shared access group
- **THEN** the app deletes the old-scope entries
- **THEN** subsequent reads by the share extension succeed

#### Scenario: Fresh install (no migration needed)

- **WHEN** the app is installed fresh (no existing Keychain entries)
- **THEN** credentials are written directly to the shared access group
- **THEN** no migration is performed
