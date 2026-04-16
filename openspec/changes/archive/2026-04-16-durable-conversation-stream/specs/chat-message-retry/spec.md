## MODIFIED Requirements

### Requirement: Failed user messages persist with a Retry affordance

When a user message fails to send (network error or server error), the message SHALL remain visible in the conversation with a `failed` status indicator and an inline Retry action.

If the failure occurred after a `run_started` event was received (i.e., a `run_id` is available), the Retry action SHALL attempt to reconnect to the existing run via the event log replay endpoint before re-sending the message. If the run is expired or not found, it SHALL fall back to re-sending the message as a new request.

#### Scenario: Failed message stays in the list

- **WHEN** the stream returns an `ErrorEvent` or throws an exception after a user message was added
- **THEN** the user message bubble SHALL remain visible with a visual failed indicator
- **AND** the assistant streaming placeholder SHALL be removed

#### Scenario: Retry button appears on failed message

- **WHEN** a user message has `status == failed`
- **THEN** a Retry action SHALL be rendered inside or below the message bubble

#### Scenario: Retrying with a known run_id attempts replay first

- **WHEN** the user taps Retry on a failed message
- **AND** a `run_id` was captured before the failure
- **THEN** the client SHALL call `GET /api/conversations/{id}/runs/{run_id}/events/stream?since={last_seq}`
- **AND** if the server returns events, the UI SHALL resume streaming from the last known sequence

#### Scenario: Retrying with an expired or unknown run falls back to re-send

- **WHEN** the user taps Retry on a failed message
- **AND** the replay endpoint returns `404` or `410`
- **THEN** the client SHALL re-send the original message text via `POST /api/conversations/{id}/messages`
- **AND** the new run_id SHALL be stored for future reconnects
