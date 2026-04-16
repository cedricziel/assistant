## MODIFIED Requirements

### Requirement: Voice send endpoint accepts multipart audio

The server SHALL expose `POST /api/conversations/{id}/voice` accepting `multipart/form-data` with a single `audio` field containing raw audio bytes and a `Content-Type` header identifying the MIME type. The response is an SSE stream. The stream SHALL include:

- `event: transcript` carrying the transcribed user text before any token events
- `event: token` events with incremental assistant tokens
- `event: done` with the final assistant reply AND the DB UUID of the saved assistant message in a `message_id` field

The done event payload SHALL be: `{"role":"assistant","content":"<text>","message_id":"<uuid>"}`.

#### Scenario: Valid audio uploaded

- **WHEN** a client POSTs a valid audio file to the voice endpoint
- **THEN** the server transcribes the audio and emits `event: transcript` with the spoken text
- **AND** the server streams the assistant response as SSE token events
- **AND** the server emits a final `event: done` containing the assistant reply and the UUID of the persisted message

#### Scenario: Unsupported MIME type

- **WHEN** a client POSTs a file with an unrecognised MIME type (not `audio/*`)
- **THEN** the server returns HTTP 400

#### Scenario: Audio exceeds 25 MB

- **WHEN** the uploaded audio file exceeds 25 MB
- **THEN** the server returns HTTP 400

### Requirement: Text message send done event includes message ID

The server SHALL include the DB UUID of the saved assistant message in the `done` SSE event for `POST /api/conversations/{id}/messages` as well, so the client can identify the message for on-demand TTS.

#### Scenario: Text message done event carries message_id

- **WHEN** a client sends a text message and the assistant completes its reply
- **THEN** the `event: done` payload contains `message_id` set to the UUID of the persisted assistant message

#### Scenario: Client uses message_id for audio playback

- **WHEN** the client receives a `done` event with a `message_id` field
- **THEN** `ChatMessage.id` is set to that UUID
- **AND** `GET /api/messages/{message_id}/audio` returns HTTP 200 when TTS is configured
