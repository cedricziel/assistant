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

The server fires the cancellation only when the run's recorded
conversation matches `{conversation_id}`. Naming another conversation's
run id returns `state: "unknown"` and leaves that run untouched.

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
The status endpoint derives state from which terminal rows exist for the
turn, not from the kind of the last row:

- any `agent_error` row → `errored`
- else any `done` row → `completed`
- else → `running`
- no events for the turn id → `unknown`
- conversation id on the first row doesn't match the path → `unknown`
  (guards against turn-id collision across conversations)

Terminal rows take precedence over non-terminal ones. A state added later for
a non-terminal condition (such as `awaiting_input`) ranks below `completed`
and above `running`: `errored` > `completed` > `awaiting_input` > `running`.

`last_event_kind` and `last_event_at` describe the most recent row, and
`done` is no longer always the last frame of a run. A completed turn can end
with `last_event_kind: "suggestions"`, for example. The status stays
`completed`. The POST stream still closes at `done`; the run's replay stream
stays open until the run finishes (see
[ADR-0024](../adr/adr-0024-run-bounds-replay-stream.md)).

## Voice turns

`POST /api/conversations/{id}/voice` runs on the same turn runner as the
text send endpoint, so a voice turn has a run id, is cancellable and is
replayable:

- The response carries the run id in `X-Run-Id`.
- The SSE stream opens with `run_started` (sequence 0), then `transcript`
  (sequence 1, `{"role":"user","content":"…"}`), then the reply frames.
- Every frame, `transcript` included, is persisted under the run id, so
  `GET .../runs/{run_id}/events/stream?since=0` replays the whole turn.
- `POST .../turns/{run_id}/cancel` ends the stream with
  `agent_error {"reason":"cancelled"}`. A failed turn ends with
  `agent_error {"reason":"error"}`, never `done`.
- Voice turns offer no follow-up suggestions.

The run id only reaches the client once transcription has finished, because
the response is not sent before then.

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

### Cancel before a worker claims the turn

A cancel also takes effect while the turn request is still waiting for
a worker (every worker busy). The worker that later claims the request
acknowledges and skips it without running the LLM or any tool, and
touches no other turn's token sink. The SSE stream does not wait for
that: the runner races the turn result against the event stream and
ends at once with

```json
event: agent_error
data: {
  "reason": "cancelled",
  "message": "Turn cancelled by user",
  "partial_content": "",
  "user_message_persisted": false
}
```

`user_message_persisted: false` is present only for this case. The turn
never started, so its user message was never saved; clients show the
message as failed with a Retry action. The field is omitted for a turn
cancelled after it started.

### Partial-output preservation

A turn cancelled after it started keeps what the user saw. The runtime
persists the text streamed up to the cancel as the turn's assistant
message, marked `finish_reason: "cancelled"`. When nothing had streamed
yet, it persists the placeholder `_Stopped._` instead, so history and the
next LLM turn always contain a reply. The final `agent_error` event
carries that text as `partial_content` and the row's id as `message_id`:

```json
event: agent_error
data: {
  "reason": "cancelled",
  "message": "Turn cancelled by user",
  "partial_content": "…tokens streamed before cancel…",
  "message_id": "…assistant message uuid…"
}
```

- If the worker had already saved the turn's final answer when the cancel
  was observed, nothing is saved twice: `message_id` names that answer and
  its `finish_reason` stays unset.
- If saving fails, the event is still emitted with `partial_content` and
  without `message_id`.
- A turn cancelled before it started persists no assistant row, not even
  the placeholder (see above).
- A stopped first turn is still auto-titled: the runtime announces
  `turn.completed` for the saved reply, so the title generator runs under
  its usual eligibility rules.
- `GET /api/conversations/{id}` lists the row with `finish_reason:
  "cancelled"`.

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
