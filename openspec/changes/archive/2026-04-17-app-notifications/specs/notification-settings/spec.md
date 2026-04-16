## ADDED Requirements

### Requirement: Per-category notification toggle

The system SHALL allow users to individually enable or disable each notification category (chat messages, skill completions, agent errors) via the settings screen.

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

### Requirement: Notification settings UI

The system SHALL display a "Notifications" section in the existing settings screen with a toggle per category.

#### Scenario: Settings section is present

- **WHEN** the user opens the settings screen
- **THEN** a "Notifications" section is visible with labeled toggles for each category

#### Scenario: Toggle reflects persisted state

- **WHEN** the settings screen loads
- **THEN** each toggle reflects the currently persisted preference value
