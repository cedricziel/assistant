## Purpose

A queue mechanism that accepts user messages typed while a previous
response is streaming, drains them sequentially after each response
completes, and surfaces the queue visibly so the user retains agency
(see what's queued, remove entries) without interrupting the in-flight
turn. The `Stop` action cancels the current response without clearing
the queue.

## Requirements

### Requirement: Input remains enabled during streaming

The chat input field and send button SHALL remain interactive (enabled) at all times, including while the assistant is generating a response.

#### Scenario: User types while assistant is responding

- **WHEN** the assistant is streaming a response
- **THEN** the message input field is enabled and accepts text input

#### Scenario: User submits while assistant is responding

- **WHEN** the user submits a message while `isSending` is true
- **THEN** the message is added to the pending queue without interrupting the current stream

### Requirement: Pending queue drains sequentially

New user messages submitted while a response is in-flight SHALL be held in a `pendingQueue` and sent automatically, one at a time, after each response completes.

#### Scenario: Single queued message drains after current response

- **WHEN** the user sends a message while the assistant is responding
- **AND** the current response reaches `DoneEvent`
- **THEN** the queued message is sent automatically without user action

#### Scenario: Multiple queued messages drain in order

- **WHEN** the user sends three messages while the assistant is responding
- **AND** the current and subsequent responses complete
- **THEN** all three queued messages are sent in the order they were submitted

#### Scenario: Queue depth is visible in the UI

- **WHEN** there is at least one message in the pending queue
- **THEN** the UI displays a queue depth indicator (e.g., badge or label) showing the count of pending messages

### Requirement: Stop cancels only the current in-flight response

When the user taps Stop, the current streaming response SHALL be cancelled; queued messages SHALL be preserved and continue to drain.

#### Scenario: Stop does not clear the pending queue

- **WHEN** the user has queued messages and taps Stop
- **THEN** the current stream is cancelled
- **AND** the pending queue is unchanged and draining resumes immediately

### Requirement: Queued messages render as visible ghost bubbles

Messages sitting in `ChatState.pendingQueue` SHALL be rendered in the conversation list, in send order, as visually-distinguished "ghost" bubbles. The user SHALL be able to see exactly what is queued without being able to confuse it with a committed message.

#### Scenario: Two queued messages render as ghost bubbles

- **WHEN** the user types and sends a message while a previous turn is streaming, and then types and sends a second message before the first turn completes
- **THEN** both queued messages SHALL appear as ghost bubbles in the conversation list, in the order they were sent
- **THEN** each ghost bubble SHALL display the message text and a "Queued" badge
- **THEN** the ghost bubbles SHALL visually differ from committed user bubbles (muted text colour, no avatar, "Queued" badge)

#### Scenario: Ghost bubble promotes to a normal bubble when its turn starts

- **WHEN** the in-flight turn completes and the next queued message starts streaming
- **THEN** that queued message's ghost bubble SHALL be removed and a normal user bubble SHALL appear in its place at the same position in the list
- **THEN** the visual transition SHALL NOT cause the conversation to jump or reorder

### Requirement: Queued messages are removable before they send

The user SHALL be able to remove a queued message from `pendingQueue` before it begins streaming, without affecting the in-flight turn.

#### Scenario: Long-press to remove on mobile

- **WHEN** the user long-presses a ghost bubble for a queued message
- **THEN** an action sheet SHALL appear with a "Remove from queue" action (and a Cancel)
- **WHEN** the user selects "Remove from queue"
- **THEN** the entry SHALL be removed from `pendingQueue` and the ghost bubble SHALL disappear within one frame

#### Scenario: Right-click to remove on desktop

- **WHEN** the user right-clicks a ghost bubble for a queued message on web or macOS
- **THEN** a context menu SHALL appear with a "Remove from queue" action
- **WHEN** the user selects it
- **THEN** the entry SHALL be removed from `pendingQueue` and the ghost bubble SHALL disappear

#### Scenario: Cannot remove the actively-streaming turn

- **WHEN** the user attempts to remove the message that is currently streaming (its bubble is no longer a ghost — it has been promoted to a normal user bubble)
- **THEN** the remove affordance SHALL NOT appear on that bubble; the in-flight turn can only be ended via the `turn-status-api` cancel surface or by waiting for it to complete

### Requirement: Queue advancement uses authoritative server probe, not byte-level heuristics

When the client suspects an in-flight stream has stalled (silence past the byte heartbeat or the stall threshold in `chat-stream-progress-ux`), it SHALL probe `GET /api/conversations/{id}/turns/{turnId}/status` from `turn-status-api` before deciding whether to advance the queue. The pre-`turn-status-api` behaviour — cancelling streams based on byte-level observation alone — is no longer permitted as the primary trigger; the byte-level watchdog remains only as a final safety net for transport-dead streams.

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
- **THEN** the client SHALL `POST .../cancel` via the `turn-status-api` surface
- **THEN** no implicit / heuristic cancellation path SHALL remain in the client as the primary trigger
- **THEN** the SSE stream's terminal `agent_error` event with `reason: "cancelled"` SHALL drive the client's normal post-turn cleanup and queue advancement, rather than special-casing the cancel response status code
