## ADDED Requirements

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
