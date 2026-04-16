# Image Attachments — Design

## Architecture Overview

```
  INGEST                          STORAGE                         LLM
  ══════                          ═══════                         ═══

  Slack ──┐                    ┌──────────────────┐
  Matrix ─┤                    │ AttachmentStore   │
  Web UI ─┼──▶ validate ──▶   │                   │
  Flutter ┘    & normalize     │  ┌─ filesystem ─┐ │        ┌─────────────┐
                               │  │ ~/.assistant/ │ │        │  Anthropic  │
               mime check      │  │  agents/      │ │  load  │  (base64)   │
               size check      │  │   {agent}/    │◀├────────│             │
                               │  │    attach/    │ │ bytes  ├─────────────┤
                               │  │     {conv}/   │ │        │   OpenAI    │
                               │  │      {id}.ext │ │        │  (data-URI) │
                               │  └───────────────┘ │        ├─────────────┤
                               │                    │        │   Ollama    │
                               │  ┌─ SQLite ──────┐ │        │  (if model  │
                               │  │ message_      │ │        │   supports) │
                               │  │  attachments  │ │        └─────────────┘
                               │  │ (metadata)    │ │
                               │  └───────────────┘ │
                               └──────────────────┘

  SERVE
  ═════

  GET /api/attachments/{id}  ──▶  auth check ──▶  load meta ──▶  stream file
                                                                  Content-Type
```

## Bidirectional Flow

Attachments flow in both directions through the same `AttachmentStore`:

```
  INBOUND (user → LLM)                        OUTBOUND (LLM → user)
  ═════════════════════                        ══════════════════════

  User uploads image                           Tool produces image
  (Web UI, Slack, Matrix)                      (bash, DALL-E, skill, MCP)
       │                                            │
       ▼                                            ▼
  Interface / API handler                      Orchestrator
       │                                            │
       │  AttachmentStore::store(meta, bytes)        │  AttachmentStore::store(meta, bytes)
       │──────────────────────────▶│◀────────────────│
       │                           │                 │
       │  returns AttachmentMeta   │                 │  returns AttachmentMeta
       │◀──────────────────────────│─────────────────│▶
       │                           │                 │
       ▼                           │                 ▼
  TurnRequest {                    │            TurnResult {
    attachment_ids: [uuid]         │              answer: "...",
  }                                │              attachment_ids: [uuid]
                                   │            }
  Runtime loads bytes              │                 │
  → ContentBlock::Image            │                 ▼
  → LLM provider                   │            Interface delivers:
                                   │              Web UI: SSE event + GET /api/attachments/{id}
                                   │              Slack: files.upload (loads bytes from store)
                                   │              Matrix: PUT media upload + m.image event
```

### Type Lifecycle

```
  Attachment (transient bytes DTO)           AttachmentMeta (persisted pointer)
  ════════════════════════════════           ══════════════════════════════════

  Created by: tool handlers                  Created by: AttachmentStore::store()
  Contains:   filename, mime_type, Vec<u8>   Contains:   id, filename, mime_type, size, etc.
  Lifespan:   tool execution → store()       Lifespan:   persisted to SQLite + disk

                    store()
  Attachment ────────────────▶ AttachmentMeta
  (bytes written to disk)      (metadata written to SQLite)
```

`Attachment` remains the in-memory DTO (used inside tool execution). `AttachmentMeta` is what crosses system boundaries — bus messages, API responses, SSE events. No raw bytes on the bus.

## Types — `assistant-core`

