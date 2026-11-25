# Spec Delta

## MODIFIED Requirements

### Requirement: Server advertises voice capability to clients

The server SHALL expose `GET /api/capabilities`, returning a JSON object with these fields, resolved from the selected space's effective configuration and the caller's preferences:

- `voice_send`: whether transcription or audio input is available
- `voice_receive`: whether TTS or native audio output is available
- `image_generation`: whether `[images]` is configured and enabled
- `audio_input`: whether native audio input is enabled for an audio-capable model
- `spoken_replies`: the caller's preference

The Flutter client uses these fields to show or hide voice and image UI elements.

#### Scenario: Both STT and TTS configured

- **WHEN** the client calls `GET /api/capabilities` for a space whose effective configuration has `[transcription]` and `[tts]`
- **THEN** the response includes `"voice_send": true, "voice_receive": true`

#### Scenario: Only STT configured

- **WHEN** the space's effective configuration has `[transcription]` but no `[tts]`, and no native audio output
- **THEN** the response includes `"voice_send": true, "voice_receive": false`

#### Scenario: Neither configured

- **WHEN** the space's effective configuration has neither `[transcription]` nor `[tts]`, and no native audio
- **THEN** the response includes `"voice_send": false, "voice_receive": false`
- **THEN** the mic button and play buttons are hidden in the Flutter UI

#### Scenario: Different spaces

- **WHEN** space A has TTS and space B in another org does not
- **THEN** `GET /api/capabilities` returns `voice_receive: true` for A and `false` for B
