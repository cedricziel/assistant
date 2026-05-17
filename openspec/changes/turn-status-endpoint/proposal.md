## Why

Cancellation of an in-flight stream — whether automatic (a watchdog timeout) or user-initiated (a "Skip" button) — currently happens with no authoritative truth. The client can only observe its end of the SSE socket: bytes arriving or not. That's insufficient signal:

- An SSE socket can stay open while the server has died on its end of the connection (NAT timeout, proxy timeout, server crash).
- Conversely, an SSE socket can be silent for legitimate reasons (long tool call, extended thinking, provider rate-limiting) while the server is healthily working.

The `sse-keepalive` change closes the false-positive case (silence ≠ dead). But the dual false-negative — a socket that LOOKS alive but isn't — remains uncovered, and any cancellation policy built on socket observation alone risks killing live turns or leaving zombie turns running on the server.

This proposal adds an explicit health-check endpoint that the client can probe to get the server's authoritative view of a turn's state. The client uses this probe (a) when it suspects a stream is dead despite keep-alive arriving, (b) when the user explicitly requests cancellation via the "Skip" affordance from the `chat-stream-progress-ux` change, and (c) on app resume after a long background period.

With the probe in hand, a coherent cancellation policy becomes possible: "If the user wants to skip, ask the server for the turn's state; if it's still running, send an explicit cancel; if it's completed, drain it normally; if it's errored or unknown, recover gracefully." No more guessing from byte-level heuristics.

## What Changes

- **New endpoint**: `GET /api/conversations/{conversationId}/turns/{turnId}/status`. Returns the server's authoritative state for a single turn:
  - `running` — turn is actively being processed (LLM call in flight, tool running, etc.).
  - `completed` — turn finished; final response is in the conversation.
  - `errored` — turn failed; an `agent_error` was emitted; recovery may be possible.
  - `unknown` — turn ID was never recorded or has been garbage-collected; client should refetch the conversation.
  - Response includes the turn's most recent event timestamp (so the client can detect "running but stuck") and the turn's most recent event kind (for UX correlation with the in-flight card).
- **New endpoint**: `POST /api/conversations/{conversationId}/turns/{turnId}/cancel`. Explicitly cancels an in-flight turn server-side. Returns `204 No Content` on success, `409 Conflict` if the turn already terminated (with the terminal state in the body for the client to reconcile), `404` if unknown. The implementation aborts the runtime task processing the turn and emits a final `agent_error` event on any open SSE stream so connected clients see consistent state.
- **OpenAPI**: Both endpoints documented in `openapi.json` and surfaced in the generated Dart client via `make dump-openapi && make generate-flutter-client`.
- **Client integration**:
  - On suspected stall (`chat-stream-progress-ux` card's stall threshold reached), the client polls `GET …/turns/{turnId}/status` to confirm the server's view before showing a "Skip" affordance.
  - The "Skip" affordance triggers `POST …/turns/{turnId}/cancel`. On success, the client advances the queue. On `409`, the client reconciles to the terminal state. On `404`, the client refetches the conversation.
  - On `AppLifecycleState.resumed`, if there was an interrupted in-flight turn, probe the status endpoint before deciding whether to reconnect, replay, or refetch.
- **Cancellation policy**: With the probe in place, the client's automatic watchdog stops cancelling streams. All cancellation flows go through the explicit `POST …/cancel` endpoint, gated by either user action or a confirmed-dead status probe. The 90-second byte heartbeat remains as a connection-level fallback (it triggers a probe rather than an outright stream close).

## Capabilities

### New Capabilities

- `turn-status-api`: The `GET /api/conversations/{id}/turns/{turnId}/status` and `POST /api/conversations/{id}/turns/{turnId}/cancel` endpoints, their response shapes, and their state-machine semantics (running / completed / errored / unknown plus the cancel-conflict edge cases).

### Modified Capabilities

- `chat-message-queue`: Add a requirement that automatic queue advancement SHALL only cancel an in-flight turn after a successful probe to `turn-status-api` confirms the turn is no longer running, or after the user explicitly triggers cancellation. Today's behaviour (and PR #809's behaviour) cancels based on byte-level observation alone.

## Impact

- **Backend code**:
  - New module `crates/web-ui/src/api/turns.rs` for the two handlers.
  - The runtime layer (`assistant-runtime` Orchestrator + turn-result bus) needs to expose a "lookup turn state by ID" surface and an "abort turn by ID" surface. Likely a small extension to the existing turn-tracking machinery — turn IDs are already in scope.
  - Routes registered in `crates/web-ui/src/api/mod.rs` and reflected in `openapi.json`.
- **OpenAPI**: ~2 new operations following the project's response-shape conventions (`*Response`, `*Detail`, snake_case fields, RFC 3339 timestamps).
- **Flutter client**:
  - Regenerate `app/packages/assistant_api/` via `make generate-flutter-client`.
  - New methods in `app/lib/api/api_client.dart` for `turnStatus(turnId)` and `cancelTurn(turnId)`.
  - `chat_provider.dart` integrates the probe into the suspected-stall flow and the AppLifecycleState.resumed flow.
- **Tests**:
  - Rust integration tests for both endpoints, including the state-machine edge cases.
  - Flutter unit tests for `chat_provider`'s probe + cancel paths.
  - End-to-end integration test exercising the full "queue → suspected stall → probe → user skip → cancel → queue advances" loop.
- **Dependencies**:
  - Best deployed after both `sse-keepalive` and `chat-stream-progress-ux` land. Without keep-alive, the stall probe trips on every slow tool call (noise). Without the UX, there's no surface to trigger the user-initiated cancel.
  - The original PR #809 fixes (2) (`isSending` guard) and (3) (`attemptReconnect` drains queue) are still independent useful fixes; they predate this change and should ship as their own small PR.
- **Risk**: The cancel endpoint introduces a new server-side abort mechanism. Needs careful handling to:
  - Avoid leaking in-flight LLM/tool processes (every cancel must propagate to the spawned tasks).
  - Avoid race conditions where the turn completes between status probe and cancel call (the `409` response handles this; client must reconcile).
  - Not leave partial output in the conversation that the user thinks is committed. The current behaviour (best-effort save of partial output as a `failed` message) should be preserved.
