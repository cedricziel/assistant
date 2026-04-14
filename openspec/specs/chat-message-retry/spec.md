## ADDED Requirements

### Requirement: Failed user messages persist with a Retry affordance

When a user message fails to send (network error or server error), the message SHALL remain visible in the conversation with a `failed` status indicator and an inline Retry action.

#### Scenario: Failed message stays in the list

- **WHEN** the stream returns an `ErrorEvent` or throws an exception after a user message was added
- **THEN** the user message bubble remains visible with a visual failed indicator (e.g., error icon or red tint)
- **AND** the assistant streaming placeholder is removed

#### Scenario: Retry button appears on failed message

- **WHEN** a user message has `status == failed`
- **THEN** a Retry action is rendered inside or below the message bubble

### Requirement: Retrying a failed message re-sends it

Tapping Retry on a failed message SHALL re-enqueue the original message text through the same `sendMessage` path.

#### Scenario: Retry re-sends the message

- **WHEN** the user taps Retry on a failed message
- **THEN** the failed status indicator is cleared
- **AND** the message text is sent via `sendMessage`, entering the queue or sending immediately

#### Scenario: Retry respects the queue

- **WHEN** the user taps Retry while another response is in-flight
- **THEN** the retried message is added to the pending queue and drains after the current response

### Requirement: Successful messages carry a confirmed status

User messages that are successfully acknowledged by the server SHALL transition from `sending` to `ok` status when the corresponding `DoneEvent` is received.

#### Scenario: Message marked ok after DoneEvent

- **WHEN** the assistant stream for a user message completes with a `DoneEvent`
- **THEN** the corresponding user message bubble transitions to `status == ok`
- **AND** no error indicator is shown
