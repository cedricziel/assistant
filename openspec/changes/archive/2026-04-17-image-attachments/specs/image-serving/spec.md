## ADDED Requirements

### Requirement: Images are served via an authenticated API endpoint with on-demand resizing

The web UI SHALL expose `GET /api/attachments/{id}` to serve stored images. The endpoint supports query-parameter-based resizing with disk-cached variants.

#### Scenario: Serve thumbnail

- **WHEN** the client requests `GET /api/attachments/{id}?w=300`
- **THEN** the server loads the attachment metadata from SQLite
- **THEN** if a cached variant `cache/{id}_300x0.{ext}` exists on disk, it is served directly
- **THEN** if no cached variant exists, the server loads the original, resizes to max width 300px (preserving aspect ratio), writes the variant to disk cache, and serves it
- **THEN** the response includes `Content-Type` matching the original format and `Cache-Control: private, immutable, max-age=31536000`

#### Scenario: Serve full viewer image

- **WHEN** the client requests `GET /api/attachments/{id}?w=1920`
- **THEN** the server serves a variant capped at 1920px width, following the same cache logic

#### Scenario: Serve with height constraint

- **WHEN** the client requests `GET /api/attachments/{id}?w=400&h=300`
- **THEN** the server resizes to fit within a 400x300 bounding box, preserving aspect ratio

#### Scenario: No resize params defaults to sensible max

- **WHEN** the client requests `GET /api/attachments/{id}` with no query params
- **THEN** the server serves a variant capped at a sensible default (e.g. 1920px on longest side)
- **THEN** the original file is never served directly to clients

#### Scenario: Conditional request with ETag

- **WHEN** the client sends `If-None-Match: "{id}-300x0"`
- **THEN** the server responds with `304 Not Modified` if the variant exists

#### Scenario: Serving requires authentication

- **WHEN** an unauthenticated request is made to `GET /api/attachments/{id}`
- **THEN** the server responds with `401 Unauthorized`

#### Scenario: Attachment not found

- **WHEN** the client requests an attachment ID that does not exist
- **THEN** the server responds with `404 Not Found`

### Requirement: Resize cache lives on disk alongside originals

Resized variants SHALL be stored in a `cache/` subdirectory within the attachment's conversation directory. Cache entries are derived data and can be regenerated from originals.

#### Scenario: Cache directory layout

- **GIVEN** an original at `~/.assistant/agents/{agent}/attachments/{conv}/{id}.png`
- **WHEN** a `?w=300` variant is generated
- **THEN** it is written to `~/.assistant/agents/{agent}/attachments/{conv}/cache/{id}_300x0.png`

### Requirement: SSE events include attachment metadata

Message SSE events SHALL include attachment metadata so the Flutter app can render inline images without additional API calls for metadata.

#### Scenario: User message with attachment

- **WHEN** a user sends a message with attachments
- **THEN** the SSE message event includes an `attachments` array with `id`, `filename`, `mime_type`, and a relative URL for each attachment

#### Scenario: Assistant message with tool-generated attachment

- **WHEN** the assistant's turn produces attachments from tool outputs
- **THEN** the SSE message event includes the attachment metadata in the same format

### Requirement: Flutter renders images inline in chat

The Flutter app SHALL render attachment images inline in chat message bubbles, with thumbnails in the message flow and a tap-to-expand full-size viewer.

#### Scenario: User message with image

- **WHEN** a user message has attachments
- **THEN** the chat bubble shows a thumbnail loaded from `GET /api/attachments/{id}?w=300`
- **THEN** tapping the thumbnail opens a full-size viewer at `GET /api/attachments/{id}?w=1920`

#### Scenario: Assistant message with generated image

- **WHEN** an assistant message has attachments
- **THEN** the same inline thumbnail and tap-to-expand behavior applies

#### Scenario: Chat history replay

- **WHEN** the user scrolls through or reopens a conversation with image attachments
- **THEN** all images are rendered from the attachment metadata included in the message list response
