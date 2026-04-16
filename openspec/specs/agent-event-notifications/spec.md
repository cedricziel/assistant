## ADDED Requirements

### Requirement: New chat message notification

The system SHALL show a notification when a new chat message arrives from the assistant while the app window is not the focused window.

#### Scenario: Notification shown for incoming message when unfocused

- **WHEN** a new assistant message is received via SSE
- **AND** the app window does not have focus (backgrounded or minimized)
- **AND** the "New chat messages" notification category is enabled
- **THEN** a notification appears with title "New message" and the first 80 characters of the message as the body

#### Scenario: No notification when app is focused

- **WHEN** a new assistant message is received
- **AND** the app window has focus
- **THEN** no notification is shown (the message is visible in the chat UI)

#### Scenario: Notification not shown when category disabled

- **WHEN** a new assistant message is received
- **AND** the "New chat messages" category is disabled in settings
- **THEN** no notification is shown

### Requirement: Skill/workflow run completion notification

The system SHALL show a notification when a skill or workflow run completes (success or failure) while the user is away.

#### Scenario: Skill run success notification

- **WHEN** a skill run completes successfully
- **AND** the "Skill completions" notification category is enabled
- **THEN** a notification appears with title "Skill complete" and the skill name in the body

#### Scenario: Skill run failure notification

- **WHEN** a skill run completes with an error
- **AND** the "Skill completions" category is enabled
- **THEN** a notification appears with title "Skill failed" and the skill name and error summary in the body

### Requirement: Agent critical error notification

The system SHALL show a notification when the agent encounters a critical error that requires user attention.

#### Scenario: Critical error notification

- **WHEN** the agent emits a critical error event
- **AND** the "Agent errors" notification category is enabled
- **THEN** a notification appears with title "Assistant error" and a brief error description

### Requirement: macOS tray badge

The system SHALL badge the macOS tray icon with the count of unread notifications and clear the badge when the app gains focus.

#### Scenario: Badge increments on new notification

- **WHEN** a notification is delivered on macOS
- **THEN** the tray icon badge count increments by 1

#### Scenario: Badge cleared on app focus

- **WHEN** the app window gains focus (foreground lifecycle state)
- **THEN** the tray icon badge is cleared and the unread count resets to 0
