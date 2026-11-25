# Spec Delta

## Purpose

Lets an org transcribe voice messages through OpenRouter's speech-to-text endpoint, with no separate speech vendor key, at any recording length.

## ADDED Requirements

### Requirement: OpenRouter transcription provider

The system SHALL accept `provider = "openrouter"` in `[transcription]`. Audio SHALL be posted to `{base_url}/audio/transcriptions` as JSON with:

- `model`
- `input_audio: {data: <base64>, format}`
- `language`, when configured
- `prompt`, when `[transcription] prompt` is set
- `response_format: "verbose_json"`

The provider SHALL accept `wav`, `mp3`, `flac`, `m4a`, `ogg`, `webm` and `aac`. Other formats are converted per `transcription-format-conversion`. `base_url` SHALL default to `https://openrouter.ai/api/v1`, and `model` SHALL default to `openai/whisper-1`.

#### Scenario: Voice note

- **WHEN** an `audio/ogg` voice note is transcribed
- **THEN** the request carries `input_audio.format = "ogg"` and the base64 audio, and the response `text` becomes the transcript

#### Scenario: Language reported

- **WHEN** the `verbose_json` response includes `language` and `duration`
- **THEN** the result carries that language, normalized to a BCP-47 code (e.g. `english` becomes `en`), and that duration

### Requirement: Models without verbose output

When OpenRouter rejects `verbose_json` with a 400 error, the provider SHALL retry the same audio once with `response_format: "json"`. It SHALL send `json` for all later requests from that provider instance. The result SHALL then carry the configured language hint, if any, as its language.

#### Scenario: gpt-4o-transcribe

- **WHEN** `model = "openai/gpt-4o-transcribe"` is set and the first request returns 400 for `verbose_json`
- **THEN** the provider retries with `json` and returns the transcript, and the next transcription sends `json` directly

### Requirement: Long recordings are chunked

Recordings longer than `[transcription] max_chunk_secs` (default 50, maximum 55, because upstream providers time out after 60 seconds) SHALL be split with ffmpeg into consecutive chunks no longer than that. The chunks SHALL be transcribed in order, and their texts joined with single spaces. When any chunk fails, the whole transcription SHALL fail. Recordings whose duration cannot be determined SHALL be sent whole.

#### Scenario: Three-minute recording

- **WHEN** a 180-second recording is transcribed with the default settings
- **THEN** four requests are made in order, and the transcript is their texts joined with spaces

### Requirement: Transcription errors

Non-2xx responses SHALL produce an error that carries the status and `error.message`. 408, 429, 502, 503, 504, 524 and 529 SHALL be retried within the bounded retry policy and SHALL honor `Retry-After`. 413 SHALL produce an error saying the audio is too large.

#### Scenario: Payload too large

- **WHEN** OpenRouter answers 413
- **THEN** the error says the audio exceeds the upstream size limit
