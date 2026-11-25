# Spec Delta

## Purpose

Lets an org speak replies through any OpenRouter text-to-speech model, in the audio format each consumer needs, whatever formats the model itself produces.

## ADDED Requirements

### Requirement: OpenRouter TTS provider

The system SHALL accept `provider = "openrouter"` in `[tts]`. Synthesis SHALL post `{model, input, voice, response_format}` to `{base_url}/audio/speech`. It SHALL add `speed` when the request or config sets it, and `instructions` when `[tts] instructions` is set. `base_url` SHALL default to `https://openrouter.ai/api/v1`, `model` to `openai/gpt-4o-mini-tts` and `voice` to `alloy`. A voice in the request SHALL override the configured voice.

#### Scenario: Synthesis

- **WHEN** a reply is synthesized with `voice = "nova"` for a model that outputs mp3
- **THEN** the request carries `"voice": "nova", "response_format": "mp3"`, and the result is the response bytes as `audio/mpeg`

### Requirement: Output formats

The TTS output format SHALL be one of `mp3` (`audio/mpeg`), `wav` (`audio/wav`), `ogg` (Opus in Ogg, `audio/ogg`) or `pcm` (16-bit little-endian mono, `audio/pcm`). It is taken from the request's format, then from `[tts] format`, and defaults to `mp3`. Other values SHALL be rejected. The result's MIME type SHALL always match the delivered format.

#### Scenario: Ogg for a voice note

- **WHEN** `[tts] format = "ogg"` is set
- **THEN** the result is Ogg/Opus audio with MIME type `audio/ogg`

#### Scenario: Unsupported format

- **WHEN** `[tts] format = "aac"` is set
- **THEN** config loading fails and lists the supported formats

### Requirement: Native format selection and transcoding

The provider SHALL read the configured model's supported `response_format` values from `{base_url}/audio/speech/models/{model}`. When the model supports the requested format, the provider SHALL request it directly. Otherwise it SHALL request the first supported format among `wav`, `mp3` and `pcm`, and transcode the result to the requested format with ffmpeg. Raw PCM SHALL be read at the model's sample rate: `[tts] pcm_sample_rate` when set, otherwise the documented rate for known model families (44100 Hz for `heygen/*`, 24000 Hz otherwise). When transcoding is required and ffmpeg is not installed, the provider SHALL fail to build and name ffmpeg.

#### Scenario: PCM-only model

- **WHEN** `model = "heygen/heygen-voice-1"` is set (the model supports only `pcm`) and `mp3` is requested
- **THEN** the provider requests `pcm`, reads it at 44100 Hz, and returns `audio/mpeg`

#### Scenario: Gemini to Ogg

- **WHEN** a Gemini TTS model (`pcm` or `wav`) is configured and `ogg` is requested
- **THEN** the provider requests `wav` and returns `audio/ogg`

### Requirement: Validation against the catalog

At build time, the provider SHALL validate the config against the model's catalog entry:

- When `voice` is listed as an enum, the configured voice SHALL be one of its values.
- `speed` SHALL be within the model's range.
- `instructions` SHALL be set only when the model lists `instructions`.

Violations SHALL fail the build with an error naming the allowed values. A request-level voice outside the enum SHALL fail that request without calling OpenRouter. When the catalog cannot be reached, the provider SHALL build without validation, request the requested format directly, and log a warning.

#### Scenario: Unknown voice

- **WHEN** `voice = "nova"` is configured for `mistralai/voxtral-mini-tts-2603`, whose voices do not include `nova`
- **THEN** startup fails and lists the model's voices

#### Scenario: Speed out of range

- **WHEN** `speed = 1.5` is configured for a model with a speed range of 0.7 to 1.2
- **THEN** startup fails and states the range

### Requirement: Consumers choose their format

The `voice_response` tool SHALL use the configured TTS format instead of always asking for mp3. The on-demand synthesis endpoint SHALL return the provider's MIME type as its `Content-Type`.

#### Scenario: Tool-synthesized Ogg

- **WHEN** `[tts] format = "ogg"` is set and the model calls `voice_response`
- **THEN** the stored audio has MIME type `audio/ogg`
