## 1. Outbound Audio Pipeline (Orchestrator → Adapter)

- [x] 1.1 Add `audio_attachments` collection to `TurnResult` in `crates/runtime/src/orchestrator/mod.rs` — when `AudioReady { audio_id }` is emitted during dispatch, retrieve the blob from `AudioStore` and append as an `Attachment` with correct MIME type and filename `voice-response.{ext}`
- [x] 1.2 Add `Arc<AudioStore>` as an optional dependency to the orchestrator (passed via constructor/builder) so it can retrieve audio blobs during dispatch
- [x] 1.3 Write unit tests: turn with `voice-response` tool produces audio attachment in `TurnResult`; turn without voice-response has no audio attachments; expired `AudioStore` entry logs warning and produces no attachment
- [x] 1.4 Wire `AudioStore` into the orchestrator in `crates/interface-cli/src/main.rs` and `crates/web-ui/src/main.rs` where the orchestrator is constructed

## 2. Signal Voice Messages

- [ ] 2.1 Add `assistant-transcription` dependency to `crates/interface-signal/Cargo.toml`
- [ ] 2.2 Add `transcription: Option<Arc<dyn TranscriptionProvider>>` and `transcription_language: Option<String>` fields to `SignalAdapter`, with `with_transcription(provider, language)` builder method
- [ ] 2.3 Implement inbound audio detection in the WebSocket message parser — check attachment MIME types with `is_audio_mime()`, decode base64 data, transcribe, emit `[Voice message]: <transcript>` as `ChannelContent::Text`
- [ ] 2.4 Implement outbound `FileData` handling in `send()` — encode audio data as base64, include as attachment in `POST /v1/send` request body with filename and content type
- [ ] 2.5 Wire `with_transcription()` in `SignalRunner` (or the CLI entrypoint) when a transcription provider is configured
- [ ] 2.6 Write unit tests: inbound audio with provider → transcribed text; inbound audio without provider → dropped with warn; outbound audio FileData → correct POST body; audio > 25 MB → dropped

## 3. Mattermost Voice Messages

- [ ] 3.1 Add `assistant-transcription` dependency to `crates/interface-mattermost/Cargo.toml`
- [ ] 3.2 Add `transcription: Option<Arc<dyn TranscriptionProvider>>` and `transcription_language: Option<String>` fields to `MattermostAdapter`, with `with_transcription(provider, language)` builder method
- [ ] 3.3 Implement inbound audio detection — when a post has `file_ids`, fetch `GET /api/v4/files/{id}/info` to check MIME type, download via `GET /api/v4/files/{id}`, transcribe, emit `[Voice message]: <transcript>`
- [ ] 3.4 Implement outbound `FileData` handling in `send()` — upload via `POST /api/v4/files?channel_id={id}` (multipart), then create post with `file_ids` array
- [ ] 3.5 Wire `with_transcription()` in `MattermostRunner` when a transcription provider is configured
- [ ] 3.6 Write unit tests: inbound audio file → transcribed; no provider → dropped; outbound audio → uploaded and posted; file > 25 MB → skipped

## 4. Nextcloud Voice Messages

- [ ] 4.1 Add `assistant-transcription` dependency to `crates/interface-nextcloud/Cargo.toml`
- [ ] 4.2 Add `transcription: Option<Arc<dyn TranscriptionProvider>>` and `transcription_language: Option<String>` fields to `NextcloudAdapter`, with `with_transcription(provider, language)` builder method
- [ ] 4.3 Implement inbound audio detection — detect file share messages, check MIME type, download via Nextcloud file API, transcribe, emit `[Voice message]: <transcript>`
- [ ] 4.4 Implement outbound `FileData` handling in `send()` — upload via WebDAV `PUT`, share into conversation via `POST /ocs/v2.php/apps/files_sharing/api/v1/shares` with `shareType=10`
- [ ] 4.5 Wire `with_transcription()` in the Nextcloud runner/entrypoint when a transcription provider is configured
- [ ] 4.6 Write unit tests: inbound audio share → transcribed; no provider → dropped; outbound audio → uploaded and shared; file > 25 MB → skipped

## 5. Integration and Verification

- [ ] 5.1 Run `make lint` and `make format` — ensure all new code passes clippy and fmt
- [ ] 5.2 Run `make test` — ensure all existing and new tests pass
- [ ] 5.3 Verify `make build` succeeds with the new `assistant-transcription` dependencies in adapter crates
