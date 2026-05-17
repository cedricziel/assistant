## ADDED Requirements

### Requirement: GET /api/conversations/{id}/turns/{turnId}/status returns the authoritative turn state

The server SHALL expose `GET /api/conversations/{conversationId}/turns/{turnId}/status` returning a JSON body describing the current state of the named turn. The response uses the standard `*Response` shape conventions.

Response body:

```json
{
  "turn_id": "uuid",
  "conversation_id": "uuid",
  "state": "running" | "completed" | "errored" | "unknown",
  "last_event_at": "RFC 3339 timestamp",
  "last_event_kind": "run_started" | "token" | "status" | "thinking" | "tool_result" | "skill_complete" | "agent_error" | "subagent_started" | "subagent_completed" | "audio_ready" | "done" | null
}
```

#### Scenario: A running turn returns state=running

- **WHEN** a turn is being processed by the runtime (LLM call or tool invocation in flight)
- **AND** a client issues `GET /api/conversations/{cid}/turns/{tid}/status`
- **THEN** the response SHALL be `200 OK` with body `state: "running"` and `last_event_at` populated with the timestamp of the most recent SSE event emitted for that turn
- **THEN** `last_event_kind` SHALL match the kind of that most recent event

#### Scenario: A completed turn returns state=completed

- **WHEN** a turn finished cleanly (the runtime emitted `done`)
- **AND** a client issues the status request
- **THEN** the response SHALL be `200 OK` with `state: "completed"`

#### Scenario: An errored turn returns state=errored

- **WHEN** a turn terminated with `agent_error`
- **AND** a client issues the status request
- **THEN** the response SHALL be `200 OK` with `state: "errored"` and `last_event_kind: "agent_error"`

#### Scenario: An unknown turn returns state=unknown

- **WHEN** a client issues the status request for a turn ID that was never recorded, has been garbage-collected, or belongs to a different conversation
- **THEN** the response SHALL be `200 OK` with `state: "unknown"` and `last_event_at: null`, `last_event_kind: null`
- **THEN** the response SHALL NOT be `404` — `unknown` is a valid response state, not an error

### Requirement: POST /api/conversations/{id}/turns/{turnId}/cancel cancels an in-flight turn

The server SHALL expose `POST /api/conversations/{conversationId}/turns/{turnId}/cancel` that explicitly cancels an in-flight turn. Cancellation propagates through the runtime, aborting any spawned LLM call, tool invocation, or subagent task.

#### Scenario: Cancelling a running turn succeeds with 204

- **WHEN** a turn is `running` and a client issues `POST .../cancel`
- **THEN** the server SHALL abort the runtime task processing the turn
- **THEN** the server SHALL respond `204 No Content` once the abort is acknowledged (does not wait for cleanup)
- **THEN** any partial assistant output already streamed for that turn SHALL be saved into the conversation as a single message with the project's existing `failed` / partial-message semantics — partial output is never silently dropped

#### Scenario: Cancelling an already-terminal turn returns 409

- **WHEN** the turn has already reached `completed` or `errored` by the time the cancel request is processed
- **THEN** the server SHALL respond `409 Conflict` with body `{"state": "completed" | "errored"}` so the client can reconcile to the actual terminal state without showing a misleading error

#### Scenario: Cancelling an unknown turn returns 404

- **WHEN** the turn ID is not recorded
- **THEN** the server SHALL respond `404 Not Found` with the standard error envelope `{"error": "..."}`

#### Scenario: Cancelled turn emits a final agent_error to open SSE streams

- **WHEN** any SSE consumer is connected to the turn's stream at the moment of cancel
- **THEN** the server SHALL emit one final `agent_error` SSE event with a cancellation marker (e.g. `data: {"kind": "cancelled", ...}`) before closing the stream
- **THEN** clients SHALL be able to surface the cancellation distinctively (e.g. "Cancelled — partial response") without polling status

### Requirement: OpenAPI documentation for the new endpoints

Both endpoints SHALL appear in `openapi.json` with full schemas, operation IDs in snake_case (`get_turn_status`, `cancel_turn`), and standard `*Response` body types. The generated Dart client (`app/packages/assistant_api/`) SHALL expose typed methods.

#### Scenario: openapi.json is updated

- **WHEN** `make dump-openapi` is run after the endpoints land
- **THEN** the resulting `openapi.json` SHALL include both operations with full request/response schemas
- **WHEN** `make generate-flutter-client` is run
- **THEN** the regenerated `app/packages/assistant_api/` SHALL expose typed methods for both endpoints
