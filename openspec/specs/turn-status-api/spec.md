## Purpose

Two REST endpoints expose the authoritative server-side state of an
in-flight conversation turn and let clients cancel one. They live
under the conversations API and use the same bearer-token auth as
every other `/api/...` route.

The endpoints replace the prior heuristic where the client cancelled
its SSE stream on a byte-level watchdog. With the probe, the server is
the source of truth for `running` / `completed` / `errored` / `unknown`,
so clients can disambiguate iOS Dio buffering (server still running)
from a dead turn (server already terminal) without discarding in-flight
work.

## Requirements

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

The server SHALL expose `POST /api/conversations/{conversationId}/turns/{turnId}/cancel`. The endpoint triggers the runtime's per-turn `CancellationToken`, aborting any spawned LLM call, tool invocation, or subagent task. Propagation is asynchronous — the response reflects the **current** turn state, not the post-cancel terminal state. Clients poll `GET .../status` (or wait for the stream's `agent_error`) to observe the transition.

The endpoint is idempotent: it always returns `200 OK` with a `TurnStatusResponse` body in the same shape as the status read. Cancelling a turn that is already `completed`, `errored`, or `unknown` is a no-op; the response simply reports the current state.

#### Scenario: Cancelling a running turn returns 200 with current state

- **WHEN** a turn is `running` and a client issues `POST .../cancel`
- **THEN** the server SHALL trigger the runtime cancellation token (the worker's `tokio::select!` aborts on its next yield point)
- **THEN** the server SHALL respond `200 OK` with the current `TurnStatusResponse` (which may still report `state: "running"` if the response is built before the runtime has observed the cancel)
- **THEN** the runtime's `submit_turn` SHALL bail with an error containing the `turn_cancelled` marker
- **THEN** the SSE handler SHALL emit a final `agent_error` event with `{"reason": "cancelled", "message": "...", "partial_content": "..."}` and persist it to the event store
- **THEN** any partial assistant output already streamed for that turn SHALL appear in `partial_content` on that final event, so clients can display the truncated text rather than discarding it

#### Scenario: Cancelling an already-terminal turn is a no-op

- **WHEN** the turn has already reached `completed` or `errored` by the time the cancel request is processed
- **THEN** the server SHALL respond `200 OK` with the current `TurnStatusResponse` reflecting the actual terminal state
- **THEN** the response is indistinguishable from a fresh status read — clients reconcile to the terminal state without any special error handling

#### Scenario: Cancelling an unknown turn is a no-op

- **WHEN** the turn ID is not recorded (never started, GC'd, or belongs to a different conversation)
- **THEN** the server SHALL respond `200 OK` with `state: "unknown"`
- **THEN** the response SHALL NOT be `404` — same disambiguation rationale as the status endpoint

### Requirement: OpenAPI documentation for the new endpoints

Both endpoints SHALL appear in `openapi.json` with full schemas, operation IDs in snake_case (`get_turn_status`, `cancel_turn`), and standard `*Response` body types. The generated Dart client (`app/packages/assistant_api/`) SHALL expose typed methods.

#### Scenario: openapi.json is updated

- **WHEN** `make dump-openapi` is run after the endpoints land
- **THEN** the resulting `openapi.json` SHALL include both operations with full request/response schemas
- **WHEN** `make generate-flutter-client` is run
- **THEN** the regenerated `app/packages/assistant_api/` SHALL expose typed methods for both endpoints
