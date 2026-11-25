# Spec Delta

## Purpose

Lets voice messages in any common audio or video container be transcribed by any configured provider. Audio is converted to a format the provider accepts when needed.

## ADDED Requirements

### Requirement: Recognized audio inputs

The system SHALL treat attachments with any of these MIME types as audio to transcribe:

- `audio/ogg`, `audio/opus`
- `audio/mpeg`, `audio/mp3`
- `audio/mp4`, `audio/m4a`, `audio/x-m4a`
- `audio/wav`, `audio/x-wav`, `audio/wave`
- `audio/flac`, `audio/x-flac`
- `audio/webm`, `audio/aac`
- `audio/amr`, `audio/amr-wb`
- `audio/3gpp`, `audio/3gpp2`
- `audio/x-caf`
- `audio/aiff`, `audio/x-aiff`
- `video/mp4`, `video/webm`, `video/quicktime`

Parameters after `;` (e.g. `audio/ogg; codecs=opus`) SHALL be ignored when matching.

#### Scenario: AMR voice note

- **WHEN** a channel delivers an `audio/amr` attachment
- **THEN** it is transcribed and not ignored

#### Scenario: MIME parameters

- **WHEN** the MIME type is `audio/ogg; codecs=opus`
- **THEN** it is recognized as Ogg audio

### Requirement: Conversion to an accepted format

Each transcription provider SHALL declare the input formats it accepts. Audio in any other recognized format SHALL be converted with ffmpeg to 16 kHz mono WAV before it is sent. For video containers, only the audio stream SHALL be extracted. When ffmpeg is required but not installed, transcription SHALL fail with an error saying ffmpeg is needed for that format. Audio already in an accepted format SHALL be sent unchanged.

#### Scenario: CAF to OpenRouter

- **WHEN** an `audio/x-caf` attachment is transcribed by a provider that does not accept CAF
- **THEN** it is converted to WAV first and transcribed

#### Scenario: Accepted format passes through

- **WHEN** an `audio/ogg` attachment goes to a provider that accepts Ogg
- **THEN** ffmpeg is not invoked

#### Scenario: ffmpeg missing

- **WHEN** conversion is needed and ffmpeg is not on the PATH
- **THEN** the error names ffmpeg and the source format
