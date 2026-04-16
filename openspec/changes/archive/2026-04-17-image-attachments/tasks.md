## 1. Core Types (`assistant-core`)

- [x] 1.1 Create `crates/core/src/attachment.rs` with `AttachmentMeta` struct (id, message_id, conversation_id, agent_id, filename, mime_type, size_bytes, created_at)
- [x] 1.2 Add `SUPPORTED_MIME_TYPES`, `RESIZABLE_MIME_TYPES`, `MAX_ATTACHMENT_SIZE` constants
- [x] 1.3 Add `is_supported_mime_type()` and `AttachmentMeta::extension()` helpers
- [x] 1.4 Export `attachment` module from `crates/core/src/lib.rs`
- [x] 1.5 Add `attachment_ids: Vec<Uuid>` field to `TurnRequest` in `crates/core/src/bus_messages.rs` (default empty vec)
- [x] 1.6 Change `TurnResult.attachments` from `Vec<Attachment>` to `attachment_ids: Vec<Uuid>` in `crates/core/src/bus_messages.rs`
- [x] 1.7 Fix all compile errors from `TurnResult` change across the workspace (runtime, web-ui, channel_runner, interfaces)
- [x] 1.8 Write unit tests for `AttachmentMeta::extension()`, `is_supported_mime_type()`, and MIME type constants

## 2. Storage — AttachmentStore (`assistant-storage`)

- [x] 2.1 Add SQLite migration for `message_attachments` table (id, message_id nullable, conversation_id, agent_id, filename, mime_type, size_bytes, created_at) with indexes on message_id and conversation_id
- [x] 2.2 Create `crates/storage/src/attachments.rs` with `AttachmentStore` struct holding `SqlitePool`
- [x] 2.3 Implement `AttachmentStore::store(meta, data)` — write bytes to `~/.assistant/agents/{agent_id}/attachments/{conv_id}/{id}.{ext}` via `tokio::fs`, insert metadata row
- [x] 2.4 Implement `AttachmentStore::load_bytes(id)` — look up metadata, read file from disk
- [x] 2.5 Implement `AttachmentStore::get_meta(id)` — SELECT from `message_attachments`
- [x] 2.6 Implement `AttachmentStore::list_for_message(message_id)` and `list_for_conversation(conversation_id)`
- [x] 2.7 Implement `AttachmentStore::link_to_message(attachment_id, message_id)` — UPDATE message_id
- [x] 2.8 Export `AttachmentStore` from `crates/storage/src/lib.rs`
- [x] 2.9 Write unit tests using `StorageLayer::new_in_memory()` — store, load_bytes, get_meta, list, link_to_message

## 3. Runtime — Orchestrator Integration

- [x] 3.1 Add `AttachmentStore` (or `Arc<AttachmentStore>`) to orchestrator state
- [x] 3.2 Inbound: in `prepare_history`, when `attachment_ids` is non-empty, load bytes from `AttachmentStore` and build `ChatHistoryMessage::MultimodalUser` with `ContentBlock::Text` + `ContentBlock::Image` blocks
- [x] 3.3 Inbound: resize images per provider limits before base64 encoding (Anthropic ~5 MB, OpenAI ~20 MB) using the `image` crate — transient, in-memory only
- [x] 3.4 Inbound: for Ollama provider, emit `warn!("Ollama model may not support vision. Image attachments included but may be ignored by the model.")`
- [x] 3.5 Outbound: in `finalize_tool_result`, when `ToolOutput.attachments` is non-empty, persist each via `AttachmentStore::store()`, link to assistant message, collect IDs
- [x] 3.6 Outbound: populate `TurnResult.attachment_ids` with persisted attachment IDs
- [x] 3.7 History replay: when loading conversation history, include `ContentBlock::Image` for messages with linked attachments
- [x] 3.8 Write tests for inbound multimodal history building with mock `AttachmentStore`
- [x] 3.9 Write tests for outbound tool attachment persistence flow

## 4. Web UI — API Endpoints

- [x] 4.1 Add `AttachmentStore` to `ApiState` in `crates/web-ui/src/api/mod.rs`
- [x] 4.2 Wire `AttachmentStore` creation in web-ui `main.rs` startup (pass existing `SqlitePool`)
- [x] 4.3 Implement `POST /api/conversations/{id}/attachments` — multipart upload handler: validate MIME type, enforce 10 MB limit, persist via `AttachmentStore`, return `201` with `AttachmentMeta` JSON
- [x] 4.4 Implement `GET /api/attachments/{id}` — auth check, load metadata, resolve resize params (`w`, `h` query params, default 1920px), serve resized image with `Cache-Control: private, immutable, max-age=31536000` and `ETag`
- [x] 4.5 Implement on-demand resize using `image` crate — load original, resize to bounding box preserving aspect ratio, write cached variant to `cache/{id}_{w}x{h}.{ext}` alongside originals
- [x] 4.6 Implement conditional request support — check `If-None-Match` against `ETag`, return `304` when matched
- [x] 4.7 Add optional `attachment_ids` field to `SendMessageRequest` — link attachments to message, include IDs in `TurnRequest`
- [x] 4.8 Include attachment metadata in SSE message events (id, filename, mime_type, url)
- [x] 4.9 Include attachment metadata in message list/detail API responses
- [x] 4.10 Add utoipa path annotations to new handlers
- [x] 4.11 Register new routes in `api_router()` behind existing auth middleware
- [x] 4.12 Write tests for upload validation (MIME type, size limit, auth)
- [x] 4.13 Write tests for serving with resize params and cache headers

