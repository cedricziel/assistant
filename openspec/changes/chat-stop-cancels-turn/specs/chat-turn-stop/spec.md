## ADDED Requirements

### Requirement: Every Stop affordance cancels the server turn

The Flutter client SHALL expose a single stop entry point, `ChatNotifier.stopTurn()`. The input-row stop button (`stop_button`), the streaming bubble's Stop action (`stop_action`) and the stall card's Skip button SHALL all call it. `stopTurn()` SHALL request server-side cancellation through `POST /api/conversations/{id}/turns/{run_id}/cancel`. The client SHALL NOT keep any stop path that only stops listening locally without asking the server to cancel.

#### Scenario: Input-row Stop cancels on the server

- **WHEN** a turn is streaming with run id `R` and the user taps `stop_button`
- **THEN** the client SHALL issue `POST .../turns/R/cancel` exactly once
- **THEN** the client SHALL keep consuming the turn's SSE stream until a terminal frame arrives

#### Scenario: Bubble Stop and stall Skip use the same path

- **WHEN** the user activates the streaming bubble's `stop_action`, or the stall card's Skip
- **THEN** the client SHALL behave exactly as if `stop_button` had been tapped

#### Scenario: Stop is a no-op when nothing is streaming

- **WHEN** `stopTurn()` is called while no turn is in flight
- **THEN** no cancel request SHALL be sent and the chat state SHALL be unchanged

### Requirement: Stopping state is visible and Stop is not re-entrant

While a stop is in progress, the client SHALL expose `ChatState.isStopping == true`. The input row SHALL show the stop affordance as disabled with a "Stopping…" semantic label, and the streaming bubble's `stop_action` SHALL be disabled. Further `stopTurn()` calls SHALL be ignored until the turn reconciles. Tokens that arrive while stopping SHALL still render.

#### Scenario: Double tap sends one cancel

- **WHEN** the user taps Stop twice in quick succession
- **THEN** exactly one cancel request SHALL be sent
- **THEN** `stop_button` SHALL be disabled until the turn reconciles

#### Scenario: Bubble Stop is disabled while stopping

- **WHEN** a stop is in progress
- **THEN** the streaming bubble's `stop_action` SHALL be disabled and activating it SHALL send nothing

#### Scenario: Stopping clears on reconciliation

- **WHEN** the terminal frame for a stopping turn is processed
- **THEN** `isStopping` SHALL be `false` and the input row SHALL show the send button again

### Requirement: Cancelled terminal frame keeps the partial reply as a stopped message

When the client receives `agent_error` with `reason: "cancelled"` on the live, voice or replay stream, it SHALL turn the streaming placeholder into an assistant message whose content is the frame's `partial_content`, falling back to the client's accumulated streaming content when `partial_content` is empty. When the user stopped before any text streamed, the server persists the placeholder "_Stopped._" and sends it as `partial_content`, so the client shows it like any other partial. The message SHALL have status `MessageStatus.stopped`. Its id SHALL be the frame's `message_id` when present. The originating user message SHALL be marked `ok`. Tool calls of that turn still `pending` SHALL become `ToolCallStatus.cancelled`, and active timeline entries SHALL be completed. No error banner SHALL be shown. The same reconciliation SHALL apply whoever triggered the cancel.

#### Scenario: Partial text survives Stop

- **WHEN** the assistant has streamed "The capital of France" and the user taps Stop
- **AND** the server sends `agent_error {"reason":"cancelled","partial_content":"The capital of France","message_id":"M"}`
- **THEN** the conversation SHALL show an assistant message with id `M`, content "The capital of France" and status `stopped`
- **THEN** the message's action row SHALL show a "Stopped" label in place of the Stop action

#### Scenario: Stopped before any text

- **WHEN** no tokens were streamed and the cancelled frame carries `partial_content: "_Stopped._"` and `message_id: "M"`
- **THEN** the conversation SHALL show an assistant message with id `M`, content "_Stopped._" and status `stopped`
- **THEN** tool-call and thinking entries already shown for the turn SHALL remain, with pending tool calls marked `cancelled`

