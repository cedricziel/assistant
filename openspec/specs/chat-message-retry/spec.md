## MODIFIED Requirements

### Requirement: Retrying a failed message re-sends it

Tapping Retry on a failed message SHALL re-enqueue the original message text through the same `sendMessage` path. The retry SHALL preserve the `toolCalls` list from the original message on the new attempt's assistant response placeholder (starting empty, as with any new stream).

#### Scenario: Retry re-sends the message

- **WHEN** the user taps Retry on a failed message
- **THEN** the failed status indicator is cleared
- **AND** the message text is sent via `sendMessage`, entering the queue or sending immediately

#### Scenario: Retry respects the queue

- **WHEN** the user taps Retry while another response is in-flight
- **THEN** the retried message is added to the pending queue and drains after the current response

#### Scenario: Retry starts with empty tool calls

- **WHEN** a failed message is retried
- **THEN** the new assistant response placeholder starts with an empty `toolCalls` list
- **AND** tool call chips accumulate fresh from the new stream
