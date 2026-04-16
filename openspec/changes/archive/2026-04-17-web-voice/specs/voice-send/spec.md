## ADDED Requirements

### Requirement: User can record and send a voice message

The web UI SHALL provide a microphone button in the chat input row. When activated, the client records audio using the device microphone. On stop, the audio bytes are uploaded to the server, which transcribes them using the configured STT provider and injects the transcript as a user text message, returning a streaming SSE response identical to a normal text send.

#### Scenario: Successful voice send on web

- **WHEN** the user taps the microphone button, speaks, and taps it again to stop
- **THEN** the client uploads the WebM/Opus recording to `POST /api/conversations/{id}/voice`
- **THEN** the server transcribes the audio via the configured `TranscriptionProvider`
- **THEN** the transcript is sent as a user message and the assistant's streaming SSE response appears in the chat

#### Scenario: Successful voice send on macOS

- **WHEN** the user taps the microphone button, speaks, and taps it again to stop
- **THEN** the client uploads the M4A recording to `POST /api/conversations/{id}/voice`
- **THEN** the server transcribes and streams a response as above

#### Scenario: Transcription not configured

- **WHEN** the server has no `[transcription]` section in config
- **THEN** `POST /api/conversations/{id}/voice` returns HTTP 503
- **THEN** the Flutter client shows an error snackbar: "Voice messages require transcription to be configured on the server"

#### Scenario: No active conversation

- **WHEN** the user taps the microphone button before starting a conversation
- **THEN** the client creates a new conversation automatically (same behaviour as text send with no active conversation)

#### Scenario: Recording exceeds 2-minute limit

- **WHEN** the user records for more than 2 minutes
- **THEN** the client automatically stops recording and proceeds with the captured audio
- **THEN** a visible countdown timer is shown during recording

#### Scenario: Microphone permission denied

- **WHEN** the user taps the microphone button but denies microphone permission
- **THEN** the client shows an error snackbar: "Microphone access is required to send voice messages"
- **THEN** no audio is recorded or uploaded

### Requirement: Voice send endpoint accepts multipart audio

The server SHALL expose `POST /api/conversations/{id}/voice` accepting `multipart/form-data` with a single `audio` field containing raw audio bytes and a `Content-Type` header identifying the MIME type. The response is an SSE stream identical to `POST /api/conversations/{id}/messages`.

#### Scenario: Valid audio uploaded

- **WHEN** a client POSTs a valid audio file to the voice endpoint
- **THEN** the server transcribes the audio and streams the assistant response as SSE events

#### Scenario: Unsupported MIME type

- **WHEN** a client POSTs a file with an unrecognised MIME type (not in `AUDIO_MIME_PREFIXES`)
- **THEN** the server returns HTTP 415 Unsupported Media Type

#### Scenario: Audio exceeds 25 MB

- **WHEN** the uploaded audio file exceeds 25 MB
- **THEN** the server returns HTTP 413 Payload Too Large
