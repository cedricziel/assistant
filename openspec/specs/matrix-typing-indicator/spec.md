## ADDED Requirements

### Requirement: MatrixClient exposes a send_typing method

The `MatrixClient` struct SHALL expose an async `send_typing(room_id: &str, typing: bool) -> Result<()>` method using `PUT /_matrix/client/v3/rooms/{roomId}/typing/{userId}`.

#### Scenario: send_typing true sends start signal with timeout

- **WHEN** `send_typing(room_id, true)` is called
- **THEN** a PUT request is sent with body `{ "typing": true, "timeout": 30000 }` to the correct endpoint

#### Scenario: send_typing false sends stop signal

- **WHEN** `send_typing(room_id, false)` is called
- **THEN** a PUT request is sent with body `{ "typing": false }`

### Requirement: MatrixClient exposes reaction add and redact methods

The `MatrixClient` struct SHALL expose `add_reaction(room_id: &str, event_id: &str, emoji: &str) -> Result<String>` (returns the reaction event ID) and `redact_event(room_id: &str, event_id: &str) -> Result<()>` methods.

#### Scenario: add_reaction sends m.reaction event

- **WHEN** `add_reaction(room_id, event_id, "⏳")` is called
- **THEN** a PUT request is sent to `PUT /rooms/{roomId}/send/m.reaction/{txnId}` with the correct relates_to body

#### Scenario: add_reaction returns the new event ID

- **WHEN** the homeserver responds with `{ "event_id": "$abc" }`
- **THEN** `add_reaction` returns `Ok("$abc".to_string())`

#### Scenario: redact_event removes a reaction

- **WHEN** `redact_event(room_id, reaction_event_id)` is called
- **THEN** a PUT request is sent to `PUT /rooms/{roomId}/redact/{eventId}/{txnId}`

### Requirement: Matrix adapter adds hourglass reaction on message receipt

In `on_message_received`, the Matrix adapter SHALL add an ⏳ reaction to the inbound message and store the returned reaction event ID for later redaction.

#### Scenario: Hourglass reaction event ID stored for redaction

- **WHEN** `on_message_received` adds the ⏳ reaction successfully
- **THEN** the returned event ID is stored in adapter state keyed by the message's platform ID

#### Scenario: Reaction failure is silently ignored

- **WHEN** the homeserver rejects the reaction
- **THEN** the error is logged at `debug!` level and no event ID is stored

### Requirement: Matrix adapter removes hourglass and sends typing on turn start

In `on_turn_start`, the Matrix adapter SHALL redact the stored ⏳ reaction event and then call `send_typing(room_id, true)`.

#### Scenario: Hourglass redacted before typing sent

- **WHEN** `on_turn_start` is called and a stored reaction event ID exists
- **THEN** `redact_event` is called first, then `send_typing(room_id, true)`

#### Scenario: Missing stored event ID does not block typing

- **WHEN** no reaction event ID is stored for the conversation
- **THEN** `send_typing(room_id, true)` is still called

#### Scenario: Turn start failure does not fail the turn

- **WHEN** either redact or send_typing fails
- **THEN** the error is logged at `debug!` level and `on_turn_start` returns `Ok(())`

### Requirement: Matrix adapter clears typing on turn end

In `on_turn_success` and `on_turn_error`, the Matrix adapter SHALL call `send_typing(room_id, false)`.

#### Scenario: Typing cleared on turn success

- **WHEN** `on_turn_success` is called
- **THEN** `send_typing(room_id, false)` is called (best-effort)

#### Scenario: Typing cleared on turn error

- **WHEN** `on_turn_error` is called
- **THEN** `send_typing(room_id, false)` is called (best-effort)
