## MODIFIED Requirements

### Requirement: User can attach images to messages via the web UI

The web UI SHALL accept image uploads as multipart form data. Uploaded images are persisted to the filesystem with metadata stored in SQLite. Attachments are linked to messages when the message is sent.

#### Scenario: Successful image upload

- **WHEN** the user selects an image file (PNG, JPEG, GIF, or WebP) and sends a message
- **THEN** the client uploads the file to `POST /api/conversations/{id}/attachments`
- **THEN** the server validates the MIME type and size, writes the file to `~/.assistant/agents/{agent_id}/attachments/{conversation_id}/{id}.{ext}`, and inserts metadata into `message_attachments`
- **THEN** the server responds with `201 Created` and the `AttachmentMeta` JSON
- **THEN** the client includes the attachment ID in the subsequent `POST /api/conversations/{id}/messages` request

#### Scenario: Unsupported file type

- **WHEN** the user selects a file with an unsupported MIME type (e.g., `application/zip`, `video/mp4`)
- **THEN** the server responds with `400 Bad Request` and `{"error": "Unsupported file type. Supported: image/png, image/jpeg, image/gif, image/webp, application/pdf, text/plain, text/markdown, text/csv, application/json"}`

#### Scenario: File too large

- **WHEN** the user uploads a file larger than 25 MB
- **THEN** the server responds with `413 Payload Too Large` and `{"error": "File exceeds maximum size of 25 MB"}`

#### Scenario: Upload requires authentication

- **WHEN** an unauthenticated request is made to `POST /api/conversations/{id}/attachments`
- **THEN** the server responds with `401 Unauthorized`