#### Scenario: Nothing streamed and nothing persisted

- **WHEN** the cancelled frame has empty `partial_content`, no `message_id` and no tokens were streamed (the server could not persist the placeholder)
- **THEN** the streaming placeholder SHALL be removed and no assistant message SHALL be added
- **THEN** tool-call and thinking entries already shown for the turn SHALL remain, with pending tool calls marked `cancelled`

#### Scenario: Replay of a cancelled run reconciles instead of retrying

- **WHEN** the stream dropped and the event-log replay delivers `agent_error {"reason":"cancelled",...}`
- **THEN** the turn SHALL reconcile as stopped
- **THEN** the replay retry loop SHALL NOT retry that run again

#### Scenario: Turn cancelled before it started

- **WHEN** the client receives `agent_error {"reason":"cancelled","user_message_persisted":false,...}`
- **THEN** the placeholder SHALL be removed and no stopped assistant message SHALL be shown
- **THEN** the user message SHALL be marked `failed` with Retry available, and `ChatState.error` SHALL be null

#### Scenario: Partial reply that the server could not persist

- **WHEN** the cancelled frame carries non-empty `partial_content` and no `message_id`
- **THEN** the client SHALL keep the partial as a `stopped` assistant message under a client-generated id

#### Scenario: Stop while retrying a dropped stream

- **WHEN** the stream dropped and the client is waiting to replay the run, and the user taps Stop
- **THEN** the client SHALL issue the server cancel for that run
- **THEN** the client SHALL NOT start another replay attempt, and SHALL reconcile from the replayed terminal frame or by local detach

#### Scenario: Errored terminal frame fails the turn

- **WHEN** the client receives `agent_error` with `reason: "error"`, an unknown reason, or a non-JSON body
- **THEN** the placeholder SHALL be removed, the user message SHALL be marked `failed`, and the frame's message SHALL be surfaced as the chat error

### Requirement: A stopped reply still shows Stopped after reload

When the client loads conversation history, it SHALL map a message whose `finish_reason` is `"cancelled"` to `MessageStatus.stopped`. Such a message SHALL render the same "Stopped" label and normal Copy action as in the live session. Messages without a `finish_reason` SHALL map as before.

#### Scenario: Reload keeps the Stopped label

- **WHEN** a reply was stopped and the user reloads the conversation
- **AND** the history contains that assistant message with `finish_reason: "cancelled"`
- **THEN** the message SHALL have status `stopped` and show the "Stopped" label

#### Scenario: Stopped before any text, after reload

- **WHEN** the user stopped before any text streamed and reloads the conversation
- **THEN** the history SHALL show an assistant message "_Stopped._" with status `stopped` after the user message

#### Scenario: Ordinary replies are unchanged

- **WHEN** the history contains an assistant message without `finish_reason`
- **THEN** it SHALL map to status `ok` and show no "Stopped" label

### Requirement: The finish-vs-stop race resolves to whichever terminal frame the server sent

If the turn finishes before the cancel takes effect, the client SHALL reconcile from the `done` frame exactly as for an uncancelled turn and SHALL NOT mark the message stopped.

#### Scenario: Done wins the race

- **WHEN** the user taps Stop and the cancel response reports `state: "completed"`
- **AND** the stream delivers `done`
- **THEN** the final assistant message SHALL have status `ok` and the full `done.content`

### Requirement: Stop before the run id is known is deferred, not dropped

When `stopTurn()` is called before the client has received the turn's run id, the client SHALL remember the request and SHALL send the cancel as soon as `RunStartedEvent` arrives. If the stream fails before any run id arrives, the client SHALL drop the placeholder, mark the user message `failed` (retryable), and SHALL NOT show an error banner. If no run id arrives within `ChatStreamSession.stopGrace` of the Stop, the client SHALL detach locally.

#### Scenario: Stop during request setup

- **WHEN** the user taps Stop after sending but before `RunStartedEvent`
- **AND** `RunStartedEvent("R")` then arrives
- **THEN** the client SHALL issue `POST .../turns/R/cancel` immediately

