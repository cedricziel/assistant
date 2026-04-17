## ADDED Requirements

### Requirement: Detect and transcribe inbound Mattermost audio file attachments

The Mattermost adapter SHALL detect audio file attachments on inbound posts by fetching file metadata and checking the MIME type with `is_audio_mime()`. When a transcription provider is configured, the adapter SHALL download the file content, transcribe it, and emit a `ChannelMessage` with `ChannelContent::Text` containing `[Voice message]: <transcript>`.

#### Scenario: Post with audio file attachment and transcription configured

- **WHEN** an inbound post contains a `file_ids` array with a file whose MIME type is an audio type and a transcription provider is configured
- **THEN** the adapter fetches `GET /api/v4/files/{file_id}/info` to check MIME type
- **THEN** the adapter downloads the file via `GET /api/v4/files/{file_id}`
- **THEN** the adapter transcribes the audio via the configured `TranscriptionProvider`
- **THEN** a `ChannelMessage` is emitted with content `[Voice message]: <transcript text>` (the original post text, if any, is prepended)

#### Scenario: Post with audio file but no transcription provider

- **WHEN** an inbound post contains an audio file attachment and no transcription provider is configured
- **THEN** the adapter logs a `warn!` and drops the audio portion (text content, if present, is still dispatched)

#### Scenario: Audio file exceeds 25 MB

- **WHEN** the file metadata indicates a size exceeding 25 MB
- **THEN** the adapter logs a `warn!` and skips transcription for that file

### Requirement: Mattermost adapter supports with_transcription builder

The `MattermostAdapter` SHALL provide a `with_transcription(provider, language)` builder method.

#### Scenario: Transcription provider set via builder

- **WHEN** `MattermostAdapter::new(config)?.with_transcription(provider, Some("en".into()))` is called
- **THEN** the adapter stores the provider and language for use during audio message handling

### Requirement: Send audio files via Mattermost

The Mattermost adapter's `send()` method SHALL handle `ChannelContent::FileData` with audio MIME types by uploading the file via `POST /api/v4/files` and then creating a post with the file ID attached.

#### Scenario: Outbound audio FileData

- **WHEN** `send()` is called with `ChannelContent::FileData` where the MIME type starts with `audio/`
- **THEN** the adapter uploads the file via `POST /api/v4/files?channel_id={channel_id}` with multipart form data
- **THEN** the adapter creates a post via `POST /api/v4/posts` with the returned `file_id` in the `file_ids` array

#### Scenario: Outbound text still works

- **WHEN** `send()` is called with `ChannelContent::Text`
- **THEN** existing text-only post behavior is unchanged
