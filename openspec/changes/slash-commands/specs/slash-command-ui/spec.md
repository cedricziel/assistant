## ADDED Requirements

### Requirement: Autocomplete popup triggered on `/`

The Flutter web UI SHALL display a command autocomplete popup when the user types `/` as the first character in the message input field. The popup SHALL list all available commands fetched from `GET /api/commands`.

#### Scenario: Popup appears on `/`

- **WHEN** the user types `/` as the first character in an empty input field
- **THEN** a popup appears showing all available commands with names and descriptions

#### Scenario: Popup does not appear mid-sentence

- **WHEN** the user types `use /model to switch` (slash not at position 0)
- **THEN** no popup appears

#### Scenario: Popup dismissed on Escape

- **WHEN** the autocomplete popup is visible and the user presses Escape
- **THEN** the popup is dismissed
- **THEN** the input field retains the current text

#### Scenario: Popup dismissed on backspace past `/`

- **WHEN** the autocomplete popup is visible and the user deletes the `/` character
- **THEN** the popup is dismissed

### Requirement: Command filtering as user types

The system SHALL filter the command list as the user types characters after `/`. Filtering SHALL be case-insensitive prefix matching on the command name.

#### Scenario: Filtered results

- **WHEN** the user types `/mo`
- **THEN** the popup shows only `/model` (and any other commands starting with `mo`)

#### Scenario: No matches

- **WHEN** the user types `/xyz`
- **THEN** the popup shows an empty state or is dismissed

### Requirement: Argument completion for commands

For commands with a `completions_endpoint` in their arg definition, the system SHALL fetch argument completions and display them when the user selects the command and begins typing the argument.

#### Scenario: Model name completion

- **WHEN** the user selects `/model` and starts typing the model name
- **THEN** the popup fetches available models from the completions endpoint
- **THEN** it shows matching model names as the user types

#### Scenario: Command with no arguments submits immediately

- **WHEN** the user selects `/new` (which takes no arguments)
- **THEN** the command is submitted immediately
- **THEN** the input field is cleared

### Requirement: Command events rendered in timeline

The Flutter web UI SHALL render command events from `GET /api/conversations/{id}/events` interleaved with messages by timestamp. Command events SHALL have a visually distinct style from chat messages (system-event appearance, not a chat bubble).

#### Scenario: Command event in timeline

- **WHEN** a conversation timeline contains messages and command events
- **THEN** events are displayed inline at their chronological position
- **THEN** events show the command name and ack text with distinct styling

#### Scenario: Model switch visible in timeline

- **WHEN** the user executed `/model claude-opus-4` between two messages
- **THEN** the timeline shows the model switch event between those messages
- **THEN** the event shows the command and the ack text
