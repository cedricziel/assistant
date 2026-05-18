# Chat Stream Probe (Client)

How the Flutter app decides whether to keep an SSE stream attached
or fall back to a refresh. Reference: `openspec/changes/turn-status-endpoint/`.

## TL;DR

The 12 s initial-stall watchdog (`_initialStallTimeout`) **no longer
cancels the SSE stream on a heuristic**. It calls
`GET .../turns/{run_id}/status` and decides based on the server's
authoritative reply.

```
        12 s of no UI-visible event
                  │
                  ▼
          [run_id known?]
              ├── no ──▶ legacy `_recoverStalledStream`
              │          (refetch conversation, surface error)
              ▼
       [api.turnStatus()]
              │
   ┌──────────┼─────────────┬──────────────┐
   ▼          ▼             ▼              ▼
running   completed     errored        unknown / null (probe failed)
   │          │             │              │
   │          ▼             ▼              ▼
   │     legacy           legacy        legacy
   │   recovery (refetch, mark failed)
   │
   ▼
keep sink open
(90 s byte-watchdog still guards genuinely dead streams)
```

## Two watchdogs, two layers

| Watchdog               | Layer       | Threshold | What fires                                                       |
| ---------------------- | ----------- | --------- | ---------------------------------------------------------------- |
| `_initialStallTimeout` | App         | 12 s      | One probe via `GET .../status`. Decides per server state.        |
| `withHeartbeatTimeout` | Byte stream | 90 s      | Closes the stream when no bytes (including SSE comments) arrive. |

The byte-level watchdog still exists. It's the final safety net for
streams where the keep-alive comments have stopped arriving (the
connection is genuinely dead at the transport layer). It does NOT
make decisions about partial work — when it fires, the stream is
already gone.

## Why the probe matters

Before this change, the 12 s watchdog cancelled the stream
unconditionally. That worked for the iOS Dio buffering case (Dio
holds the chunked body until the connection closes) but discarded
in-flight work whenever the server was actually still processing —
e.g. a slow tool call genuinely taking 20 s.

The probe disambiguates:

- **iOS Dio buffering** → server says `running` → app keeps waiting,
  user sees the progress card with stall messaging.
- **Genuinely dead turn** → server says `completed`/`errored`/`unknown`
  → app falls back to recovery (refetch conversation).
- **Server unreachable** → probe returns `null` → app falls back to
  recovery (same as the legacy path).

## When the probe is skipped

The probe needs a `run_id` to address the right turn. If the
`RunStartedEvent` hasn't fired by 12 s (header buffered alongside
the body), `_currentRunId` is null and the probe is skipped — legacy
recovery runs instead. This preserves the safety net for the
worst-case Dio path where headers are also buffered.

## User-initiated Skip

The progress card transitions to a stalled state after
`kTurnStallThreshold` (30 s). In that state a "Skip" button appears
(behind `kSkipButtonEnabled`, default `true`). Tapping it calls
`api.cancelTurn(conversationId, runId)` via
`ChatNotifier.requestCancelTurn()`.

The button is fire-and-forget: the SSE stream eventually emits a
final `agent_error` with `reason: "cancelled"`, which the notifier's
existing terminal-event handling translates into normal post-turn
cleanup (placeholder cleared, user bubble marked ok). The user gets
instant visual feedback that the tap registered; reconciliation
arrives over the stream a tick later.

## Server side

See `docs/operations/turn-status-endpoint.md` for the endpoint
contract, state machine, and observability hooks.

## Test surface

- `app/test/unit/chat/chat_provider_test.dart` group
  `ChatNotifier — stall probe routing` — pins every state's
  behaviour (running keeps sink open, others recover, no-run-id
  bypasses).
- `app/test/widget/features/chat/turn_progress_card_test.dart` —
  Skip button visibility and tap-invokes-cancel.
- `crates/web-ui/src/api/turns.rs::tests` — server-side state
  derivation, cancel idempotency, cross-conversation guard.
- `crates/runtime/src/orchestrator/tests/bus_worker.rs` —
  `cancel_turn_*` tests for the runtime registry + abort
  propagation.
