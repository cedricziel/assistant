## ADDED Requirements

### Requirement: Detect and transcribe inbound Nextcloud Talk audio messages

The Nextcloud adapter SHALL detect audio file shares in inbound chat messages. When a transcription provider is configured, the adapter SHALL download the shared file, check its MIME type with `is_audio_mime()`, transcribe it, and emit a `ChannelMessage` with `ChannelContent::Text` containing `[Voice message]: <transcript>`.

#### Scenario: Voice message with transcription provider configured

- **WHEN** an inbound Nextcloud Talk message is a file share (message type `comment` with `file` parameter) and the shared file has an audio MIME type and a transcription provider is configured
- **THEN** the adapter downloads the file via the Nextcloud file API
- **THEN** the adapter transcribes the audio via the configured `TranscriptionProvider`
- **THEN** a `ChannelMessage` is emitted with content `[Voice message]: <transcript text>`

#### Scenario: Voice message without transcription provider

- **WHEN** an inbound message is an audio file share and no transcription provider is configured
- **THEN** the adapter logs a `warn!` and drops the audio message

#### Scenario: Audio file exceeds 25 MB

- **WHEN** the audio file exceeds 25 MB
- **THEN** the adapter logs a `warn!` and skips transcription

### Requirement: Nextcloud adapter supports with_transcription builder

The `NextcloudAdapter` SHALL provide a `with_transcription(provider, language)` builder method.

#### Scenario: Transcription provider set via builder

- **WHEN** `NextcloudAdapter::new(config)?.with_transcription(provider, Some("en".into()))` is called
- **THEN** the adapter stores the provider and language for use during audio message handling

### Requirement: Send audio files via Nextcloud Talk

The Nextcloud adapter's `send()` method SHALL handle `ChannelContent::FileData` with audio MIME types by uploading the file to the user's Nextcloud files and sharing it into the Talk conversation.

#### Scenario: Outbound audio FileData

- **WHEN** `send()` is called with `ChannelContent::FileData` where the MIME type starts with `audio/`
- **THEN** the adapter uploads the file via WebDAV `PUT` to a path under the bot user's files
- **THEN** the adapter shares the file into the conversation via `POST /ocs/v2.php/apps/files_sharing/api/v1/shares` with `shareType=10` (Talk conversation)

#### Scenario: Outbound text still works

- **WHEN** `send()` is called with `ChannelContent::Text`
- **THEN** existing text-only message behavior is unchanged
