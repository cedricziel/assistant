## MODIFIED Requirements

### Requirement: Tray context menu provides Open and Quit actions

The tray icon SHALL present a context menu with at least three items: the **active context name** (as a non-interactive label or submenu header), a **Switch Context** submenu listing all saved contexts, **Open**, and **Quit**. When no context is active the label SHALL read "No active context".

#### Scenario: User right-clicks or left-clicks the tray icon

- **WHEN** the user clicks the tray icon
- **THEN** a context menu appears with the active context name, "Switch Context", "Open", and "Quit" items

#### Scenario: User selects Open

- **WHEN** the user selects "Open" from the tray context menu
- **THEN** the main application window is shown and brought to focus

#### Scenario: User selects Quit

- **WHEN** the user selects "Quit" from the tray context menu
- **THEN** the application exits completely

#### Scenario: Active context name shown in tray menu

- **WHEN** a context named "Work" is active
- **THEN** the tray menu header or first item displays "Work"

#### Scenario: No active context

- **WHEN** no context is currently active
- **THEN** the tray menu header displays "No active context"

---

## ADDED Requirements

### Requirement: Tray menu allows quick context switching

The "Switch Context" submenu SHALL list all saved contexts. Selecting a context from the submenu SHALL activate it immediately without opening the main window.

#### Scenario: Switching context via tray submenu

- **WHEN** the user opens the "Switch Context" submenu and selects a context named "Personal"
- **THEN** "Personal" becomes the active context
- **THEN** the tray menu header updates to "Personal"
- **THEN** the main window (if open) reflects the new active context

#### Scenario: Only one context exists

- **WHEN** only one context is saved and it is already active
- **THEN** the "Switch Context" submenu shows that single context with a checkmark
- **THEN** selecting it has no effect
