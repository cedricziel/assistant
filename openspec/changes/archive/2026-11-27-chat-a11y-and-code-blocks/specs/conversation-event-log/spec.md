## MODIFIED Requirements

### Requirement: All orchestrator events are persisted to the conversation event log

Every event emitted during an orchestrator run (token, status, tool_call, tool_result, done, error) SHALL be written to the `conversation_events` table with a monotonically increasing sequence number scoped to the run. The persisted `done` payload SHALL be the same `SseDoneEvent` object sent live, including `timing` when the turn produced one, so that replay returns it unchanged.

#### Scenario: Token events are logged

- **WHEN** the LLM emits a token during an orchestrator run
- **THEN** the token SHALL be persisted as a `conversation_events` row with `event_type = "token"` and `payload = {"token": "<text>"}`

#### Scenario: Done event is logged

- **WHEN** the orchestrator emits a `done` event
- **THEN** the event SHALL be persisted with `event_type = "done"` and `payload` containing `{"role": "assistant", "content": "<full_text>", "message_id": "<message_id>"}`, plus `"timing": {"total_ms": <int>, ...}` when the turn produced timing

#### Scenario: Error event is logged

- **WHEN** the orchestrator emits an `agent_error` event
- **THEN** the event SHALL be persisted with `event_type = "error"` and `payload` containing `{"message": "<error_text>"}`
