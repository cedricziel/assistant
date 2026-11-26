# Design — chat-stop-cancels-turn

## Context

Cancellation exists end to end on the server. `POST /api/conversations/{id}/turns/{run_id}/cancel` (`crates/web-ui/src/api/turns.rs:215-244`) calls `Orchestrator::cancel_turn` (`crates/runtime/src/orchestrator/mod.rs:408-424`), which fires the per-turn `CancellationToken`. The worker's `tokio::select!` drops the turn future (`worker.rs:280-289`), `submit_turn` bails with `TURN_CANCELLED_MARKER` (`worker.rs:840-853`), and the SSE task in `send_message` emits and persists a terminal frame (`crates/web-ui/src/api/messages.rs:376-471`):

```
event: agent_error
id: <seq>
data: {"reason":"cancelled","message":"Turn cancelled by user","partial_content":"<tokens so far>"}
```

The endpoint is idempotent and always returns `200` with a `TurnStatusResponse` (`running | completed | errored | unknown`). It never returns 404.

`agent_error` is in practice only ever terminal. `OrchestratorEvent::AgentError` is never constructed outside tests and projection code (`crates/runtime/src/orchestrator/stream_event.rs:61`, `crates/runtime/src/projection/mod.rs:116`), and the replay tail already treats it as terminal (`messages.rs:700`, `messages.rs:733`). Live and replayed data are the same JSON object (`messages.rs:466-470`, `messages.rs:706-709`).

### Defects this change fixes

1. **Stop is client-only.** Both Stop affordances call `cancelStream()` (`chat_screen.dart:541-543`, `:595-596`), which sets `_session.cancelled`, deletes the placeholder and flips `isSending` (`chat_provider.dart:274-287`). `requestCancelTurn()` (`chat_provider.dart:348-356`) reaches the server, but only from the stall Skip (`turn_progress_card.dart:123-136`).
2. **The queue stalls.** The `await for` in `_streamMessage` checks `_session.cancelled` only when the next event arrives (`chat_provider.dart:765-766`). `_drainQueue` stays blocked inside `_streamMessage` (`chat_provider.dart:574-579`) for as long as the server is silent, for example during a long tool call.
3. **Partial text is thrown away.** The client ignores `partial_content`. On the text path `_applyTurnEvent` returns `false` for `agent_error`, and neither the `DoneEvent` nor the `ErrorEvent` branch matches (`chat_provider.dart:785-805`). `AgentErrorEvent` holds the JSON body as an opaque string (`api/models/stream_event.dart:332-336`, `api/api_client.dart:778-779`).
4. **A cancelled reply is never persisted.** The runtime saves the assistant message only on the final answer (`mod.rs:1328-1340`). On reload the partial is gone, nothing says the turn was stopped, and the next LLM call sees the synthetic `"[An error occurred processing the previous message.]"` inserted after an orphaned user message (`crates/runtime/src/history.rs:181-197`).
5. **Cancels before claim are lost.** `submit_turn` removes the token when it bails (`worker.rs:840-853`). A worker that claims the request later finds no token, makes a fresh one (`worker.rs:199-210`) and runs the "cancelled" turn to completion.
6. **Voice cannot be cancelled.** `send_voice_message` uses `submit_turn` with no request id, run id, `X-Run-Id` or event log (`messages.rs:1026-1045`), and maps every error to `done` (`messages.rs:1078-1081`). `_streamVoiceMessage` never resets `_session.runId` (`chat_provider.dart:595`), so Stop during a voice turn would cancel the previous text turn.
7. **Cancel ignores the conversation in the path.** `cancel_turn` checks that the caller owns `conversation_id`, then calls `orchestrator.cancel_turn(turn_id)` without checking that the run belongs to that conversation (`turns.rs:223-236`). Only the status read afterwards checks it (`turns.rs:150-160`).

## Decisions

### D1. Keep the partial reply, and persist it on the server

**Decision.** On `agent_error {reason: "cancelled"}` the streaming placeholder becomes a regular assistant message. Its content is `partial_content`, falling back to the client's `streamingContent` if the server sent an empty string. Its status is `MessageStatus.stopped`. The user message becomes `ok`, unless the frame says `user_message_persisted: false` (D10). The server persists the same text as the turn's assistant message, marked `finish_reason = "cancelled"` (D12), and returns its id as `message_id`.

