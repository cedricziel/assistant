## ADDED Requirements

### Requirement: Synchronous quick-message endpoint

The system SHALL expose `POST /api/quick-message` that accepts a JSON body with a `message` field, creates a new conversation under the active persona, submits the message as a turn to the orchestrator, awaits the complete `TurnResult`, and returns a JSON response containing the conversation ID, message ID, and full assistant answer.

The endpoint SHALL use `operationId: create_quick_message` in the OpenAPI spec.

#### Scenario: Successful quick message

- **WHEN** a client sends `POST /api/quick-message` with body `{"message": "What is 2+2?"}`
- **THEN** the server creates a new conversation, processes the turn, and returns HTTP 201 with body `{"conversation_id": "<uuid>", "message_id": "<uuid>", "answer": "2+2 equals 4."}`

#### Scenario: Empty message rejected

- **WHEN** a client sends `POST /api/quick-message` with body `{"message": "  "}`
- **THEN** the server returns HTTP 400 with body `{"error": "Message cannot be empty"}`

#### Scenario: Missing message field

- **WHEN** a client sends `POST /api/quick-message` with body `{}`
- **THEN** the server returns HTTP 400 (deserialization failure)

#### Scenario: Unauthorized request

- **WHEN** a client sends `POST /api/quick-message` without a valid `Authorization: Bearer <token>` header
- **THEN** the server returns HTTP 401

### Requirement: Quick-message auto-titles conversation

The system SHALL set the new conversation's title to the first 57 characters of the message text (with `...` appended if truncated), matching the existing `send_message` auto-title behavior.

#### Scenario: Short message becomes title

- **WHEN** a client sends a quick message with text "What should I cook for dinner?"
- **THEN** the created conversation has the title "What should I cook for dinner?"

#### Scenario: Long message title is truncated

- **WHEN** a client sends a quick message with text longer than 60 characters
- **THEN** the created conversation's title is the first 57 characters followed by "..."

### Requirement: Quick-message uses active persona

The system SHALL create the conversation and submit the turn under the server's currently active persona (`agent_id` from `ApiState`). The endpoint SHALL NOT accept a `persona_id` parameter.

#### Scenario: Message uses active persona

- **WHEN** the server's active persona is "chef" and a client sends a quick message
- **THEN** the conversation is created under the "chef" persona and the turn is processed by the "chef" worker

### Requirement: Quick-message response shape

The response body SHALL be a JSON object with the following fields:

- `conversation_id` (string, format: uuid) — the ID of the newly created conversation
- `message_id` (string, format: uuid, optional) — the ID of the persisted assistant message, omitted if unavailable
- `answer` (string) — the full assistant response text

#### Scenario: Response includes all fields

- **WHEN** a quick message completes successfully
- **THEN** the response contains `conversation_id`, `answer`, and `message_id` fields with correct types

### Requirement: Quick-message request shape

The request body SHALL be a JSON object named `QuickMessageRequest` with:

- `message` (string, required) — the text to send to the assistant

#### Scenario: Valid request accepted

- **WHEN** a client sends `{"message": "Hello"}`
- **THEN** the server accepts the request and processes it

### Requirement: Orchestrator error handling

The system SHALL return HTTP 500 with an error envelope if the orchestrator fails to process the turn (e.g., LLM provider unreachable).

#### Scenario: Orchestrator failure

- **WHEN** the orchestrator returns an error during turn processing
- **THEN** the server returns HTTP 500 with body `{"error": "Failed to process message"}`

### Requirement: OpenAPI documentation

The `POST /api/quick-message` endpoint SHALL be annotated with `utoipa` attributes and included in the OpenAPI spec under the `conversations` tag.

#### Scenario: Endpoint appears in OpenAPI spec

- **WHEN** the OpenAPI spec is generated via `make dump-openapi`
- **THEN** the spec includes the `POST /api/quick-message` path with request body, response schemas, and security requirement
