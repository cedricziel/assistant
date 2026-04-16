## ADDED Requirements

### Requirement: Nextcloud adapter adds hourglass reaction on message receipt

In `on_message_received`, the Nextcloud adapter SHALL add an ⏳ reaction to the inbound message via the Talk reactions API (best-effort).

#### Scenario: Hourglass reaction added on receipt

- **WHEN** a Nextcloud Talk message with a known message ID is received
- **THEN** the adapter sends `POST /ocs/v2.php/apps/spreed/api/v1/reaction/{token}/{messageId}` with body `{ "reaction": "⏳" }` and `OCS-APIRequest: true` header

#### Scenario: Reaction failure is silently ignored

- **WHEN** the server returns any error (including 404 for unsupported versions)
- **THEN** the error is logged at `debug!` level and the message proceeds

### Requirement: Nextcloud adapter removes hourglass and sends typing on turn start

In `on_turn_start`, the Nextcloud adapter SHALL remove the ⏳ reaction and then send a typing notification.

#### Scenario: Hourglass removed before typing sent

- **WHEN** `on_turn_start` is called with a known message ID
- **THEN** `DELETE /ocs/v2.php/apps/spreed/api/v1/reaction/{token}/{messageId}` is called with `{ "reaction": "⏳" }`, then typing is sent

#### Scenario: Turn start failures do not fail the turn

- **WHEN** reaction remove or typing call fails
- **THEN** the error is logged at `debug!` level and `on_turn_start` returns `Ok(())`

### Requirement: Nextcloud adapter sends typing notification on turn start

The Nextcloud adapter SHALL send a typing-started notification using the Talk typing API.

#### Scenario: Typing notification sent on turn start (Talk ≥ 17)

- **WHEN** `on_turn_start` is called with a valid conversation token
- **THEN** `POST /ocs/v2.php/apps/spreed/api/v1/chat/{token}/typing` is sent with body `{ "typing": true }` and `OCS-APIRequest: true`

#### Scenario: 404 response silently ignored

- **WHEN** the server returns 404 (Talk < 17)
- **THEN** the error is logged at `debug!` and the turn proceeds normally

### Requirement: Nextcloud adapter clears typing on turn end

In `on_turn_success` and `on_turn_error`, the Nextcloud adapter SHALL send a typing-stopped notification.

#### Scenario: Typing cleared on turn success

- **WHEN** `on_turn_success` is called
- **THEN** `POST /ocs/v2.php/apps/spreed/api/v1/chat/{token}/typing` is sent with body `{ "typing": false }` (best-effort)

#### Scenario: Typing cleared on turn error

- **WHEN** `on_turn_error` is called
- **THEN** `POST /ocs/v2.php/apps/spreed/api/v1/chat/{token}/typing` is sent with body `{ "typing": false }` (best-effort)

### Requirement: Nextcloud typing and reaction calls use OCS API headers

All Nextcloud Talk API requests SHALL include `OCS-APIRequest: true` and valid bot authentication credentials.

#### Scenario: OCS header present on all requests

- **WHEN** any typing or reaction API call is made
- **THEN** the request includes `OCS-APIRequest: true` and bot auth headers
