# keyboard-new-chat Specification

## Purpose
TBD - created by archiving change cmd-n-new-chat. Update Purpose after archive.
## Requirements
### Requirement: Cmd+N creates a new conversation

The app SHALL create a new conversation and navigate to it when the user presses Cmd+N (macOS) or Ctrl+N (other platforms).

#### Scenario: User presses Cmd+N on macOS

- **WHEN** the user presses Cmd+N on macOS
- **THEN** the system creates a new conversation via the existing create-conversation API and navigates to `/chat/{new-id}`

#### Scenario: User presses Ctrl+N on non-macOS

- **WHEN** the user presses Ctrl+N on a non-macOS platform (web on Windows/Linux)
- **THEN** the system creates a new conversation and navigates to `/chat/{new-id}`

### Requirement: Shortcut works globally across all screens

The keyboard shortcut SHALL be active on all screens in the app, not only when the chat input field is focused.

#### Scenario: User presses Cmd+N from settings screen

- **WHEN** the user is on a non-chat screen (e.g., personas, traces, logs, skills) and presses Cmd+N
- **THEN** the system creates a new conversation and navigates to `/chat/{new-id}`

#### Scenario: User presses Cmd+N while chat input is focused

- **WHEN** the user has focus in the chat message input and presses Cmd+N
- **THEN** the system creates a new conversation and navigates to the new chat (the shortcut is not swallowed by the text field)

### Requirement: Shortcut does not duplicate in-flight requests

The system SHALL ignore additional Cmd+N presses while a conversation creation request is already in flight.

#### Scenario: User double-taps Cmd+N rapidly

- **WHEN** the user presses Cmd+N twice in quick succession
- **THEN** only one new conversation is created

### Requirement: New conversation starts empty

The newly created conversation SHALL be empty with no pre-filled messages and the chat input focused.

#### Scenario: Fresh chat after Cmd+N

- **WHEN** a new conversation is created via Cmd+N
- **THEN** the chat screen shows an empty message timeline and the text input is focused and ready for typing

