## MODIFIED Requirements

### Requirement: Per-category notification toggle

The system SHALL allow users to individually enable or disable each notification category (chat messages, skill completions, agent errors, approval requests) via the settings screen.

#### Scenario: All categories enabled by default

- **WHEN** the app is installed fresh (no prior preferences)
- **THEN** all notification categories are enabled

#### Scenario: User disables chat notifications

- **WHEN** the user toggles "New chat messages" off in Settings
- **THEN** no notifications are shown for incoming chat messages
- **AND** the preference persists across app restarts

#### Scenario: User re-enables a category

- **WHEN** the user toggles a previously disabled category back on
- **THEN** notifications for that category resume immediately

#### Scenario: Approval request while backgrounded

- **WHEN** an `interaction_requested` event arrives on the active stream, the app is not in the `resumed` lifecycle state, and "Approval requests" is enabled
- **THEN** a local notification titled "Approval needed" (tool approval) or "Your assistant has a question" (question) SHALL be shown with the conversation id as payload

#### Scenario: No duplicate on web

- **WHEN** the web app has an active push subscription and an `interaction_requested` event arrives while it is not resumed
- **THEN** no local notification SHALL be shown, because the delayed push covers it

#### Scenario: Approval notifications disabled

- **WHEN** "Approval requests" is toggled off
- **THEN** no local notification SHALL be shown for interaction requests
