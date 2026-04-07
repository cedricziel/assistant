# macOS Tray Spec

## Requirements

### Requirement: Tray icon is shown on macOS launch

The app SHALL display a menu bar status icon when launched on macOS.
The icon SHALL NOT appear when running as a web app.

#### Scenario: App launches on macOS

- **WHEN** the user launches the macOS build
- **THEN** a tray icon appears in the macOS menu bar
- **THEN** no icon appears in the macOS Dock

#### Scenario: App launches as web

- **WHEN** the app runs in a browser
- **THEN** no tray initialization occurs and the app renders normally

---

### Requirement: Tray context menu provides Open and Quit actions

The tray icon SHALL present a context menu with at least two items: **Open** and **Quit**.

#### Scenario: User right-clicks or left-clicks the tray icon

- **WHEN** the user clicks the tray icon
- **THEN** a context menu appears with "Open" and "Quit" items

#### Scenario: User selects Open

- **WHEN** the user selects "Open" from the tray context menu
- **THEN** the main application window is shown and brought to focus

#### Scenario: User selects Quit

- **WHEN** the user selects "Quit" from the tray context menu
- **THEN** the application exits completely

---

### Requirement: Window hides instead of quitting on close

When running on macOS, closing the main window (red X button) SHALL hide the window rather than quitting the application, keeping the tray icon active.

#### Scenario: User closes the main window

- **WHEN** the user clicks the red close button on the main window
- **THEN** the window is hidden
- **THEN** the tray icon remains in the menu bar
- **THEN** the application process continues running

#### Scenario: App is re-opened after window is hidden

- **WHEN** the window is hidden and the user selects "Open" from the tray
- **THEN** the main window reappears and is focused

---

### Requirement: App window is shown on first launch

On first launch the main window SHALL be visible so the user is not confused by an invisible app.

#### Scenario: Fresh launch with no prior window state

- **WHEN** the user launches the app for the first time
- **THEN** the main window opens and is visible
- **THEN** the tray icon is also present in the menu bar
