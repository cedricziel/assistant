## ADDED Requirements

### Requirement: Real-time connectivity monitoring

The system SHALL expose a reactive connectivity state stream that reports the current network transport status (wifi, mobile, ethernet, none) across all target platforms (macOS, iOS, web).

#### Scenario: Connectivity state updates on network change

- **WHEN** the device network transport changes (e.g., Wi-Fi disconnects)
- **THEN** the connectivity provider SHALL emit the new connectivity result within 2 seconds of the OS reporting the change

#### Scenario: Initial connectivity state on app launch

- **WHEN** the app starts and the connectivity provider is first watched
- **THEN** the provider SHALL emit the current connectivity state before any user interaction occurs

### Requirement: Offline banner display

The system SHALL display a persistent, non-dismissible banner when connectivity is lost, visible across all screens.

#### Scenario: Banner appears when offline

- **WHEN** connectivity state transitions to none (no network transport)
- **THEN** a banner SHALL appear at the top of the navigation shell indicating "No internet connection"

#### Scenario: Banner disappears when connectivity returns

- **WHEN** connectivity state transitions from none to any connected state (wifi, mobile, ethernet)
- **THEN** the offline banner SHALL be removed immediately

#### Scenario: Banner is not shown on web platform

- **WHEN** the app is running in a web browser
- **THEN** the offline banner SHALL NOT be displayed (browsers provide native offline indicators)

### Requirement: Automatic SSE reconnection on connectivity restoration

The system SHALL automatically attempt to reconnect dropped SSE streams when network connectivity is restored, without requiring user interaction.

#### Scenario: Reconnect after Wi-Fi restore on desktop

- **WHEN** connectivity transitions from none to connected AND the chat provider has a pending reconnect (`_needsReconnect == true`)
- **THEN** the system SHALL call `attemptReconnect()` to resume the SSE event stream using the stored run ID

#### Scenario: No reconnect when no pending stream

- **WHEN** connectivity transitions from none to connected AND there is no pending reconnect state
- **THEN** the system SHALL NOT initiate any reconnection (no wasted requests)

#### Scenario: Reconnect coexists with lifecycle-based trigger

- **WHEN** connectivity is restored AND the app is subsequently resumed from background
- **THEN** only one reconnection attempt SHALL be made (the first trigger clears the `_needsReconnect` flag)

### Requirement: Offline guard for voice recording

The system SHALL prevent voice recording from starting when the device is offline, since the recording cannot be uploaded.

#### Scenario: Voice recording blocked while offline

- **WHEN** the user taps the voice record button AND connectivity state is none
- **THEN** the system SHALL NOT start recording AND SHALL display a brief message indicating that voice recording requires an internet connection

#### Scenario: Voice recording allowed when online

- **WHEN** the user taps the voice record button AND connectivity state is not none
- **THEN** voice recording SHALL proceed normally

### Requirement: Offline guard for file uploads

The system SHALL prevent file upload initiation when the device is offline.

#### Scenario: File upload blocked while offline

- **WHEN** the user attempts to attach a file AND connectivity state is none
- **THEN** the system SHALL NOT initiate the upload AND SHALL display a brief message indicating that file upload requires an internet connection

#### Scenario: File upload allowed when online

- **WHEN** the user attempts to attach a file AND connectivity state is not none
- **THEN** file upload SHALL proceed normally
