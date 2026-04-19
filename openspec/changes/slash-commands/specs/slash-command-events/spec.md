## ADDED Requirements

### Requirement: Command invocations are stored as conversation events

The system SHALL persist each command invocation to a `conversation_events` table with columns: `id` (UUID), `conversation_id`, `event_type`, `command`, `payload` (JSON), `ack_text`, and `created_at`. These records SHALL NOT be stored in the `messages` table.

#### Scenario: Command event is persisted

- **WHEN** a user executes `/model claude-opus-4`
- **THEN** a row is inserted into `conversation_events` with `event_type = "command"`, `command = "model"`, `payload = {"model_name": "claude-opus-4"}`, and `ack_text = "Model switched to claude-opus-4."`

#### Scenario: Events are excluded from LLM context

- **WHEN** the orchestrator loads conversation history for a turn
- **THEN** it queries only the `messages` table
- **THEN** `conversation_events` records are never included in the LLM prompt

### Requirement: Command events are queryable per conversation

The system SHALL provide a `GET /api/conversations/{id}/events` endpoint that returns all events for a conversation as a bare JSON array sorted by `created_at` ascending.

#### Scenario: List events for a conversation

- **WHEN** a client calls `GET /api/conversations/{conv_id}/events`
- **THEN** it receives a JSON array of `CommandEventResponse` objects
- **THEN** results are ordered by `created_at` ascending

#### Scenario: No events exist

- **WHEN** a client calls `GET /api/conversations/{conv_id}/events` for a conversation with no commands executed
- **THEN** it receives an empty JSON array `[]`

### Requirement: REST endpoint to list available commands

The system SHALL provide a `GET /api/commands` endpoint that returns all registered command definitions as a bare JSON array.

#### Scenario: List commands

- **WHEN** a client calls `GET /api/commands`
- **THEN** it receives a JSON array of command definitions
- **THEN** each entry includes `name`, `description`, `category`, and `args`

### Requirement: REST endpoint to execute a command

The system SHALL provide a `POST /api/conversations/{id}/command` endpoint that executes a slash command and returns the resulting event.

#### Scenario: Execute valid command

- **WHEN** a client sends `POST /api/conversations/{conv_id}/command` with `{"command": "model", "args": ["claude-opus-4"]}`
- **THEN** the command is executed
- **THEN** the response is 200 with the `CommandEventResponse` body
- **THEN** a `conversation_events` row is persisted

#### Scenario: Execute unknown command

- **WHEN** a client sends `POST /api/conversations/{conv_id}/command` with `{"command": "unknown"}`
- **THEN** the response is 400 with `{"error": "Unknown command: unknown"}`

#### Scenario: Missing required argument

- **WHEN** a client sends `POST /api/conversations/{conv_id}/command` with `{"command": "model", "args": []}`
- **THEN** the command executes and shows the current model (no-arg behavior)