```rust
// crates/core/src/attachment.rs

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Supported MIME types for attachment upload.
/// The storage layer and API are file-type-agnostic — this list gates what
/// the LLM integration path and resize pipeline currently handle.
/// Expand as new content types are supported (e.g. application/pdf).
pub const SUPPORTED_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
];

/// MIME types that support server-side resizing.
pub const RESIZABLE_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
];

/// Maximum attachment size in bytes (10 MB).
pub const MAX_ATTACHMENT_SIZE: u64 = 10 * 1024 * 1024;

/// Persisted attachment metadata. The actual bytes live on the filesystem.
#[derive(Debug, Clone)]
pub struct AttachmentMeta {
    pub id: Uuid,
    pub message_id: Option<Uuid>,   // None during upload, set when message is sent
    pub conversation_id: Uuid,
    pub agent_id: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
}

impl AttachmentMeta {
    /// File extension derived from mime_type.
    pub fn extension(&self) -> &str {
        match self.mime_type.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "bin",
        }
    }
}

/// Validates that a MIME type is a supported image type.
pub fn is_supported_image_type(mime_type: &str) -> bool {
    SUPPORTED_IMAGE_TYPES.contains(&mime_type)
}
```

The existing `Attachment` struct (in `crates/core/src/tool.rs`) remains unchanged — it's the in-memory DTO for tool outputs carrying bytes.

## Migration — `message_attachments`

```sql
CREATE TABLE IF NOT EXISTS message_attachments (
    id              TEXT PRIMARY KEY,
    message_id      TEXT REFERENCES messages(id),
    conversation_id TEXT NOT NULL,
    agent_id        TEXT NOT NULL,
    filename        TEXT NOT NULL,
    mime_type       TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_attachments_message ON message_attachments(message_id);
CREATE INDEX idx_attachments_conversation ON message_attachments(conversation_id);
```

`message_id` is nullable — attachments are uploaded before the message is sent, then linked when the message is created.

## Storage — `assistant-storage`

```rust
// crates/storage/src/attachments.rs

pub struct AttachmentStore {
    pool: SqlitePool,
}

impl AttachmentStore {
    /// Persist an attachment: write bytes to disk, insert metadata row.
    pub async fn store(
        &self,
        meta: &AttachmentMeta,
        data: &[u8],
    ) -> Result<()>;

    /// Read raw bytes from disk.
    pub async fn load_bytes(&self, id: Uuid) -> Result<Vec<u8>>;

    /// Get metadata by ID.
    pub async fn get_meta(&self, id: Uuid) -> Result<AttachmentMeta>;

    /// List attachments for a message.
    pub async fn list_for_message(&self, message_id: Uuid) -> Result<Vec<AttachmentMeta>>;

    /// List attachments for a conversation (for history replay).
    pub async fn list_for_conversation(&self, conversation_id: Uuid) -> Result<Vec<AttachmentMeta>>;

    /// Link an attachment to a message after the message is created.
    pub async fn link_to_message(&self, attachment_id: Uuid, message_id: Uuid) -> Result<()>;
}
```

### Filesystem Layout

```
~/.assistant/agents/{agent_id}/attachments/{conversation_id}/{attachment_id}.{ext}
```

`AttachmentStore::store()` creates directories as needed via `tokio::fs::create_dir_all`.

### Path Resolution

```rust
fn attachment_path(meta: &AttachmentMeta) -> PathBuf {
    agent_base_dir(&meta.agent_id)       // from assistant_core::context
        .join("attachments")
        .join(meta.conversation_id.to_string())
        .join(format!("{}.{}", meta.id, meta.extension()))
}
```

## Bus Changes — `TurnRequest` and `TurnResult`

```rust
// crates/core/src/bus_messages.rs

pub struct TurnRequest {
    pub prompt: String,
    pub conversation_id: Uuid,
    pub extension_tools: Vec<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub attachment_ids: Vec<Uuid>,          // NEW — user-provided image IDs
}

pub struct TurnResult {
    pub content: String,
    pub attachment_ids: Vec<Uuid>,          // CHANGED — was Vec<Attachment> (bytes)
}
```

Both directions carry IDs only. Default: empty vec. Fully backwards compatible.

## Runtime — Orchestrator Integration

### Inbound (user → LLM)

When the orchestrator receives a `TurnRequest` with non-empty `attachment_ids`:

1. Load each `AttachmentMeta` from `AttachmentStore`
2. Load bytes from disk via `AttachmentStore::load_bytes()`
3. Base64-encode the bytes
4. Build `ChatHistoryMessage::MultimodalUser` with interleaved `ContentBlock::Text` + `ContentBlock::Image` blocks
5. Pass to LLM provider (existing multimodal handling takes over)

