## 1. Dependency and Cargo setup

- [x] 1.1 Add `assistant-transcription` to `[dependencies]` in `crates/interface-matrix/Cargo.toml`
- [x] 1.2 Run `cargo check -p assistant-interface-matrix` to confirm the dependency resolves

## 2. MatrixClient: media download

- [x] 2.1 Write a failing unit test in `client.rs` for `download_media` with a valid `mxc://` URI (wiremock mock returns 200 + bytes + `Content-Type`)
- [x] 2.2 Write a failing unit test for `download_media` with a malformed URI (no `mxc://` prefix) expecting an error
- [x] 2.3 Write a failing unit test for `download_media` where the mock returns non-200, expecting an error
- [x] 2.4 Write a failing unit test for `download_media` where the response exceeds max_bytes, expecting an error
- [x] 2.5 Implement `MatrixClient::download_media(mxc_url: &str, max_bytes: usize) -> Result<(Vec<u8>, String)>` — parse `mxc://`, build `/_matrix/media/v3/download/<server>/<media_id>`, GET with bearer auth, enforce size cap, return bytes + MIME
- [x] 2.6 Confirm all `download_media` unit tests pass

## 3. MatrixAdapter: transcription wiring (voice)

- [x] 3.1 Add `transcription: Option<Arc<dyn TranscriptionProvider>>` and `transcription_language: Option<String>` fields to `MatrixAdapter`
- [x] 3.2 Add `pub fn with_transcription(mut self, provider: Arc<dyn TranscriptionProvider>, language: Option<String>) -> Self` builder to `MatrixAdapter`
- [x] 3.3 Clone transcription fields into the `tokio::spawn` closure in `MatrixAdapter::start`

## 4. MatrixAdapter: voice message handling

- [x] 4.1 Write a failing test: sync response with `m.audio` event + transcription provider configured → `ChannelMessage` emitted with `[Voice message]: ...` content
- [x] 4.2 Write a failing test: `m.audio` event with no transcription provider → no message emitted, warn logged
- [x] 4.3 Write a failing test: `m.audio` event from self → dropped
- [x] 4.4 Write a failing test: `m.audio` event from non-allowed user → dropped
- [x] 4.5 In the sync loop, add an `m.audio` branch: extract `url` from content, apply self/room/user allowlist checks, call `client.download_media` with 25 MB cap, call `provider.transcribe`, emit `ChannelMessage(ChannelContent::Text("[Voice message]: <text>"))` or log warn on failure
- [x] 4.6 Confirm all voice-message adapter tests pass

## 5. MatrixAdapter: image message handling

- [x] 5.1 Write a failing test: sync response with `m.image` event → `ChannelMessage` emitted with `ChannelContent::FileData` containing raw bytes, filename, and MIME type
- [x] 5.2 Write a failing test: `m.image` event exceeding 10 MB → no message emitted, warn logged
- [x] 5.3 Write a failing test: `m.image` event from self → dropped
- [x] 5.4 Write a failing test: `m.image` event from non-allowed user → dropped
- [x] 5.5 In the sync loop, add an `m.image` branch: extract `url` and `body` from content, apply allowlist checks, call `client.download_media` with 10 MB cap, emit `ChannelMessage(ChannelContent::FileData { data, filename, mime_type })` or log warn on failure
- [x] 5.6 Confirm all image-message adapter tests pass

## 6. Runtime: channel runner image dispatch

- [x] 6.1 Write a failing unit test in `channel_runner.rs`: dispatch a `ChannelMessage` with `ChannelContent::FileData { mime_type: "image/jpeg", .. }` → orchestrator called with `"[Image attached]"` and a `ContentBlock::Image` attachment
- [x] 6.2 Write a failing unit test: `FileData` with `mime_type: "application/pdf"` → orchestrator NOT called (return early)
- [x] 6.3 Extend the `dispatch` function in `crates/runtime/src/channel_runner.rs`: match `ChannelContent::FileData { mime_type, data, filename }` where `mime_type.starts_with("image/")` → base64-encode bytes, build `ContentBlock::Image { media_type: mime_type, data: base64 }`, call `run_turn_with_tools("[Image attached]", .., attachments: vec![image_block])`
- [x] 6.4 Confirm channel runner image dispatch tests pass

## 7. MatrixRunner: transcription builder

- [x] 7.1 Add `transcription: Option<Arc<dyn TranscriptionProvider>>` and `transcription_language: Option<String>` fields to `MatrixRunner`
- [x] 7.2 Add `pub fn with_transcription(mut self, provider: Arc<dyn TranscriptionProvider>, language: Option<String>) -> Self` builder to `MatrixRunner`
- [x] 7.3 In `MatrixRunner::run`, call `adapter.with_transcription(...)` when the field is `Some`
- [x] 7.4 Verify the runner compiles and `cargo test -p assistant-interface-matrix` is green

## 8. Lint and format

- [x] 8.1 Run `make lint` and fix any clippy warnings
- [x] 8.2 Run `make format` to ensure `cargo fmt` is satisfied
- [x] 8.3 Run `make test` to confirm the full workspace is green
