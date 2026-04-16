## ADDED Requirements

### Requirement: User can attach images to messages via the web UI

The web UI SHALL accept image uploads as multipart form data. Uploaded images are persisted to the filesystem with metadata stored in SQLite. Attachments are linked to messages when the message is sent.

#### Scenario: Successful image upload

- **WHEN** the user selects an image file (PNG, JPEG, GIF, or WebP) and sends a message
- **THEN** the client uploads the file to `POST /api/conversations/{id}/attachments`
- **THEN** the server validates the MIME type and size, writes the file to `~/.assistant/agents/{agent_id}/attachments/{conversation_id}/{id}.{ext}`, and inserts metadata into `message_attachments`
- **THEN** the server responds with `201 Created` and the `AttachmentMeta` JSON
- **THEN** the client includes the attachment ID in the subsequent `POST /api/conversations/{id}/messages` request

#### Scenario: Unsupported file type

- **WHEN** the user selects a file with an unsupported MIME type (e.g. `application/pdf`, `text/plain`)
- **THEN** the server responds with `400 Bad Request` and `{"error": "Unsupported file type. Supported: image/png, image/jpeg, image/gif, image/webp"}`

#### Scenario: File too large

- **WHEN** the user uploads an image larger than 10 MB
- **THEN** the server responds with `413 Payload Too Large` and `{"error": "File exceeds maximum size of 10 MB"}`

#### Scenario: Upload requires authentication

- **WHEN** an unauthenticated request is made to `POST /api/conversations/{id}/attachments`
- **THEN** the server responds with `401 Unauthorized`

### Requirement: User can attach images to messages via chat interfaces

Slack and Matrix adapters SHALL persist downloaded image attachments via `AttachmentStore` and include their IDs in the `TurnRequest`.

#### Scenario: Image shared in Slack

- **WHEN** a user shares an image file in a Slack channel where the bot is present
- **THEN** the adapter downloads the file via Slack's `files.info` API
- **THEN** the adapter persists the image via `AttachmentStore::store()`
- **THEN** the adapter includes the attachment ID in `TurnRequest.attachment_ids`

#### Scenario: Image shared in Matrix

- **WHEN** a user sends an `m.image` event in a Matrix room
- **THEN** the adapter downloads the image via the `mxc://` URL
- **THEN** the adapter persists the image via `AttachmentStore::store()`
- **THEN** the adapter includes the attachment ID in `TurnRequest.attachment_ids`

### Requirement: Flutter app provides image picker and preview

The Flutter app SHALL provide multiple ways to attach images: an image picker button, drag-and-drop, and paste from clipboard. A thumbnail preview is shown before sending.

#### Scenario: Image selection via picker (mobile)

- **WHEN** the user taps the image button on a mobile device
- **THEN** the platform image picker opens (camera + gallery)
- **THEN** the selected image is shown as a thumbnail preview in the input area
- **THEN** the user can remove the image before sending or send with the message

#### Scenario: Image selection via picker (web/desktop)

- **WHEN** the user taps the image button on web or desktop
- **THEN** a file picker dialog opens filtered to image types
- **THEN** the selected image is shown as a thumbnail preview

#### Scenario: Drag and drop (web/desktop)

- **WHEN** the user drags an image file onto the chat input area
- **THEN** a visual drop zone indicator appears
- **THEN** on drop, the image is shown as a thumbnail preview in the input area

#### Scenario: Paste from clipboard (web/desktop)

- **WHEN** the user pastes while the chat input is focused and the clipboard contains an image
- **THEN** the image is shown as a thumbnail preview in the input area

#### Scenario: Multiple images

- **WHEN** the user selects multiple images before sending (via any input method)
- **THEN** all images are shown as thumbnail previews
- **THEN** all images are uploaded and their IDs included in the message request

#### Scenario: Send with image and text

- **WHEN** the user types a message and attaches an image
- **THEN** both the text and attachment IDs are sent together in the message request
- **THEN** the assistant receives both text and image content

#### Scenario: Send image without text

- **WHEN** the user attaches an image but does not type any text
- **THEN** the message is sent with an empty text and the attachment IDs
- **THEN** the assistant receives the image and can respond to it

### Requirement: Chat interfaces expose attachments in their native way

Each interface adapter SHALL handle inbound attachments using the platform's native attachment mechanism, and expose outbound attachments appropriately per MIME type.

#### Scenario: Slack image attachment inbound

- **WHEN** a user shares an image in Slack (drag-and-drop, paste, or mobile share)
- **THEN** Slack delivers a `file_share` event
- **THEN** the adapter downloads via `files.info` and persists through `AttachmentStore`

#### Scenario: Slack image attachment outbound

- **WHEN** the assistant produces an image attachment
- **THEN** the Slack adapter uploads it via `files.upload` to the thread
- **THEN** the image appears natively in Slack's UI (inline preview, download button)

#### Scenario: Matrix image attachment inbound

- **WHEN** a user sends an image in Matrix (any client's native upload)
- **THEN** Matrix delivers an `m.image` event with `mxc://` URL
- **THEN** the adapter downloads and persists through `AttachmentStore`

#### Scenario: Matrix image attachment outbound

- **WHEN** the assistant produces an image attachment
- **THEN** the Matrix adapter uploads via the media API and sends an `m.image` event
- **THEN** the image appears natively in Matrix clients (inline preview)