For history replay (loading past conversations), the same path applies: attachments linked to messages are loaded and converted to `ContentBlock::Image` when rebuilding the chat history.

### Outbound (tool → user)

When a tool produces attachments in its `ToolOutput`:

1. Orchestrator receives `ToolOutput { attachments: Vec<Attachment> }` (bytes in memory)
2. For each `Attachment`, call `AttachmentStore::store()` — writes bytes to disk, metadata to SQLite
3. Link to the assistant's message via `AttachmentStore::link_to_message()`
4. Collect the returned `AttachmentMeta` IDs into `TurnResult.attachment_ids`
5. Bytes are dropped from memory — only IDs cross the bus

### Interface Delivery (outbound, per adapter)

The `ChannelRunner` receives `TurnResult` with `attachment_ids`. After sending the text reply:

1. For each attachment ID, load metadata from `AttachmentStore`
2. Deliver per-platform:
   - **Web UI / Flutter**: attachment metadata already in SSE event; client fetches via `GET /api/attachments/{id}`
   - **Slack**: load bytes via `AttachmentStore::load_bytes()`, upload via `files.upload` API to the thread
   - **Matrix**: load bytes, upload via `PUT /_matrix/media/.../upload`, send `m.image` event to room

## Web UI API

### Upload: `POST /api/conversations/{id}/attachments`

- **Auth**: Required (existing middleware)
- **Content-Type**: `multipart/form-data`
- **Fields**: `file` (binary), `agent_id` (text, optional — defaults to active agent)
- **Validation**: mime type in `SUPPORTED_IMAGE_TYPES`, size <= `MAX_ATTACHMENT_SIZE`
- **Response**: `201 Created` with `AttachmentMeta` JSON
- **Errors**: `400` (invalid type), `413` (too large)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "conversation_id": "...",
  "agent_id": "default",
  "filename": "screenshot.png",
  "mime_type": "image/png",
  "size_bytes": 245760,
  "created_at": "2026-04-16T10:30:00Z"
}
```

### Serve: `GET /api/attachments/{id}`

- **Auth**: Required (existing middleware)
- **Query params**:
  - `w` (optional) — max width in pixels, e.g. `?w=400`
  - `h` (optional) — max height in pixels, e.g. `?h=300`
  - When both given, image fits within the bounding box preserving aspect ratio
  - When neither given, serves a sensible default (e.g. max 1920px on longest side)
  - Original is always preserved on disk but never served raw — all responses go through the resize pipeline
- **Response**: Image bytes with `Content-Type` header from metadata (resized images keep the same format)
- **Cache headers**:
  - `Cache-Control: private, immutable, max-age=31536000` — content-addressed by UUID + dimensions, never changes
  - `ETag: "{id}-{w}x{h}"` — enables conditional requests
  - Vary on query params
- **Errors**: `404` if not found

#### Resize Cache

Resized variants are cached on disk alongside originals in a `cache/` subdirectory:

```
~/.assistant/agents/{agent_id}/attachments/{conversation_id}/
  {id}.png                          ← original (always preserved)
  cache/
    {id}_300x0.png                  ← thumbnail (w=300, h=auto)
    {id}_1920x0.png                 ← full viewer default
    {id}_0x600.png                  ← height-constrained variant
