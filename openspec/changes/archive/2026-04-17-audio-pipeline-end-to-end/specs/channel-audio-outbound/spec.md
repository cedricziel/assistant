## ADDED Requirements

### Requirement: TTS audio from voice-response tool is delivered as attachment

When the `voice-response` tool synthesizes audio during a turn, the orchestrator SHALL retrieve the audio blob from the `AudioStore` and include it in `TurnResult::attachments` as an `Attachment` with the correct MIME type and filename. The channel runner already sends all turn result attachments through the adapter, so no channel runner changes are needed for delivery.

#### Scenario: voice-response tool produces audio during a messaging turn

- **WHEN** the `voice-response` tool emits an `AudioReady { audio_id }` event during a turn
- **THEN** the orchestrator retrieves the audio bytes and MIME type from the `AudioStore` using the `audio_id`
- **THEN** the audio is appended to `TurnResult::attachments` with filename `voice-response.{ext}` (extension derived from MIME type)
- **THEN** the channel runner sends it via `adapter.send()` as `ChannelContent::FileData`

#### Scenario: AudioStore entry has expired

- **WHEN** the `voice-response` tool emits `AudioReady` but the `AudioStore` entry has already expired
- **THEN** the orchestrator logs a `warn!` and does not append an attachment
- **THEN** the turn completes normally with only the text answer

#### Scenario: No voice-response tool invoked

- **WHEN** a turn completes without any `voice-response` tool calls
- **THEN** `TurnResult::attachments` contains no audio entries (existing behavior unchanged)

### Requirement: Adapters send audio FileData using platform-appropriate method

Each `ChannelAdapter` implementation SHALL handle `ChannelContent::FileData` with audio MIME types by uploading and sending the audio using the platform's native audio/file message format. Adapters that cannot send files SHALL log a warning and drop the content gracefully.

#### Scenario: Matrix adapter receives audio FileData

- **WHEN** the Matrix adapter's `send()` is called with `FileData` where the MIME type starts with `audio/`
- **THEN** it uploads via `upload_media()` and sends via `send_audio()` as an `m.audio` message

#### Scenario: Slack adapter receives audio FileData

- **WHEN** the Slack adapter's `send()` is called with `FileData` where the MIME type starts with `audio/`
- **THEN** it uploads the file using `upload_file()` (existing generic file upload)

#### Scenario: Adapter without file send support

- **WHEN** an adapter that does not support file sending receives audio `FileData`
- **THEN** it logs a `warn!` and returns `Ok(())`
