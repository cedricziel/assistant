## MODIFIED Requirements

### Requirement: GET /api/conversations/{id}/turns/{turnId}/status returns the authoritative turn state

The server SHALL expose `GET /api/conversations/{conversationId}/turns/{turnId}/status` returning a JSON body describing the current state of the named turn. The response uses the standard `*Response` shape conventions. `state` SHALL depend on which rows are present in the turn's event log, not on the kind of the last row: `errored` if any `agent_error` row exists, otherwise `completed` if any `done` row exists, otherwise `awaiting_input` if any `interaction_requested` row has no `interaction_resolved` row with the same `interaction_id`, otherwise `running`. `last_event_at` and `last_event_kind` SHALL describe the most recent row, which MAY be an event persisted after `done` (for example `suggestions`) or after `interaction_requested` (for example `surface_update`).

Response body:

```json
{
  "turn_id": "uuid",
  "conversation_id": "uuid",
  "state": "running" | "awaiting_input" | "completed" | "errored" | "unknown",
  "last_event_at": "RFC 3339 timestamp",
  "last_event_kind": "run_started" | "token" | "status" | "thinking" | "tool_result" | "skill_complete" | "agent_error" | "subagent_started" | "subagent_completed" | "audio_ready" | "done" | "suggestions" | "interaction_requested" | "interaction_resolved" | "surface_update" | null
}
```

#### Scenario: A running turn returns state=running

- **WHEN** a turn is being processed by the runtime (LLM call or tool invocation in flight)
- **AND** a client issues `GET /api/conversations/{cid}/turns/{tid}/status`
- **THEN** the response SHALL be `200 OK` with body `state: "running"` and `last_event_at` populated with the timestamp of the most recent SSE event emitted for that turn
- **THEN** `last_event_kind` SHALL match the kind of that most recent event

#### Scenario: A turn waiting on its owner returns state=awaiting_input

- **GIVEN** a turn whose log contains `interaction_requested` for an interaction with no matching `interaction_resolved`
- **WHEN** a client issues the status request
- **THEN** the response SHALL be `200 OK` with `state: "awaiting_input"`, even when a later row (for example `status` or `surface_update`) is the most recent

#### Scenario: A resolved interaction returns to running

- **GIVEN** a turn whose log contains `interaction_requested` and a matching `interaction_resolved`, and no terminal row
- **WHEN** a client issues the status request
- **THEN** the response SHALL be `200 OK` with `state: "running"`

#### Scenario: A completed turn returns state=completed

- **WHEN** a turn finished cleanly (the runtime emitted `done`)
- **AND** a client issues the status request
- **THEN** the response SHALL be `200 OK` with `state: "completed"`

#### Scenario: A completed turn with trailing suggestions stays completed

- **GIVEN** a turn whose log ends with `done` followed by `suggestions`
- **WHEN** a client issues the status request
- **THEN** the response SHALL be `200 OK` with `state: "completed"` and `last_event_kind: "suggestions"`

#### Scenario: An errored turn with a later row stays errored

- **GIVEN** a turn whose log holds `agent_error` followed by another row
- **WHEN** a client issues the status request
- **THEN** the response SHALL be `200 OK` with `state: "errored"`

#### Scenario: An errored turn returns state=errored

- **WHEN** a turn terminated with `agent_error`
- **AND** a client issues the status request
- **THEN** the response SHALL be `200 OK` with `state: "errored"` and `last_event_kind: "agent_error"`

#### Scenario: An unknown turn returns state=unknown

- **WHEN** a client issues the status request for a turn ID that was never recorded, has been garbage-collected, or belongs to a different conversation
- **THEN** the response SHALL be `200 OK` with `state: "unknown"` and `last_event_at: null`, `last_event_kind: null`
- **THEN** the response SHALL NOT be `404` — `unknown` is a valid response state, not an error
