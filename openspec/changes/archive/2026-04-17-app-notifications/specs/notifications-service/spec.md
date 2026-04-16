## ADDED Requirements

### Requirement: Platform notification permission

The system SHALL request OS-level notification permission before delivering the first notification, not at app startup.

#### Scenario: Permission requested on first notification trigger

- **WHEN** the notification service is asked to show a notification for the first time
- **THEN** the system requests OS permission if not already granted
- **AND** the notification is shown if permission is granted

#### Scenario: Permission previously denied

- **WHEN** OS permission has been permanently denied
- **THEN** the notification is silently skipped (no error thrown)
- **AND** the app displays a one-time in-app banner suggesting the user enable notifications in system settings

### Requirement: Show notification (macOS and web foreground)

The system SHALL deliver a native OS notification with a title, body, and optional payload when the app is running and the tab is active or the macOS app is in the foreground.

#### Scenario: Successful notification on macOS

- **WHEN** `NotificationService.show(title, body)` is called on macOS with permission granted
- **THEN** a macOS notification appears in Notification Center with the given title and body

#### Scenario: Successful notification on web (tab open)

- **WHEN** `NotificationService.show(title, body)` is called on web with permission granted and the tab is active
- **THEN** a browser notification appears with the given title and body

#### Scenario: Notification with conversation payload

- **WHEN** a notification is shown with a `conversationId` payload
- **THEN** tapping the notification navigates the app to that conversation via go_router

### Requirement: Graceful degradation

The system SHALL not crash or throw unhandled exceptions when notifications are unavailable (unsupported platform, denied permission, insecure context).

#### Scenario: Web in non-HTTPS context

- **WHEN** the app runs on web over HTTP (e.g., localhost dev)
- **THEN** notification calls complete without error and log a debug-level warning

#### Scenario: Unsupported platform

- **WHEN** the app runs on a platform not in scope (e.g., Linux)
- **THEN** all notification calls are no-ops
