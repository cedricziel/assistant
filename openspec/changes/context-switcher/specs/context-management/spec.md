## ADDED Requirements

### Requirement: User can create a named context

The system SHALL allow a user to create a context by providing a display name and a server URL. An optional authentication token MAY be supplied. Each context SHALL be assigned a unique UUID on creation. Context names SHALL be unique; attempting to create a duplicate name SHALL result in an error.

#### Scenario: Successful context creation with name and URL

- **WHEN** the user submits a new context form with a non-empty name and a valid URL
- **THEN** the context is persisted to local storage
- **THEN** the context appears in the context list

#### Scenario: Duplicate name rejected

- **WHEN** the user submits a context with a name that already exists
- **THEN** an error message is shown: "A context with this name already exists"
- **THEN** no new context is stored

#### Scenario: Invalid URL rejected

- **WHEN** the user submits a context with a malformed URL (not http:// or https://)
- **THEN** an error message is shown: "Please enter a valid server URL"
- **THEN** no new context is stored

#### Scenario: Optional auth token stored securely

- **WHEN** the user provides an auth token during context creation
- **THEN** the token is stored in the device secure storage (keychain on macOS)
- **THEN** the token is NOT stored in shared_preferences

---

### Requirement: User can edit an existing context

The system SHALL allow a user to modify the name, URL, and auth token of any existing context. If the edited context is currently active, the updated values SHALL take effect immediately.

#### Scenario: Editing an inactive context

- **WHEN** the user edits and saves changes to an inactive context
- **THEN** the updated values are persisted
- **THEN** the active context is unaffected

#### Scenario: Editing the active context

- **WHEN** the user edits and saves changes to the currently active context
- **THEN** the updated values are persisted
- **THEN** the active context reflects the new name and URL immediately

---

### Requirement: User can delete a context

The system SHALL allow a user to delete any context. Deleting the active context SHALL deactivate it, setting the active context to `null` and redirecting the user to the Context Switcher screen.

#### Scenario: Deleting an inactive context

- **WHEN** the user deletes a context that is not currently active
- **THEN** the context is removed from storage
- **THEN** the context no longer appears in the list
- **THEN** the active context is unchanged

#### Scenario: Deleting the active context

- **WHEN** the user deletes the currently active context
- **THEN** the context is removed from storage
- **THEN** the active context is set to null
- **THEN** the app navigates to the Context Switcher screen

---

### Requirement: Active context persists across app restarts

The system SHALL persist the currently active context ID to local storage. On launch, the system SHALL restore the active context from storage. If the stored ID no longer exists, the active context SHALL be set to `null`.

#### Scenario: Active context restored on relaunch

- **WHEN** the user selects a context and then restarts the app
- **THEN** the same context is active after restart
- **THEN** the app proceeds directly to the main screen without showing the switcher

#### Scenario: Stored context ID no longer exists

- **WHEN** the app launches and the stored active context ID does not match any known context
- **THEN** the active context is set to null
- **THEN** the Context Switcher screen is shown

---

### Requirement: Context list is ordered by creation date

The system SHALL display contexts ordered by `createdAt` ascending (oldest first) so the list is stable across app restarts.

#### Scenario: Multiple contexts displayed in creation order

- **WHEN** the user has created contexts "Personal" then "Work"
- **THEN** "Personal" appears before "Work" in the list
