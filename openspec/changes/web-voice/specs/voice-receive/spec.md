## ADDED Requirements

### Requirement: Assistant messages have a play button for on-demand TTS

The web UI SHALL display a play (▶) button on every assistant message bubble when a TTS provider is configured on the server. Tapping the button fetches synthesized audio from `GET /api/messages/{msg_id}/audio` and plays it. The button toggles to a stop (■) icon while audio is playing.

#### Scenario: User taps play on an assistant message

- **WHEN** the user taps ▶ on an assistant message
- **THEN** the client fetches `GET /api/messages/{msg_id}/audio`
- **THEN** the server synthesizes the message text via the configured `TtsProvider` and returns mp3 bytes
- **THEN** the client plays the audio; the button shows ■ while playing

#### Scenario: User taps stop during playback

- **WHEN** the user taps ■ while audio is playing
- **THEN** playback stops immediately and the button reverts to ▶

#### Scenario: TTS not configured

- **WHEN** the server has no `[tts]` section in config
- **THEN** `GET /api/messages/{msg_id}/audio` returns HTTP 503
- **THEN** the play button is not shown in the Flutter UI (server advertises capability via `GET /api/capabilities`)

#### Scenario: Message not found

- **WHEN** the client requests audio for a non-existent message ID
- **THEN** the server returns HTTP 404

### Requirement: Server exposes on-demand TTS synthesis endpoint

The server SHALL expose `GET /api/messages/{msg_id}/audio` which reads the message text from the database, synthesizes it via the configured `TtsProvider`, and returns the audio as `audio/mpeg` bytes. The endpoint SHALL NOT cache results (synthesis is stateless and on-demand).

#### Scenario: Valid assistant message

- **WHEN** `GET /api/messages/{msg_id}/audio` is called for an existing assistant message
- **THEN** the server returns HTTP 200 with `Content-Type: audio/mpeg` and synthesized mp3 bytes

#### Scenario: Request for user message

- **WHEN** `GET /api/messages/{msg_id}/audio` is called for a user (non-assistant) message
- **THEN** the server returns HTTP 422 Unprocessable Entity (only assistant messages are voiced)

### Requirement: Server exposes audio store endpoint for tool-synthesized audio

The server SHALL expose `GET /api/audio/{audio_id}` which serves audio bytes previously synthesized and stored by the `voice_response` tool. Entries expire after 1 hour.

#### Scenario: Valid audio ID

- **WHEN** the client fetches `GET /api/audio/{audio_id}` within 1 hour of synthesis
- **THEN** the server returns HTTP 200 with `Content-Type: audio/mpeg` and the audio bytes

#### Scenario: Expired or unknown audio ID

- **WHEN** the client fetches `GET /api/audio/{audio_id}` for an unknown or expired ID
- **THEN** the server returns HTTP 404

### Requirement: Assistant can proactively voice a reply using the `voice_response` tool

The assistant SHALL have access to a `voice_response` tool when TTS is configured. Invoking it synthesizes the provided text, stores the audio in the in-memory `AudioStore`, and causes the client to auto-play the response.

#### Scenario: Assistant invokes voice_response

- **WHEN** the assistant calls `voice_response(text: "Here is your answer")` during a turn
- **THEN** the server synthesizes the text via `TtsProvider`
- **THEN** an `audio_ready` SSE event is emitted with `{"audio_id": "<uuid>", "auto_play": true}`
- **THEN** the Flutter client fetches and auto-plays the audio from `GET /api/audio/{uuid}`

#### Scenario: TTS not configured — tool absent

- **WHEN** the server has no `[tts]` config
- **THEN** `voice_response` is NOT registered in the tool executor
- **THEN** the tool does not appear in the assistant's tool list

#### Scenario: Auto-play when user sent voice

- **WHEN** the user sent the triggering message via the mic button (voice send)
- **THEN** if the assistant uses `voice_response`, the response auto-plays without further user action

### Requirement: Server advertises voice capability to clients

The server SHALL expose `GET /api/capabilities` returning a JSON object indicating which voice features are available, so the Flutter client can conditionally show/hide voice UI elements.

#### Scenario: Both STT and TTS configured

- **WHEN** the client calls `GET /api/capabilities`
- **THEN** the server returns `{"voice_send": true, "voice_receive": true}`

#### Scenario: Only STT configured

- **WHEN** the server has `[transcription]` but no `[tts]`
- **THEN** `GET /api/capabilities` returns `{"voice_send": true, "voice_receive": false}`

#### Scenario: Neither configured

- **WHEN** the server has neither `[transcription]` nor `[tts]`
- **THEN** `GET /api/capabilities` returns `{"voice_send": false, "voice_receive": false}`
- **THEN** the mic button and play buttons are hidden in the Flutter UI
