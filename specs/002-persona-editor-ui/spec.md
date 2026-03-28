# Feature Specification: Persona Editor UI

**Feature Branch**: `002-persona-editor-ui`
**Created**: 2026-03-28
**Status**: Draft
**Input**: User description: "Add a user interface to the web-ui to edit the persona files"

## Clarifications

### Session 2026-03-28

- Q: Should the UI allow users to set a persona as the default? → A: No — setting a default persona is explicitly out of scope; the default designation must not appear in this feature.

## User Scenarios & Testing _(mandatory)_

### User Story 1 - View and Edit Persona Markdown Files (Priority: P1)

An administrator opens the web UI and navigates to a persona management section. They see a list of all existing personas. They select a persona and see its associated markdown files (SOUL.md, IDENTITY.md, USER.md, MEMORY.md, AGENTS.md, TOOLS.md). They click on a file to open an inline text editor, make changes to the content, and save the file. The changes are immediately persisted to the filesystem.

**Why this priority**: The ability to view and edit the persona's defining markdown files is the core value of this feature. Without this, the persona system is opaque and requires direct filesystem access to configure.

**Independent Test**: Can be fully tested by navigating to a persona, opening SOUL.md, editing its content, saving it, reopening it, and confirming the updated content is displayed.

**Acceptance Scenarios**:

1. **Given** the user is on the web UI, **When** they navigate to the Personas section, **Then** they see a list of all personas stored in the system (ID and human-readable name only).
2. **Given** the user selects a persona, **When** the detail view opens, **Then** they see all standard markdown file slots listed (SOUL.md, IDENTITY.md, USER.md, MEMORY.md, AGENTS.md, TOOLS.md) with a visual indicator showing which files exist vs. are absent.
3. **Given** the user clicks on an existing markdown file, **When** the editor opens, **Then** the current file content is displayed in an editable text area.
4. **Given** the user edits content and clicks Save, **When** the save completes, **Then** a confirmation is shown and the file on the filesystem reflects the new content.
5. **Given** the user edits content and clicks Cancel or navigates away, **When** the action is taken, **Then** the original file content is preserved and a warning is shown if there are unsaved changes.

---

### User Story 2 - Create and Edit a New Persona Markdown File (Priority: P2)

When a persona's markdown file does not yet exist, the user can click a "Create" button for that file slot. An empty editor opens, the user types in content, and upon saving the file is created on the filesystem. This applies to all standard persona files.

**Why this priority**: Personas are often created without any files initially; allowing file creation from the UI completes the core editing workflow.

**Independent Test**: Can be tested by creating a SOUL.md file for a persona that has none, saving content, and confirming the file appears in the file list with the saved content.

**Acceptance Scenarios**:

1. **Given** a persona has no SOUL.md, **When** the user clicks "Create" next to SOUL.md, **Then** an empty editor opens for that file.
2. **Given** the user enters content and saves a new file, **When** save completes, **Then** the file appears in the file list and the filesystem reflects the new content.

---

### User Story 3 - Create a New Persona (Priority: P2)

An administrator wants to define a new persona for the assistant. They click "New Persona" in the web UI, enter a unique identifier and a human-readable name, and the persona is created and immediately available in the persona list.

**Why this priority**: Without the ability to create personas from the UI, users are limited to editing pre-existing ones. Creating new personas is the natural complement to file editing.

**Independent Test**: Can be tested by creating a persona with a unique ID and name, confirming it appears in the persona list, and verifying its file slots are accessible.

**Acceptance Scenarios**:

1. **Given** the user clicks "New Persona", **When** they enter a unique ID and name and confirm, **Then** the new persona appears in the persona list.
2. **Given** the new persona is created, **When** the user opens its detail view, **Then** all standard markdown file slots are listed showing "not yet created" status.
3. **Given** the user tries to create a persona with an ID that already exists, **When** they submit, **Then** an error is shown and no duplicate is created.

---

### Edge Cases

- What happens when a persona's markdown file is very large (e.g., hundreds of KB)? The editor must load the full content without truncation, or warn the user if the file exceeds a practical display limit.
- How does the system handle a persona whose directory is missing on the filesystem? The UI must gracefully show all files as absent and allow creating them through the UI.
- What if a save fails due to a permissions error or disk-full condition? A clear error message must be shown and the unsaved content must remain in the editor so the user does not lose their work.
- What if two browser sessions edit the same persona file simultaneously? Last-write wins is acceptable; the UI does not need real-time conflict resolution.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: System MUST display a list of all personas, showing each persona's ID and human-readable name. No default designation is shown or managed.
- **FR-002**: System MUST allow users to select a persona and view the list of its standard markdown file slots (SOUL.md, IDENTITY.md, USER.md, MEMORY.md, AGENTS.md, TOOLS.md, BOOTSTRAP.md, HEARTBEAT.md), with a clear indication for each whether the file currently exists.
- **FR-003**: System MUST allow users to open and edit the content of any existing persona markdown file through an in-browser text editor.
- **FR-004**: System MUST persist edits to the corresponding file on the server filesystem when the user saves.
- **FR-005**: System MUST allow users to create a new markdown file for a persona file slot that does not yet exist.
- **FR-006**: System MUST allow users to create a new persona by specifying a unique alphanumeric ID and a human-readable name.
- **FR-007**: System MUST warn users if they attempt to navigate away from an editor with unsaved changes.
- **FR-008**: System MUST display a success confirmation after a file is successfully saved.
- **FR-009**: System MUST display a clear error message if a file save operation fails, and preserve the unsaved content in the editor.
- **FR-010**: System MUST prevent creation of a persona with a duplicate ID.

### Key Entities

- **Persona**: An assistant identity with a unique ID, a human-readable name, and a set of associated markdown files on the filesystem.
- **Persona Markdown File**: A named markdown document (e.g., SOUL.md, IDENTITY.md) associated with a specific persona, stored on the filesystem under the persona's directory. Contains free-form text that shapes the assistant's behavior and memory.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: Users can view, open, edit, and save any persona markdown file within 30 seconds of focused interaction.
- **SC-002**: All persona markdown files for a given persona are accessible from a single screen without additional navigation.
- **SC-003**: 100% of save operations produce either a visible success confirmation or a visible error message — silent failures are not acceptable.
- **SC-004**: Users can create a new persona and begin editing its markdown files entirely within the web UI, without requiring direct filesystem access, within 60 seconds of initiating the creation flow.

## Assumptions

- The web UI is accessed by trusted administrators only; no additional per-user permission model is required for persona editing in this version.
- Setting or displaying a "default" persona designation is explicitly out of scope for this feature.
- Persona markdown files are stored on the server filesystem under `~/.assistant/agents/<persona-id>/` following the existing convention.
- The set of standard persona file slots (SOUL.md, IDENTITY.md, USER.md, MEMORY.md, AGENTS.md, TOOLS.md, BOOTSTRAP.md, HEARTBEAT.md) is fixed; support for arbitrary custom filenames is out of scope for this version.
- Persona management operates on the currently running server instance's filesystem; no remote filesystem abstraction is needed.
- Mobile browser support is out of scope; the UI targets desktop browsers.
- Real-time collaborative editing is out of scope; last-write wins is acceptable.
