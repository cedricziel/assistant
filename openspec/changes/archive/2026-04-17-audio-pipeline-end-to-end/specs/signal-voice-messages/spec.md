## ADDED Requirements

### Requirement: Detect and transcribe inbound Signal audio attachments

The Signal adapter SHALL detect audio attachments in inbound WebSocket messages by checking the MIME type with `is_audio_mime()`. When a transcription provider is configured, the adapter SHALL decode the base64 attachment data, transcribe it, and emit a `ChannelMessage` with `ChannelContent::Text` containing `[Voice message]: <transcript>`.

#### Scenario: Audio attachment with transcription provider configured

- **WHEN** a WebSocket message contains an attachment with an audio MIME type (e.g. `audio/ogg`, `audio/mpeg`) and a transcription provider is configured
- **THEN** the adapter decodes the base64 attachment data
- **THEN** the adapter transcribes the audio via the configured `TranscriptionProvider`
- **THEN** a `ChannelMessage` is emitted with content `[Voice message]: <transcript text>`

#### Scenario: Audio attachment without transcription provider

- **WHEN** a WebSocket message contains an audio attachment and no transcription provider is configured
- **THEN** the adapter logs a `warn!("Signal audio received but no transcription provider configured")` and drops the message

#### Scenario: Audio attachment exceeds size limit

- **WHEN** the decoded audio data exceeds 25 MB
- **THEN** the adapter logs a `warn!` and drops the message without transcribing

#### Scenario: Non-audio attachment

- **WHEN** a WebSocket message contains an attachment with a non-audio MIME type (e.g. `image/png`)
- **THEN** the adapter handles it as before (existing behavior unchanged)

### Requirement: Signal adapter supports with_transcription builder

The `SignalAdapter` SHALL provide a `with_transcription(provider, language)` builder method matching the pattern used by `MatrixAdapter` and `SlackAdapter`.

#### Scenario: Transcription provider set via builder

- **WHEN** `SignalAdapter::new(config)?.with_transcription(provider, Some("en".into()))` is called
- **THEN** the adapter stores the provider and language for use during audio message handling

### Requirement: Send audio files via Signal

The Signal adapter's `send()` method SHALL handle `ChannelContent::FileData` with audio MIME types by encoding the data as base64 and including it as an attachment in the `POST /v1/send` request body.

#### Scenario: Outbound audio FileData

- **WHEN** `send()` is called with `ChannelContent::FileData` where the MIME type starts with `audio/`
- **THEN** the adapter POSTs to `/v1/send` with the audio data as a base64-encoded attachment
- **THEN** the attachment includes the filename and MIME type (content type)

#### Scenario: Outbound non-audio FileData

- **WHEN** `send()` is called with `ChannelContent::FileData` where the MIME type does not start with `audio/`
- **THEN** the adapter handles it identically (generic file attachment)
