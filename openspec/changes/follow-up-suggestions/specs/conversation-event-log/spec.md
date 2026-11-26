## MODIFIED Requirements

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