If the user stopped before any text streamed (during thinking or the first tool), the server persists the short placeholder row "_Stopped._" instead (D13), so the reply is never empty. The timeline entries the turn produced (tool calls, thinking) stay.

**Why.** The user saw the text, and every reference chat UI (assistant-ui's `ThreadPrimitive` cancel, ChatGPT, Claude.ai) keeps a stopped response. Persisting it matters as much as displaying it:

- Reload must show what the session showed, including that the reply was stopped (D12).
- The next LLM turn should see what the assistant actually said, not the sanitizer's text (`history.rs:181-197`). A user who stops a long answer and asks "shorter please" needs the model to know what it was shortening.
- `message_id` gives `message-feedback` and `message-branching` a stable handle.

**Alternatives.**

- _Discard the partial (today)._ Rejected for the reasons above.
- _Keep it in the client only._ Reload and session disagree, and the LLM context stays wrong. Rejected.
- _Mark it failed and offer Retry._ Wrong semantics: the user chose to stop. `message-branching` owns regenerate.

**Where the server writes it.** The runtime has already persisted the user message (`mod.rs:1560-1565`) and every tool-call and tool-result row (`dispatch.rs:99`, `dispatch.rs:252`). Only the final text is missing. The accumulated text lives in the web-ui SSE task (`full_text`, `messages.rs:215-258`), because the turn future was dropped. The runner passes it to a new `Orchestrator::persist_cancelled_reply(conversation_id, text, interface) -> Result<Option<Uuid>>`, modelled on `persist_error_recovery` (`history.rs:208`). It picks the turn index the same way `prepare_history` does (`mod.rs:1553-1554`). Storage access stays behind the orchestrator (interface-implementation rule).

**What the text contains.** `full_text` accumulates token frames from every LLM iteration of the turn, including narration before a tool call (`messages.rs:258`). The completed path persists only the final iteration's answer, and tool-call rows carry empty content (`dispatch.rs:60-69`). The persisted partial deliberately keeps all streamed text, because the client also accumulates tokens across iterations (`chat_provider.dart:165`) and the user saw all of it. A stopped reply can therefore read slightly differently from a completed one. That is accepted.

**Race guard.** If the worker saved the final answer just before the cancel won the `select!`, persisting again would duplicate it. The guard looks only at rows after the turn's user message. If any of them is an assistant **text** row, meaning non-empty `content` and no `tool_calls_json`, the method saves nothing and returns that row's id. Tool-call rows (assistant role, empty content, `tool_calls_json` set) and tool-result rows do not trip the guard. `message-branching` PR 7 makes this anchor-aware: a regenerate turn has no new user row, so the guard considers only rows written by this run, and the partial is branched under the anchor user message.

**Titling.** A conversation whose first turn is stopped still gets a title (D14): when `persist_cancelled_reply` saves a row, it also announces `turn.completed` so the title generator runs.

**Persistence failure.** If `persist_cancelled_reply` fails, the runner logs `warn!` and emits the frame without `message_id`. The client keeps the partial under a synthetic id, or removes the placeholder when nothing streamed.

### D2. One client entry point: `stopTurn()`

**Decision.** `cancelStream()` is deleted and `requestCancelTurn()` becomes a private `_requestServerCancel()`. The input-row `stop_button`, the bubble's `stop_action` and the stall card's Skip (`turn_progress_card.dart:135`) all call `ChatNotifier.stopTurn()`. No local-only stop path is left.

**State.** `ChatState` gains `isStopping` (bool). `ChatStreamSession` gains `stopRequested` (bool), a `detach` completer the stream loops race against (D5), and a `stopGrace` timer.

```
stopTurn()
  ├─ not sending / already stopping ─────────────► no-op
  ├─ isStopping = true; start stopGrace timer   (Stop button and bubble Stop → disabled "Stopping…")
  ├─ runId == null ──► stopRequested = true; wait for RunStartedEvent (D3)
  └─ runId != null ──► _requestServerCancel()  (D4)
                         │
       stream keeps flowing; tokens still render
                         │
   ┌─────────────────────┼──────────────────────────────┐
 agent_error{cancelled}  done (race lost)      stopGrace expires
   │                     │                              │
 stoppedTurnState()   completedTurnState()     D5 fallback
   └──────── isStopping = false; _streamMessage returns; queue drains ────────┘
```

**Why keep listening.** The terminal frame is the only authoritative answer (cancelled, finished first, or errored), and it carries the partial text and `message_id`. The `chat-message-queue` spec already asks for this (`openspec/specs/chat-message-queue/spec.md:116-121`).

**Alternative.** _Optimistic stop_: finalize as stopped immediately and reconcile later. The reconciliation can contradict the UI (a `done` with the full answer after "Stopped"), and it complicates the queue. The server reacts within one poll iteration of `submit_turn` (`worker.rs:840`), so a short "Stopping…" state is the honest choice.

### D3. Stop before the run id is known

`streamMessages` yields `RunStartedEvent` from `X-Run-Id` as soon as headers arrive (`api_client.dart:294-299`). Before that, nothing can be cancelled by id, yet the server may already have spawned the turn (`messages.rs:157-173`). A dropped SSE receiver only stops forwarding (`messages.rs:359-361`).

**Decision.** Set `stopRequested`. When `RunStartedEvent` arrives with `stopRequested` set, call `_requestServerCancel()` at once. If the request fails before any run id (transport error or `ErrorEvent`, `api_client.dart:289-291`), drop the placeholder, mark the user message `failed` so Retry is available, and show no error banner.

The `stopGrace` timer starts when `stopTurn()` is called, not when a cancel succeeds. If no run id has arrived when it fires, the client detaches locally (D5) with the error "Couldn't confirm the stop. The reply may still finish." and leaves the user message `ok`, because the server may have received it. The ceiling is the same 10 s whether or not the run id is known, so the user waits at most that long in the "Stopping…" state. This bounds Stop on a slow header, for example while voice transcription runs before any header is sent, and on servers without PR 3, where voice turns never get a run id.

**Alternative.** A client-generated run id sent with the request, so cancel works at any time. It changes the send API for a window of a few hundred milliseconds. Rejected for this change.

### D4. Cancel request failures and finished turns

`ApiClient.cancelTurn` returns `null` on any error (`api_client.dart:225-235`). `_requestServerCancel()`:

| Result                    | Meaning                                        | Client action                                                                                                                                                     |
| ------------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `running`                 | cancel fired, worker not yet reacted           | keep listening for the terminal frame                                                                                                                             |
| `errored`                 | already terminal (cancelled or failed)         | keep listening; the terminal frame is in flight or already consumed                                                                                               |
| `completed`               | the turn won the race                          | keep listening; `done` reconciles normally                                                                                                                        |
| `unknown`                 | no event log row for this run (GC'd, wrong id) | local detach (D5), then the queue-preserving refresh (D5)                                                                                                         |
| `null` (network/5xx/401…) | the server did not hear us                     | retry twice with `ChatStreamSession.retryDelay` (1 s, 2 s). If all fail: local detach, error "Couldn't reach the server to stop this reply. It may still finish." |

In every row except `unknown` and the final `null`, the stream decides the outcome.

### D5. Bounded wait, local detach and refresh

**Bounded wait.** When `stopGrace` (10 s from `stopTurn()`) expires without a terminal frame and a run id is known, the client probes `turnStatus` once:

- `running`: the server is wedged. Detach, keep the client's `streamingContent` as a `stopped` message, and show "The server hasn't confirmed the stop. The reply may still finish."
- `completed`, `errored` or `unknown`: the server has an outcome the stream failed to deliver. Detach, then run the queue-preserving refresh, so the UI shows what the server persisted (the full answer if the turn won the race) and not a truncated "Stopped" message.

**Local detach** completes `_session.detach`. The `await for` loops in `_streamMessage`, `_streamVoiceMessage` and `_replayRun` consume `source` through a small `_untilDetached` wrapper that closes the stream when `detach` completes. Detaching never waits for another server event, which fixes defect 2. `isSending` and `isStopping` clear.

**Queue-preserving refresh.** This is today's `_recoverStalledStream` step 2 (`chat_provider.dart:968-995`), extracted as `_refreshFromServer(conversationId)`. It fetches the conversation, does nothing if a newer turn is already sending (`if (latest.isSending) return;`), and otherwise replaces the messages while keeping `pendingQueue`. `loadConversation` is **not** used, because it builds a fresh `ChatState` and drops `pendingQueue` (`chat_provider.dart:309-311`).

**The `_session.cancelled` flag is removed.** Stall recovery (`_recoverStalledStream`, `chat_provider.dart:957-958`) has the same "wait for the next event" problem, so it completes `detach` too. All readers of the flag (`chat_provider.dart:509`, `:652`, `:766`, `:865`, `:919-941`) switch to `detach.isCompleted`. The existing stall-recovery tests are ported, including "stalled recovery does not clobber a concurrently-started stream".

**Alternative.** Keep the flag and rely on the next event. That is defect 2. Rejected.

### D6. Queue

`_drainQueue` awaits `_streamMessage` (`chat_provider.dart:574-579`). Because `_streamMessage` now returns only after reconciliation (terminal frame or detach), the queue drains right after the stopped turn settles and never overlaps it. Stop leaves `pendingQueue` untouched.

### D7. Parsing `agent_error`

`AgentErrorEvent` gains `reason` (`AgentErrorReason.cancelled | error`), `message`, `partialContent`, `messageId?` and `userMessagePersisted` (default `true`). Parsing accepts the JSON object (live and replay) and, for safety, a non-JSON body as `reason: error, message: <body>`. An unknown `reason` maps to `error`. All three stream paths route it through one `_onAgentError` handler:

- `cancelled` → `stoppedTurnState(...)` (new, in `chat_turn_transitions.dart`, beside `abortedTurnState` / `completedTurnState`). With `userMessagePersisted == false` the user message becomes `failed` (retryable) and no error banner is shown.
- `error` → `abortedTurnState(failedUserMsgId: …, error: message)`, the `ErrorEvent` semantics.

`_replayRun` returns `true` for `cancelled` so `_handleStreamFailure` stops retrying. Today it marks the user message failed and returns `false` (`chat_provider.dart:525-531`).

A cancelled frame that arrives without the user pressing Stop (another tab, `/stop`, server shutdown) reconciles the same way.

### D8. Tool calls left pending

`ToolCallStatus` gains `cancelled`. `stoppedTurnState` flips every `pending` record of the turn to `cancelled` and completes active timeline entries (`completeActiveTimelineEntries`, `chat_timeline_reducers.dart:48-55`). `streaming_timeline_entry.dart:300` renders a muted block icon. `parseToolStatus` (`chat_history_mapping.dart:7-15`) maps `"cancelled"`. The dead `tool_call_chip.dart` and `timeline_section.dart` are not touched: PR 4 stacks on `chat-element-library` PR 2, which deletes them.

### D9. Voice turns join the run lifecycle

`send_voice_message` (`messages.rs:931-1110`) differs from the text path in three ways: no request id, no persisted frames, and every error maps to `done` (`messages.rs:1078-1081`).

**Decision.** Voice moves onto the shared turn runner that `message-branching` PR 5 extracts from `send_message` (`stream_turn_to_sse`, taking `RunOptions { interactive, suggestions, push, anchor }`). This change does not extract its own. Voice calls it with `interactive: false`, `suggestions: false`, `push: true` (voice pushes today, `messages.rs:1102-1108`) and no anchor. It gets a run id, `X-Run-Id`, replay and the `agent_error` terminals, and `transcript` is emitted and persisted as the first frame after `run_started`.

On the client, `ApiClient.sendVoiceMessage` yields `RunStartedEvent` from the `X-Run-Id` header, as `streamMessages` does (`api_client.dart:294-299`). Without that, iOS Dio body buffering could hold back the `run_started` body frame until the stream closes. `_streamVoiceMessage` calls `_session.beginTurn()` and captures `RunStartedEvent`, which fixes the stale run id (`chat_provider.dart:595`). Both client changes are harmless before PR 3, because the header is simply absent and D3's grace bounds the Stop.

### D10. Turns cancelled before a worker claims them

**Runtime.** `turn_cancellations` entries gain a `claimed` flag. The worker's claim path takes the write lock once. If the request id is in a bounded `cancelled_requests` tombstone (a `VecDeque` + `HashSet`, capacity 256, pruned FIFO), the worker acks the message and `continue`s. It does **not** touch `token_sinks` or `extension_registrations`. Both are keyed by conversation id (`worker.rs:38-44`, `worker.rs:186-188`), so by the time a stale request is claimed they may belong to a newer turn of the same conversation. Otherwise the worker marks the entry `claimed` and proceeds. `cancel_turn` takes the same lock. For an unclaimed entry it cancels the token and adds the id to the tombstone. `submit_turn`'s cancel branch reads `claimed` when it removes the entry. For an unclaimed turn the error carries a second marker, `TURN_CANCELLED_BEFORE_START_MARKER`, beside `TURN_CANCELLED_MARKER`. The `try_read` token lookup (`worker.rs:203-209`) goes away, because claim now takes the lock with `.write().await`. The lock is held only for map operations, while today's fallback silently produces an uncancellable turn.

A new `Orchestrator::release_token_sink(conversation_id, &Sender)` removes the conversation's sink only if `Sender::same_channel` matches. A runner can then release its own sink without touching a newer turn's.

**Runner.** The SSE loop no longer waits for `event_rx` to close before looking at the turn result (`messages.rs:224-391`). It `select!`s on both, with `biased` priority for `event_rx`. On `Ok(result)` it keeps draining `event_rx` until it closes, as today, because buffered events can trail the result. On a cancelled `Err` it drains what is already buffered (`try_recv`), calls `release_token_sink` with its own sender clone, and emits the terminal frame at once. A cancel before claim therefore ends the stream immediately, even when every worker is busy. When the error carries the before-start marker, the frame adds `user_message_persisted: false`, because `prepare_history` never ran (`mod.rs:1560-1565`) and the user's message is not in the history.

**Alternatives.**

- _Keep the token after cancel and have the worker check `is_cancelled()`._ It leaks tokens for requests never claimed (restart, redelivery). The tombstone has a fixed ceiling.
- _Let the worker release the sink on a tombstoned claim (the first draft)._ It removes a newer turn's sink, and the new turn then runs without streaming. Rejected.
- _Key sinks by request id._ Every interface's `register_token_sink` call would change. Out of scope.

**Client outcome.** The `user_message_persisted: false` frame is what lets the client show the user message as failed with Retry (D7). Nothing is persisted for such a turn: no assistant row, no "_Stopped._" placeholder (D13), no title announcement (D14). The user message was never saved, so Retry sends it again as a new turn. Showing it as failed with Retry, and not silently dropping it, keeps the user's text from vanishing.

**Scope limit.** `turn_cancellations` is in-process. With an external bus and workers in another process, cancel only reaches same-process workers, as today.

### D11. Cancel checks the run's conversation

`cancel_turn` loads the run's event-log rows (as `turn_status` does, `turns.rs:130-160`) and calls `orchestrator.cancel_turn` only when the first row's `conversation_id` matches the path. Otherwise it returns the `unknown` status without firing anything. Without this check, a caller who owns conversation A can cancel any turn in the space whose run UUID they know, and after this change that cancel also persists the other turn's partial.

### D12. A persisted `finish_reason` marks stopped replies

**Decision.** `messages` gains a nullable `finish_reason TEXT` column through a new space.db migration named `message_finish_reason` (the stack plan assigns its number; the next free one on main is 046, which `message-branching` PR 1 already claims). The only value written today is `"cancelled"`. `NULL` means an ordinary message. `Message` gains `finish_reason: Option<MessageFinishReason>` (`#[serde(default)]`, `None` in `Message::new`), both conversation stores round-trip it, and `MessageSummary` exposes it as an optional `finish_reason` string. The history mapper turns `"cancelled"` into `MessageStatus.stopped`, so a reload shows the same "Stopped" label as the live session.

**Why.** Showing "Stopped" only until the next reload makes the session and the history disagree, and a stopped answer looks like a complete one afterwards. A column is the smallest durable marker. It also lets `message-branching` and `message-feedback` tell a stopped reply from a finished one without parsing content.

**Alternatives.**

- _A `stopped_at` timestamp._ Carries no meaning beyond the boolean, and a second reason would need a second column.
- _Detect the "_Stopped._" text._ Fragile and does not cover partials.
- _Store it in a metadata JSON blob._ `tool-result-renderers` S3 adds metadata columns for tool rows only. A typed column is simpler to query and project.

**Name.** `finish_reason` also appears as an LLM response field (`crates/core/src/llm/types.rs`). The message column is a different concept: why the persisted reply ended, not why the model stopped. The Rust type is `MessageFinishReason`, so the two do not mix.

**Lane.** The column touches the `Message` struct lane that `message-branching` PR 1 opens, followed by `tool-result-renderers` S3 and `message-feedback` PR 3. The storage PR rebases on whichever has merged.

### D13. Stopping before any text persists a short "_Stopped._" row

**Decision.** When a turn is cancelled after the user message was saved and no text streamed, `persist_cancelled_reply` saves the assistant row "_Stopped._" with `finish_reason = "cancelled"` and returns its id. The terminal frame then carries `partial_content: "_Stopped._"` and `message_id`, so the client renders exactly what was persisted, with no special case. A whitespace-only partial counts as empty.

**Why.** Without a row, reload shows an unanswered user message, the next LLM call sees the sanitizer's synthetic error text (`history.rs:181-197`), and the live session shows a stopped state that no longer exists. One visible row makes the live session, the reload and the LLM context agree. The text stays short and visible, and the model reads it as "the previous reply was stopped".

**Not persisted.** A turn cancelled before a worker claims it never saved the user message (D10), so there is nothing to answer: no row. If persisting fails, the frame omits `message_id` and the client removes the placeholder.

**Trade-off.** The stopped bubble shows "_Stopped._" and the "Stopped" label together. That is slightly redundant, and it keeps every stopped reply self-describing in clients that do not map `finish_reason`.

### D14. A stopped first turn still gets a title

**Decision.** `persist_cancelled_reply` announces `turn.completed` (the `TurnResult` payload: conversation id, the persisted text, turn number, `message_id`, `had_errors: false`, the caller's interface) on the bus when it saved a row and the turn's user message has non-empty text. It reuses the worker's `announce_completed_turn` path, and it does not publish `turn.result` or webhooks. The title generator already runs on any `turn.completed` of an untitled conversation (`title_generator.rs`, `is_eligible`), so the usual eligibility settings still apply and no title code changes.

**Why.** A user who stops the first answer and walks away would otherwise leave an untitled conversation in the list, which is the common case for stopped runaway answers.

**Not announced.** When the race guard finds the worker's own final answer (the worker already announced it), when the turn was cancelled before start (no user row), or when the user message has no text (attachments only).

## Cross-change coordination

- **Shared turn runner.** `message-branching` PR 5 extracts it once (`stream_turn_to_sse` with `RunOptions`). PR 1b, PR 2 and PR 3 of this change edit the runner, not `send_message`.
- **`finish_reason` lane.** The storage PR (PR 2a) follows `message-branching` PR 1 and shares the `Message` struct lane with `tool-result-renderers` S3 and `message-feedback` PR 3. `message-feedback` ships after `message-branching` D5, so it can read `finish_reason`.
- **`persist_cancelled_reply` and branching.** `message-branching` PR 7 makes it anchor-aware (D1) and depends on PR 2 here.
- **Client stream loop.** Order: this change's PR 4, then PR 5b, then `follow-up-suggestions` PR 7, then `message-branching` PR 12. Each later change keeps `_untilDetached` around its stream source.
- **`chat-element-library`.** PR 4 here stacks on its PR 2. Its PR 4 (moves the enums) comes after PR 4 here. Its PR 9 and PR 11 (progress card, composer) come after PR 5a here, and its Composer gains an `isStopping` input.
- **Archive order.** This change archives before `follow-up-suggestions`, which archives before `tool-approval-prompts`. All three modify `turn-status-api`.

## Risks / Trade-offs

- **Stopping feels slower than today.** Today Stop is instant but false. Mitigation: the button changes state at once, tokens keep rendering, and there is a 10 s ceiling (D5).
- **Duplicate assistant rows on a cancel/finish race.** Handled by the guard (D1) and tested.
- **Migration on the `messages` table.** One nullable `ALTER TABLE ... ADD COLUMN`, no backfill. Existing rows read as `NULL`, which is the ordinary state.
- **The claim/save window.** A cancel that lands after the worker marked the request `claimed` but before `prepare_history` saved the user message reports `user_message_persisted: true`, although nothing was saved. The window is one `await`. Accepted.
- **A stale sink can linger.** After a cancel before claim, the runner releases its own sink. If the runner itself died first, the stale `Sender` stays in `token_sinks` until the conversation's next streamed turn overwrites it. Sends into a closed channel are already ignored.
- **A stopped reply that had no text reads "_Stopped._" to the model.** Intended (D13). The model sees an explicit, short assistant turn instead of the sanitizer's error text.
- **Siblings touch the same enums and stream loop.** Ordered in tasks.md.