```

Cache is derived data — can be regenerated from originals at any time. The `image` crate handles resizing. Cache entries are created on first request and served on subsequent requests with no recomputation.

### Send Message (modified): `POST /api/conversations/{id}/messages`

```json
{
  "message": "What's wrong with this UI?",
  "attachment_ids": ["550e8400-e29b-41d4-a716-446655440000"]
}
```

The existing `SendMessageRequest` gains an optional `attachment_ids` array. The handler links attachments to the created message and includes them in the `TurnRequest`.

### SSE Events (modified)

Message events include attachment metadata so the UI can render inline images:

```json
{
  "type": "message",
  "data": {
    "role": "user",
    "content": "What's wrong with this UI?",
    "attachments": [
      {
        "id": "550e8400-...",
        "filename": "screenshot.png",
        "mime_type": "image/png",
        "url": "/api/attachments/550e8400-..."
      }
    ]
  }
}
```

## Interface Adapters

### Slack

The adapter already downloads files from `file_share` events. Change: instead of discarding image files, pass them through `AttachmentStore::store()` and include the attachment IDs in the `TurnRequest`.

### Matrix

The adapter already downloads `m.image` events into `ChannelContent::FileData`. Same change: persist via `AttachmentStore`, attach IDs to `TurnRequest`.

### Common Pattern

Both adapters already produce `ChannelContent::FileData { data, filename, mime_type }` or `ChannelContent::Image { url, caption }`. The `ChannelRunner` (or equivalent dispatch point) can handle the `AttachmentStore` integration in one place rather than duplicating in each adapter.

## Flutter App

### Upload Flow

1. User taps image button in chat input → platform image picker
2. Selected image shown as thumbnail preview in input area
3. On send: `POST /api/conversations/{id}/attachments` (multipart) → get attachment ID
4. Then: `POST /api/conversations/{id}/messages` with `attachment_ids`
5. Alternatively: upload on selection, attach on send (better UX, image is ready when user hits send)

### Rendering

- User messages with attachments: inline thumbnail above/below text, tap to expand
- Chat list thumbnail: `GET /api/attachments/{id}?w=300` — small, fast-loading preview
- Full-size viewer: `GET /api/attachments/{id}?w=1920` — capped resolution on tap-to-expand
- Auth headers on all image requests
- Chat history replay: attachment metadata comes with message list, render accordingly

### OpenAPI Changes

- New `AttachmentMeta` schema
- `SendMessageRequest` gains optional `attachment_ids` array
- New endpoints documented
- Regenerate Flutter client: `make generate-flutter-client`

## Provider-Specific Notes

| Provider  | Image Support   | Format                   | Max Size      | Notes                                                     |
| --------- | --------------- | ------------------------ | ------------- | --------------------------------------------------------- |
| Anthropic | Native vision   | base64 JSON block        | ~5 MB / image | Resize if over limit                                      |
| OpenAI    | Native vision   | data-URI in content      | ~20 MB        | More generous limit                                       |
| Ollama    | Model-dependent | base64 in `images` array | Varies        | Include images; warn user if model may not support vision |

## Sequence: Web UI Upload + Send

```
  Flutter                Web UI                 Storage              Runtime            LLM
  ═══════                ══════                 ═══════              ═══════            ═══
     │                      │                      │                    │                │
     │  POST /attachments   │                      │                    │                │
     │  (multipart: file)   │                      │                    │                │
     │─────────────────────▶│                      │                    │                │
     │                      │  store(meta, bytes)  │                    │                │
     │                      │─────────────────────▶│                    │                │
     │                      │                      │ write disk         │                │
     │                      │                      │ insert row         │                │
     │                      │  201 {id, ...}       │                    │                │
     │◀─────────────────────│                      │                    │                │
     │                      │                      │                    │                │
     │  POST /messages      │                      │                    │                │
     │  {message, [att_id]} │                      │                    │                │
     │─────────────────────▶│                      │                    │                │
     │                      │  link_to_message()   │                    │                │
     │                      │─────────────────────▶│                    │                │
     │                      │                      │                    │                │
     │                      │  TurnRequest { prompt, attachment_ids }   │                │
     │                      │─────────────────────────────────────────▶│                │
     │                      │                      │                    │                │
     │                      │                      │  load_bytes()      │                │
     │                      │                      │◀───────────────────│                │
     │                      │                      │  → Vec<u8>         │                │
     │                      │                      │───────────────────▶│                │
     │                      │                      │                    │                │
     │                      │                      │                    │ MultimodalUser │
     │                      │                      │                    │ [Text, Image]  │
     │                      │                      │                    │───────────────▶│
     │                      │                      │                    │                │
     │  SSE: assistant msg  │                      │                    │  response      │
     │◀─────────────────────│◀─────────────────────────────────────────│◀───────────────│
```
