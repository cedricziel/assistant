## MODIFIED Requirements

### Requirement: POST /api/conversations/{id}/turns/{turnId}/cancel cancels an in-flight turn

The server SHALL expose `POST /api/conversations/{conversationId}/turns/{turnId}/cancel`. The endpoint triggers the runtime's per-turn `CancellationToken`, aborting any spawned LLM call, tool invocation, or subagent task. Propagation is asynchronous — the response reflects the **current** turn state, not the post-cancel terminal state. Clients poll `GET .../status` (or wait for the stream's `agent_error`) to observe the transition.

The endpoint is idempotent: it always returns `200 OK` with a `TurnStatusResponse` body in the same shape as the status read. Cancelling a turn that is already `completed`, `errored`, or `unknown` is a no-op; the response simply reports the current state.

The server SHALL fire the cancellation only when the run's recorded conversation matches `{conversationId}`. A cancel SHALL take effect even when no worker has claimed the turn request yet: a worker that later claims a cancelled request SHALL acknowledge and skip it without running the turn, and SHALL NOT release any other turn's token sink. When a cancelled turn has produced non-empty assistant text, the runtime SHALL persist that text as the turn's assistant message, so conversation history and the next LLM turn contain it.

#### Scenario: Cancelling a running turn returns 200 with current state

- **WHEN** a turn is `running` and a client issues `POST .../cancel`
- **THEN** the server SHALL trigger the runtime cancellation token (the worker's `tokio::select!` aborts on its next yield point)
- **THEN** the server SHALL respond `200 OK` with the current `TurnStatusResponse` (which may still report `state: "running"` if the response is built before the runtime has observed the cancel)
- **THEN** the runtime's `submit_turn` SHALL bail with an error containing the `turn_cancelled` marker
- **THEN** the SSE handler SHALL emit a final `agent_error` event with `{"reason": "cancelled", "message": "...", "partial_content": "..."}` and persist it to the event store
- **THEN** any partial assistant output already streamed for that turn SHALL appear in `partial_content` on that final event, so clients can display the truncated text rather than discarding it

#### Scenario: Cancelled partial reply is persisted

- **WHEN** a turn is cancelled after streaming the text "Partial answer"
- **THEN** the conversation history SHALL contain an assistant message with content "Partial answer" after the turn's user message
- **THEN** the final `agent_error` event SHALL carry `message_id` set to that message's UUID

#### Scenario: Cancelled turn with no text persists nothing

- **WHEN** a turn is cancelled before any token was streamed
- **THEN** no assistant message SHALL be persisted for the turn
- **THEN** the final `agent_error` event SHALL omit `message_id`

#### Scenario: Partial reply after a tool call is persisted

- **WHEN** a turn's history after its user message holds only a tool-call row and its tool-result row, and the turn is cancelled after streaming further text
- **THEN** the server SHALL persist the streamed text as a new assistant message and return its id as `message_id`

#### Scenario: Persisting the partial fails

- **WHEN** a turn is cancelled with streamed text and persisting the partial fails
- **THEN** the final `agent_error` event SHALL still be emitted, with `partial_content` and without `message_id`

#### Scenario: Cancel racing a just-persisted final answer does not duplicate it

- **WHEN** the runtime persisted the turn's final assistant text message (non-empty content, no tool calls) and the cancel is then observed
- **THEN** the server SHALL NOT persist a second assistant message for that turn
- **THEN** `message_id` on the final event SHALL be the already-persisted message's UUID

#### Scenario: Cancel before a worker claims the request

- **WHEN** `submit_turn` has published a turn request, no worker has claimed it, and the turn is cancelled
- **THEN** a worker that later claims the request SHALL acknowledge it without invoking the LLM or any tool
- **THEN** the SSE stream SHALL end at once with `agent_error {"reason":"cancelled","user_message_persisted":false,...}`, without waiting for a worker to claim the request

#### Scenario: A stale cancelled request does not steal a newer turn's stream

- **WHEN** a turn in conversation C is cancelled before claim, a second turn is then sent in C, and a worker claims the stale request before the second one
- **THEN** the worker SHALL skip the stale request without touching C's token sink
- **THEN** the second turn SHALL stream its tokens to its own SSE response

#### Scenario: Cancelling an already-terminal turn is a no-op

- **WHEN** the turn has already reached `completed` or `errored` by the time the cancel request is processed
- **THEN** the server SHALL respond `200 OK` with the current `TurnStatusResponse` reflecting the actual terminal state
- **THEN** the response is indistinguishable from a fresh status read — clients reconcile to the terminal state without any special error handling

#### Scenario: Cancel with a run id from another conversation

- **WHEN** the caller issues `POST /api/conversations/A/turns/R/cancel` and run `R` belongs to conversation B
- **THEN** the server SHALL NOT fire `R`'s cancellation token
- **THEN** the server SHALL respond `200 OK` with `state: "unknown"`

#### Scenario: Cancelling an unknown turn is a no-op

- **WHEN** the turn ID is not recorded (never started, GC'd, or belongs to a different conversation)
- **THEN** the server SHALL respond `200 OK` with `state: "unknown"`
- **THEN** the response SHALL NOT be `404` — same disambiguation rationale as the status endpoint

## ADDED Requirements

### Requirement: Voice turns share the run lifecycle

`POST /api/conversations/{id}/voice` SHALL run its turn under a server-generated run id that is passed to the orchestrator as the request id. It SHALL return that id in the `X-Run-Id` response header, emit `run_started` as the first SSE frame, persist every frame to the conversation event log with sequence ids, and end with the same terminal frames as `POST /api/conversations/{id}/messages`: `done` on success, `agent_error` with `reason: "cancelled"` or `reason: "error"` otherwise. Voice turns SHALL therefore be cancellable through the cancel endpoint and replayable through `GET .../runs/{run_id}/events/stream`.

#### Scenario: Voice turn can be cancelled

- **WHEN** a voice turn with run id `V` is streaming and a client issues `POST .../turns/V/cancel`
- **THEN** the voice SSE stream SHALL end with `agent_error {"reason":"cancelled", ...}`

#### Scenario: Voice turn errors are not reported as done

- **WHEN** the orchestrator returns an error for a voice turn
- **THEN** the voice SSE stream SHALL end with `agent_error {"reason":"error", ...}` rather than `done`

#### Scenario: Voice transcript still precedes the reply

- **WHEN** a voice turn starts
- **THEN** the SSE stream SHALL emit `run_started` and then `transcript` before any token frame
