# Turn Status & Cancel Endpoints

Two REST endpoints expose the authoritative server-side state of an
in-flight conversation turn, plus a way to cancel one. They live
under the conversations API and use the same bearer-token auth as
every other `/api/...` route.

## Endpoints

### `GET /api/conversations/{conversation_id}/turns/{turn_id}/status`

Returns the authoritative state of the named turn. The `turn_id`
matches the `run_id` emitted in the SSE `run_started` event (and in
the `X-Run-Id` response header on the streaming send endpoint).

**Response 200** — `TurnStatusResponse`:

```json
{
  "turn_id": "8c1f5e0e-…",
  "conversation_id": "0a2b…",
  "state": "running" | "completed" | "errored" | "unknown",
  "last_event_at": "2026-05-18T07:30:00Z" | null,
  "last_event_kind": "token" | "tool_result" | "done" | "agent_error" | … | null
}
```

`last_event_at` and `last_event_kind` are `null` only when the
state is `unknown` (no events recorded for that turn id).

### `POST /api/conversations/{conversation_id}/turns/{turn_id}/cancel`

Idempotent. Triggers the runtime's per-turn cancellation token and
returns the **current** turn status. The cancellation propagates
asynchronously — see [State machine](#state-machine).

Cancelling a turn that is already `completed`, `errored`, or
`unknown` is a no-op that simply returns the current status.

## State machine

```
            submit_turn registered                    POST .../cancel
                       │                                      │
                       ▼                                      ▼
    ┌───────────────────────────┐    cancel_turn        ┌────────────┐
    │         running           │ ──────────────────────│  errored   │
    │ (run_started, token,      │     fires token       │ (last:     │
    │  tool_result, thinking,   │ ────────────────►     │ agent_error│
    │  subagent_*, status)      │                       │ reason:    │
    └─────────────┬─────────────┘                       │ cancelled) │
                  │                                     └────────────┘
                  │ DoneEvent persisted
                  ▼
            ┌───────────┐
            │ completed │
            └───────────┘

  unknown ←── never seeded, GC'd, or conversation_id mismatch
```

Source of truth: the `conversation_events` table (`assistant-storage`).
The status endpoint derives state by reading the most recent event
for the turn:

- last event `done` → `completed`
- last event `agent_error` → `errored`
- no events for the turn id → `unknown`
- conversation id on the first row doesn't match the path → `unknown`
  (guards against turn-id collision across conversations)
- anything else → `running`

## Cancel semantics

When `cancel_turn` is invoked on a registered turn:

1. The orchestrator's per-turn `CancellationToken` is triggered.
2. The worker's `tokio::select!` aborts the in-flight future on the
   next yield point (i.e. between an LLM token batch, a tool call, or
   a subagent dispatch — never mid-syscall).
3. `submit_turn` (the inner waiter in the runtime) returns an error
   whose message contains `TURN_CANCELLED_MARKER` ("turn_cancelled").
4. The SSE handler in `crates/web-ui/src/api/messages.rs` catches that
   marker and emits a final terminal event:

   ```json
   event: agent_error
   data: {
     "reason": "cancelled",
     "message": "Turn cancelled by user",
     "partial_content": "…tokens streamed before cancel…"
   }
   ```

5. The event is persisted to `conversation_events` so subsequent
   status reads return `state=errored` with `last_event_kind=agent_error`.

### Partial-output preservation

The text streamed up to the moment of cancellation lives in
`partial_content` on the final `agent_error` event. Clients can
display it (truncated, with a "Cancelled" badge) rather than
discarding the partial assistant reply.

Note that **no persisted message row** is created for a cancelled
turn — the conversation's message list only contains the user prompt
and any assistant replies from prior turns. The partial text is
purely on the SSE event log (which has TTL — see
`docs/operations/conversation-events.md`).

## Auth and idempotency

- Both endpoints are mounted under `/api` and apply the
  conversation-scoped auth middleware (bearer token).
- Cancel is idempotent: re-issuing it on a terminal turn is a no-op.
  Re-issuing it on a still-running turn (after a previous cancel
  succeeded) is harmless — the token's `cancel()` is a no-op once
  triggered.

## Client integration

The Flutter app uses these endpoints in two flows:

1. **Stalled stream probe** — the 12 s initial-stall watchdog in
   `ChatNotifier` now calls `getTurnStatus` instead of cancelling the
   SSE stream on a heuristic. A `running` response keeps the sink
   open (server is healthy, the issue is iOS Dio buffering); any
   other state triggers the legacy recovery path.

2. **User-initiated Skip** — once the in-flight turn has been silent
   past `kTurnStallThreshold` (30 s), the progress card surfaces a
   "Skip" button. Tapping it calls `cancelTurn`. The fire-and-forget
   flow lets the SSE stream's terminal `agent_error` reconcile the
   UI state through the normal post-turn cleanup.

See `docs/development/chat-stream-probe.md` for the client-side
specifics.

## Observability

Each turn already has an OpenTelemetry span via
`crate::otel_spans::start_interface_root_context`. The
`submit.request_id` attribute is now equal to the SSE `run_id`
(post-stack PR #850) so traces, logs, and SSE events can be
correlated by a single id.

When a turn is cancelled, the worker log includes:

```
INFO cancel_turn: cancelled in-flight turn request_id=<run_id>
INFO submit_turn waiter state submit_state=cancelled
```

These are the two markers operators can grep for to count
cancellations in a given window.
