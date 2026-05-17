## ADDED Requirements

### Requirement: Queue advancement uses authoritative server probe, not byte-level heuristics

When the client suspects an in-flight stream has stalled (silence past the byte heartbeat or the stall threshold in `chat-stream-progress-ux`), it SHALL probe `GET /api/conversations/{id}/turns/{turnId}/status` from `turn-status-api` before deciding whether to advance the queue. The pre-`turn-status-api` behaviour — cancelling streams based on byte-level observation alone — is no longer permitted.

#### Scenario: Stall probe returns running → client waits, no queue advancement

- **WHEN** the client's stall threshold is crossed and the queue is non-empty
- **AND** the client probes `.../status` and the response is `state: "running"`
- **THEN** the client SHALL NOT cancel the stream
- **THEN** the client SHALL NOT advance the queue
- **THEN** the in-flight stream continues to be consumed

#### Scenario: Stall probe returns completed → client reconciles, queue advances

- **WHEN** the client probes `.../status` and the response is `state: "completed"`
- **AND** the client's local view still shows `isSending == true` for this turn
- **THEN** the client SHALL fetch the conversation to acquire the final message and reconcile state
- **THEN** the client SHALL advance the queue normally

#### Scenario: User-initiated Skip uses POST cancel, not implicit cancellation

- **WHEN** the user invokes the "Skip" affordance from `chat-stream-progress-ux`
- **THEN** the client SHALL `POST .../cancel`
- **THEN** the client SHALL advance the queue only after receiving a `204` response or after reconciling a `409` response to the actual terminal state
- **THEN** no implicit / heuristic cancellation path SHALL remain in the client