## 5. Interface Adapters — Inbound

- [x] 5.1 Add `AttachmentStore` to `ChannelRunner` (or pass through adapter context)
- [x] 5.2 In `ChannelRunner::content_to_dispatch`, when `ChannelContent::FileData` or `ChannelContent::Image` is received, persist via `AttachmentStore::store()` and return attachment IDs alongside text
- [x] 5.3 Pass attachment IDs through to `orchestrator.run_turn_with_tools()` via the `attachments` parameter
- [x] 5.4 Slack adapter: ensure `file_share` events with image MIME types are downloaded and forwarded as `ChannelContent::FileData`
- [x] 5.5 Matrix adapter: ensure `m.image` events are downloaded and forwarded as `ChannelContent::FileData`
- [x] 5.6 Write tests for `ChannelRunner` attachment persistence path

## 6. Interface Adapters — Outbound

- [x] 6.1 In `ChannelRunner`, after sending text reply, iterate `TurnResult.attachment_ids` and deliver per-adapter
- [x] 6.2 Slack adapter: implement image upload — load bytes from `AttachmentStore`, upload via `files.upload` API to the conversation thread
- [x] 6.3 Matrix adapter: implement image upload — load bytes, upload via `PUT /_matrix/media/.../upload`, send `m.image` event
- [x] 6.4 Web UI: no additional work needed (SSE events + serving endpoint handle it)
- [x] 6.5 Write tests for outbound attachment delivery (mock HTTP for Slack/Matrix)

## 7. OpenAPI and Flutter Client Generation

- [x] 7.1 Update `openapi.json` with new endpoints (`POST /api/conversations/{id}/attachments`, `GET /api/attachments/{id}`), `AttachmentMeta` schema, and updated `SendMessageRequest` with optional `attachment_ids`
- [x] 7.2 Run `make dump-openapi` to regenerate from running server
- [x] 7.3 Run `make generate-flutter-client` to regenerate Dart API client

## 8. Flutter App — Upload and Input

- [x] 8.1 Add `image_picker` package to `app/pubspec.yaml` and run `flutter pub get`
- [x] 8.2 Add camera/photo library usage descriptions to `app/macos/Runner/Info.plist` and `app/ios/Runner/Info.plist` (if applicable)
- [x] 8.3 Create `AttachmentService` in `app/lib/api/` — wraps multipart upload to `POST /api/conversations/{id}/attachments`, returns `AttachmentMeta`
- [x] 8.4 Create `attachmentProvider` Riverpod provider for managing pending attachments (selected but not yet sent)
- [x] 8.5 Add image picker button to chat input row in `chat_screen.dart`
- [x] 8.6 Implement drag-and-drop zone on web/desktop — show visual drop indicator, add dropped images to pending attachments
- [x] 8.7 Implement paste-from-clipboard detection on web/desktop — detect image paste, add to pending attachments
- [x] 8.8 Show thumbnail previews of pending attachments in the input area (with remove button)
- [x] 8.9 Wire send button to upload pending attachments, then send message with `attachment_ids`

## 9. Flutter App — Rendering

- [x] 9.1 Update chat message bubble widget to detect attachments in message metadata
- [x] 9.2 Render inline thumbnails via `GET /api/attachments/{id}?w=300` with auth headers
- [x] 9.3 Implement tap-to-expand full-size image viewer (e.g. dialog or hero animation) at `GET /api/attachments/{id}?w=1920`
- [x] 9.4 Handle assistant messages with attachments (same rendering as user messages)
- [x] 9.5 Handle chat history replay — render images from attachment metadata in message list response
- [x] 9.6 Write widget tests for image attachment rendering in chat bubbles

## 10. Workspace Dependencies

- [x] 10.1 Add `image` crate to `[workspace.dependencies]` in root `Cargo.toml` (for server-side resizing)
- [x] 10.2 Verify `image` crate features are minimal (only decode/encode for PNG, JPEG, GIF, WebP)
- [x] 10.3 Run `make lint && make format` — ensure clean
- [x] 10.4 Run `make test` — ensure all existing tests pass with `TurnResult` change
