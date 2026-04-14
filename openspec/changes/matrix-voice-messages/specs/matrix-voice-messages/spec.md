## ADDED Requirements

### Requirement: Detect and transcribe m.audio voice messages

The Matrix adapter SHALL detect `m.room.message` events with `msgtype: m.audio` in the sync loop and, when a transcription provider is configured, download the audio and transcribe it. The resulting transcript SHALL be injected into the runtime as a `ChannelMessage` with `ChannelContent::Text` containing `[Voice message]: <transcript>`. All existing allowlist checks (room, user) SHALL apply to voice messages identically to text messages.

#### Scenario: Voice message with transcription provider configured

- **WHEN** a sync response contains an `m.room.message` event with `msgtype: m.audio` from an allowed user in an allowed room
- **THEN** the adapter downloads the audio bytes, transcribes them, and emits a `ChannelMessage` whose content is `[Voice message]: <transcript text>`

#### Scenario: Voice message without transcription provider configured

- **WHEN** a sync response contains an `m.room.message` event with `msgtype: m.audio` and no transcription provider is set on the adapter
- **THEN** the adapter logs a `warn!` and drops the event without emitting a `ChannelMessage`

#### Scenario: Voice message from self is ignored

- **WHEN** a sync response contains an `m.audio` event whose sender matches the bot's own user ID
- **THEN** the adapter drops the event and does not transcribe or emit it

#### Scenario: Voice message from non-allowed user is dropped

- **WHEN** a non-empty `allowed_users` list is configured and the sender of an `m.audio` event is not in the list
- **THEN** the adapter drops the event without transcribing

### Requirement: Detect and forward m.image image messages

The Matrix adapter SHALL detect `m.room.message` events with `msgtype: m.image` in the sync loop, download the image bytes (up to 10 MB) via the Matrix Content Repository, and emit a `ChannelMessage` with `ChannelContent::FileData { data, filename, mime_type }`. All existing allowlist checks SHALL apply identically.

#### Scenario: Image message received and forwarded

- **WHEN** a sync response contains an `m.room.message` event with `msgtype: m.image` from an allowed user in an allowed room
- **THEN** the adapter downloads the image bytes and emits a `ChannelMessage` with `ChannelContent::FileData` containing the raw bytes, filename (from `body` field or `"image"`), and MIME type from the `Content-Type` response header

#### Scenario: Image exceeds size limit

- **WHEN** the image download would exceed 10 MB
- **THEN** the adapter logs a `warn!` and drops the event without emitting a `ChannelMessage`

#### Scenario: Image message from self is ignored

- **WHEN** a sync response contains an `m.image` event whose sender matches the bot's own user ID
- **THEN** the adapter drops the event

#### Scenario: Image message from non-allowed user is dropped

- **WHEN** a non-empty `allowed_users` list is configured and the sender of an `m.image` event is not in the list
- **THEN** the adapter drops the event

### Requirement: MatrixClient provides media download

The `MatrixClient` SHALL expose a `download_media` method that accepts an `mxc://` URI and a maximum byte limit, resolves the URI to a `/_matrix/media/v3/download/<server>/<media_id>` URL, downloads the bytes with the bearer token, enforces the size limit, and returns the raw bytes and the `Content-Type` header value.

#### Scenario: Successful media download

- **WHEN** `download_media` is called with a valid `mxc://<server>/<media_id>` URI
- **THEN** it returns the audio/image bytes and MIME type from the homeserver's Content-Repository response

#### Scenario: Media exceeds size limit

- **WHEN** the response body would exceed the configured max byte limit
- **THEN** `download_media` returns an error and no bytes are stored

#### Scenario: Malformed MXC URI

- **WHEN** `download_media` is called with a URI that does not start with `mxc://` or cannot be split into server and media ID
- **THEN** it returns an error immediately without making a network request

#### Scenario: Homeserver returns non-200

- **WHEN** the homeserver returns a non-success HTTP status for the media download
- **THEN** `download_media` returns an error containing the status code

### Requirement: Transcription provider wired through runner and adapter

The `MatrixAdapter` and `MatrixRunner` SHALL each expose a `with_transcription(provider: Arc<dyn TranscriptionProvider>, language: Option<String>)` builder method. The runner SHALL pass the configured provider to the adapter during startup.

#### Scenario: Runner wires provider to adapter

- **WHEN** `MatrixRunner::with_transcription` is called before `run()`
- **THEN** the adapter created during `run()` has the same provider attached and transcribes incoming voice messages

#### Scenario: No provider set, runner starts normally

- **WHEN** `MatrixRunner` is started without calling `with_transcription`
- **THEN** the runner starts successfully and voice messages are warned and dropped

### Requirement: Channel runner dispatches image FileData as multimodal attachments

The `channel_runner.rs` dispatch function SHALL recognise `ChannelContent::FileData` whose `mime_type` starts with `image/` and convert it to a `ContentBlock::Image { media_type, data }` (base64-encoded) attachment passed to `Orchestrator::run_turn_with_tools` with `user_message = "[Image attached]"`.

#### Scenario: Image FileData dispatched as multimodal turn

- **WHEN** a `ChannelMessage` with `ChannelContent::FileData { mime_type: "image/jpeg", .. }` is dispatched
- **THEN** the channel runner calls `run_turn_with_tools` with the text `"[Image attached]"` and one `ContentBlock::Image` attachment containing the base64-encoded bytes

#### Scenario: Non-image FileData is dropped

- **WHEN** a `ChannelMessage` with `ChannelContent::FileData { mime_type: "application/pdf", .. }` is dispatched
- **THEN** the channel runner drops it (returns early without calling the orchestrator)