#### Scenario: Stop then the request fails

- **WHEN** the user taps Stop before any run id, and the send request then fails
- **THEN** the user message SHALL be `failed` with Retry available
- **THEN** `ChatState.error` SHALL be null

#### Scenario: No run id ever arrives

- **WHEN** the user taps Stop before any run id and no `RunStartedEvent` arrives within 10 seconds
- **THEN** the client SHALL detach locally, leave the user message `ok`, and show "Couldn't confirm the stop. The reply may still finish."
- **THEN** `isSending` and `isStopping` SHALL be false and the queue SHALL resume

### Requirement: Cancel request failures retry, then detach honestly

If the cancel request fails (`ApiClient.cancelTurn` returns `null`), the client SHALL retry it twice with the stream session's backoff (1 s, 2 s). If every attempt fails, the client SHALL detach locally (see next requirement) and surface the error "Couldn't reach the server to stop this reply. It may still finish."

#### Scenario: Transient cancel failure recovers

- **WHEN** the first cancel request fails and the second returns `running`
- **THEN** the client SHALL keep waiting for the terminal frame and SHALL NOT surface an error

#### Scenario: Cancel unreachable

- **WHEN** all three cancel attempts fail
- **THEN** the placeholder SHALL become a `stopped` message holding the streamed content so far (or be removed if empty)
- **THEN** the error above SHALL be shown, `isSending` and `isStopping` SHALL be false, and the queue SHALL resume

### Requirement: Stopping has a bounded wait

If no terminal frame arrives within `ChatStreamSession.stopGrace` (10 seconds) of the Stop and the run id is known, the client SHALL probe `GET .../turns/{run_id}/status` once and detach locally. If the probe reports `running`, the client SHALL keep the streamed content as a `stopped` message and show "The server hasn't confirmed the stop. The reply may still finish." For any other probe result, and when the cancel response itself reports `state: "unknown"`, the client SHALL detach and refresh the conversation from the server. Local detach SHALL end the turn's stream subscription immediately, without waiting for another server event.

The refresh SHALL preserve `pendingQueue` and SHALL NOT overwrite the state when a newer turn has started sending in the meantime.

#### Scenario: Server never sends the terminal frame

- **WHEN** the cancel request returned `running`, no event arrives for 10 seconds, and the status probe reports `running`
- **THEN** the client SHALL finalize the placeholder as `stopped`, show the unconfirmed-stop error, clear `isSending` and `isStopping`, and let the queue resume

#### Scenario: Turn completed but its done frame never arrived

- **WHEN** no terminal frame arrives within 10 seconds of the Stop and the status probe reports `completed`
- **THEN** the client SHALL detach and refresh the conversation
- **THEN** the conversation SHALL show the server-persisted full answer, not a `stopped` message

#### Scenario: Unknown turn with queued messages

- **WHEN** the cancel response reports `state: "unknown"` while two messages are queued
- **THEN** the client SHALL detach and refresh the conversation
- **THEN** both queued messages SHALL remain in `pendingQueue`
- **THEN** if the next queued turn has started sending before the refresh completes, the refresh SHALL NOT overwrite its state

### Requirement: Voice turns are stoppable

Voice turns SHALL use the same stop flow as text turns. The client SHALL reset per-turn session state (including the run id) when a voice turn starts. It SHALL capture the voice turn's run id from the `X-Run-Id` response header as soon as headers arrive, and from the `run_started` frame, so Stop never targets a previous turn's run id.

#### Scenario: Stop during a voice reply

- **WHEN** the user sends a voice message, the server responds with run id `V`, and the user taps Stop
- **THEN** the client SHALL issue `POST .../turns/V/cancel` and reconcile from the voice stream's terminal frame

#### Scenario: Stale run id is never reused

- **WHEN** a text turn with run id `T` completed and a voice turn then starts
- **AND** the user taps Stop before the voice turn's run id arrives
- **THEN** no cancel request for `T` SHALL be sent
