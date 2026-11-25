# Spec Delta

## Purpose

Lets audio-capable chat models hear a user's voice message directly, including tone, pauses and speakers, instead of only a transcript. Text history and search keep working.

## ADDED Requirements

### Requirement: Voice messages carry the original audio

Every interface that receives a voice message SHALL pass the original recording into the turn as an audio content block (MIME type and bytes), alongside the user message text. The text SHALL be the transcript when transcription is configured, and `[Voice message]` otherwise. A turn that carries an audio block is a voice turn.

#### Scenario: Signal voice note with transcription

- **WHEN** a Signal voice note arrives and `[transcription]` is configured
- **THEN** the turn carries the transcript as text and the Ogg recording as an audio block

#### Scenario: No transcription configured

- **WHEN** a Matrix voice message arrives and `[transcription]` is absent
- **THEN** the turn carries `[Voice message]` as text and the recording as an audio block, and the message is no longer dropped

### Requirement: Audio sent to audio-capable models

`[llm.openrouter.audio_input]` SHALL accept the following:

- `enabled` (default `false`)
- `formats`: the input formats sent unchanged (default `["wav", "mp3"]`)
- `max_secs`: the longest recording sent as audio (default 600)

When audio input is enabled, the catalog lists audio input for the model, and the recording is no longer than `max_secs`, the current turn's user message SHALL be sent with these parts:

- an `input_audio` part `{data: <base64>, format}`, after converting any format not in `formats` to WAV using the audio pipeline;
- a text part that contains the transcript when one exists, prefixed with "Transcript (may contain errors):".

#### Scenario: Audio-capable model

- **WHEN** audio input is enabled for a model with audio input, and a 20-second Ogg voice note arrives
- **THEN** the request's user content holds an `input_audio` part in WAV format and the transcript text part

#### Scenario: Recording too long

- **WHEN** the recording is longer than `max_secs`
- **THEN** only the text is sent, as without audio input

### Requirement: Audio is not replayed or leaked

Audio blocks SHALL be sent only for the current turn's user message. When history is rebuilt for later turns, past voice messages SHALL be represented by their text only. Providers without audio input support, and OpenRouter models without audio input, SHALL never receive audio blocks. The orchestrator SHALL strip the blocks before calling them. The stored recording SHALL remain available as the message's attachment.

#### Scenario: Next turn

- **WHEN** the user follows a voice message with a typed message
- **THEN** the second request contains the first message's text but no `input_audio` part

#### Scenario: Anthropic provider

- **WHEN** `[llm] provider = "anthropic"` is set and a voice message arrives
- **THEN** the Anthropic request contains only text

### Requirement: Fallback when audio is rejected

When the upstream rejects a request because of the audio part, the provider SHALL repeat the call once with the audio part removed. It SHALL stop sending audio for the rest of that provider instance's life, and SHALL log the reason once. If no transcript exists in that case, the text part SHALL still say a voice message was received, so the model can ask the user to type instead.

#### Scenario: Endpoint without audio support

- **WHEN** the routed endpoint answers 400 about `input_audio`
- **THEN** the call is repeated with text only and succeeds, and later voice turns send text only
