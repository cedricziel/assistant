# Spec Delta

## Purpose

Lets each user choose how the assistant speaks to them and whether they see its reasoning. The choices follow the user across devices and interfaces within an org.

## ADDED Requirements

### Requirement: Preference storage and API

`GET /api/users/me/preferences` and `PATCH /api/users/me/preferences` SHALL read and update the caller's preferences in their org. The preferences are:

- `spoken_replies`: `auto` (default; speak replies to voice messages), `always` or `never`
- `tts_voice`
- `tts_speed`
- `show_reasoning` (default `true`)

They SHALL be stored per user and org in org.db, and deleted when the user is removed. The response SHALL also list the voices and speed range of the space's effective TTS model, so clients can offer valid choices. Invalid values SHALL return 422.

#### Scenario: Pick a voice

- **WHEN** a user sets `tts_voice = "nova"` and the space's TTS model offers it
- **THEN** the preference is stored and returned on the next read

#### Scenario: Voice not offered

- **WHEN** a user sets a voice the effective TTS model does not offer
- **THEN** the server returns 422 listing the available voices

### Requirement: Preferences apply per turn

For turns by a user, the following SHALL hold:

- `spoken_replies = never` SHALL suppress native audio replies and TTS fallback clips.
- `always` SHALL produce a spoken reply for text turns too, when TTS or audio output is available.
- `auto` SHALL speak only in voice turns.
- `tts_voice` and `tts_speed` SHALL override the space voice for synthesis and native audio replies, when the active model accepts them. If it does not, the space voice SHALL be used and the response SHALL say so.
- `show_reasoning = false` SHALL stop thinking chunks from being sent to that user's clients.

Reasoning SHALL still be stored either way.

#### Scenario: Never speak

- **WHEN** a user with `spoken_replies = never` sends a voice note
- **THEN** the reply is text only

#### Scenario: Always speak

- **WHEN** a user with `spoken_replies = always` types a message and TTS is configured
- **THEN** the reply carries an audio clip in that user's voice

### Requirement: On-demand synthesis uses preferences

`GET /api/messages/{id}/audio` SHALL synthesize with the caller's `tts_voice` and `tts_speed` when the space's TTS model accepts them.

#### Scenario: Play button

- **WHEN** a user with `tts_voice = "nova"` presses play on an assistant message
- **THEN** the audio is synthesized with `nova`
