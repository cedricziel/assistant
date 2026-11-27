## Purpose

Durable per-conversation event log that backs run replay and turn status.

## Requirements

### Requirement: Orchestrator emits a run_started event with a run ID

When the orchestrator begins processing a message, it SHALL generate a UUID `run_id` and emit a `run_started` event as the first SSE event on `POST /api/conversations/{id}/messages`, before any token events.

#### Scenario: run_started is the first event in the SSE stream

- **WHEN** a client calls `POST /api/conversations/{id}/messages`
- **THEN** the first SSE event SHALL have `event: run_started` and `data` containing `{"run_id": "<uuid>"}`
- **AND** all subsequent events in the stream belong to that run_id

#### Scenario: run_started is also the first event in the durable log

- **WHEN** the orchestrator emits a run_started event
- **THEN** that event SHALL be persisted to `conversation_events` with `sequence = 0` and `event_type = "run_started"`

---

### Requirement: All orchestrator events are persisted to the conversation event log

Every event emitted during an orchestrator run (token, status, tool_call, tool_result, done, error) SHALL be written to the `conversation_events` table with a monotonically increasing sequence number scoped to the run.

#### Scenario: Token events are logged

- **WHEN** the LLM emits a token during an orchestrator run
- **THEN** the token SHALL be persisted as a `conversation_events` row with `event_type = "token"` and `payload = {"token": "<text>"}`

#### Scenario: Done event is logged

- **WHEN** the orchestrator emits a `done` event
- **THEN** the event SHALL be persisted with `event_type = "done"` and `payload` containing `{"role": "assistant", "content": "<full_text>", "id": "<message_id>"}`

#### Scenario: Error event is logged

- **WHEN** the orchestrator emits an `agent_error` event
- **THEN** the event SHALL be persisted with `event_type = "error"` and `payload` containing `{"message": "<error_text>"}`

---

### Requirement: Events expire after a configurable TTL

Persisted events SHALL have an `expires_at` timestamp set to `created_at + TTL` (default 24 hours). A background task SHALL delete expired rows.

#### Scenario: Events are pruned after TTL

- **WHEN** a conversation event row has `expires_at` in the past
- **THEN** the background pruning task SHALL delete it on its next run (at most 1 hour after expiry)

#### Scenario: Pruning does not affect active runs

- **WHEN** the pruning task runs
- **THEN** rows whose `expires_at` is in the future SHALL NOT be deleted

---

### Requirement: Client can replay a run's events from a sequence cursor

`GET /api/conversations/{id}/runs/{run_id}/events/stream` SHALL return a `text/event-stream`. The stream SHALL first replay all stored events with `sequence >= since` (default 0). If the run is still active, it SHALL then tail live events. It SHALL close when the run finishes, meaning its live broadcast channel closes, not when it sees a `done` or `agent_error` event. It SHALL also stop tailing, and release its live subscription, as soon as the client disconnects. Events persisted after `done` (such as `suggestions`) are part of the run and SHALL be replayed and tailed like any other event. A run with no stored events and no live registration, whether it never existed or its events were pruned by the TTL, SHALL return `404`.

#### Scenario: Client reconnects mid-run and replays missed tokens

- **WHEN** a client calls `GET /api/conversations/{id}/runs/{run_id}/events/stream?since=42`
- **AND** the run is still active and has emitted events with sequence 0–60
- **THEN** the server SHALL replay events 42–60 immediately
- **AND** THEN stream subsequent live events as they are emitted
- **AND** close the SSE stream when the run finishes

#### Scenario: Tail continues past done until the run finishes

- **GIVEN** a run that has emitted `done` but is still generating suggestions
- **WHEN** a client replays it
- **THEN** the server SHALL replay through `done`, then deliver the live `suggestions` event, then close

#### Scenario: Client replays a completed run

- **WHEN** a client calls `GET /api/conversations/{id}/runs/{run_id}/events/stream`
- **AND** the run has already finished
- **THEN** the server SHALL replay all events from sequence 0, including any `suggestions` event after `done`, and close the stream
- **AND** the response SHALL complete without waiting for new events

#### Scenario: Disconnected client releases its subscription

- **GIVEN** a client tailing an active run
- **WHEN** the client closes the connection
- **THEN** the server SHALL drop that client's live subscription without waiting for the next event

#### Scenario: Unknown run_id returns 404

- **WHEN** a client calls `GET /api/conversations/{id}/runs/{unknown_id}/events/stream`
- **THEN** the server SHALL return `404 Not Found` with `{"error": "run not found"}`

#### Scenario: Expired run returns 404

- **WHEN** a client calls `GET /api/conversations/{id}/runs/{run_id}/events/stream`
- **AND** all events for that run have been pruned (TTL elapsed) and the run is not active
- **THEN** the server SHALL return `404 Not Found` with `{"error": "run not found"}`

### Requirement: run_id is surfaced as a response header fallback

The `POST /api/conversations/{id}/messages` response SHALL include an `X-Run-Id` header containing the run_id, in addition to the `run_started` SSE event, so clients that crash before receiving the first event can still identify the run.

#### Scenario: X-Run-Id header is present on the streaming response

- **WHEN** a client calls `POST /api/conversations/{id}/messages`
- **THEN** the HTTP response headers SHALL include `X-Run-Id: <uuid>` before the body begins streaming
