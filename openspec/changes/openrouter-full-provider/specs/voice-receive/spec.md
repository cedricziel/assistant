# Spec Delta

## MODIFIED Requirements

### Requirement: Server exposes on-demand TTS synthesis endpoint

The server SHALL expose `GET /api/messages/{msg_id}/audio`. It reads the message text from the database, synthesizes it with the configured `TtsProvider` in the configured TTS output format (`[tts] format`, default `mp3`), and returns the audio with a `Content-Type` that matches that format. The endpoint SHALL NOT cache results, because synthesis is stateless and on demand.

#### Scenario: Valid assistant message

- **WHEN** `GET /api/messages/{msg_id}/audio` is called for an existing assistant message
- **THEN** the server returns HTTP 200 with the synthesized audio and the matching `Content-Type` (`audio/mpeg` by default)

#### Scenario: Ogg output configured

- **WHEN** `[tts] format = "ogg"` is set and the endpoint is called
- **THEN** the server returns Ogg/Opus audio with `Content-Type: audio/ogg`

#### Scenario: Request for user message

- **WHEN** `GET /api/messages/{msg_id}/audio` is called for a user (non-assistant) message
- **THEN** the server returns HTTP 422 Unprocessable Entity (only assistant messages are voiced)
