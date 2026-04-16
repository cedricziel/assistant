## ADDED Requirements

### Requirement: Tool-generated images are persisted and delivered to users

When a tool (bash, MCP, skill, image generation API) produces image attachments in its `ToolOutput`, the orchestrator SHALL persist them via `AttachmentStore` and include their IDs in `TurnResult`.

#### Scenario: Bash tool generates an image

- **WHEN** a bash command produces an image file (e.g. matplotlib chart, imagemagick output)
- **THEN** the tool handler returns `ToolOutput::success("...").with_attachment(Attachment { ... })`
- **THEN** the orchestrator calls `AttachmentStore::store()` for each attachment
- **THEN** the attachment is linked to the assistant's message
- **THEN** `TurnResult.attachment_ids` includes the stored attachment ID

#### Scenario: MCP tool returns an image

- **WHEN** an MCP tool execution returns image content
- **THEN** the MCP bridge converts it to an `Attachment` on the `ToolOutput`
- **THEN** the same orchestrator persistence flow applies

#### Scenario: Outbound delivery to web UI

- **WHEN** `TurnResult` contains `attachment_ids`
- **THEN** the SSE event for the assistant's message includes attachment metadata
- **THEN** the Flutter app renders the images inline in the assistant's chat bubble

#### Scenario: Outbound delivery to Slack

- **WHEN** `TurnResult` contains `attachment_ids` and the interface is Slack
- **THEN** the Slack adapter loads bytes from `AttachmentStore::load_bytes()`
- **THEN** the adapter uploads the image via Slack's `files.upload` API to the conversation thread

#### Scenario: Outbound delivery to Matrix

- **WHEN** `TurnResult` contains `attachment_ids` and the interface is Matrix
- **THEN** the Matrix adapter loads bytes and uploads via `PUT /_matrix/media/.../upload`
- **THEN** the adapter sends an `m.image` event to the room

### Requirement: TurnResult carries attachment IDs instead of bytes

`TurnResult` SHALL carry `attachment_ids: Vec<Uuid>` instead of `attachments: Vec<Attachment>`. The bus stays lightweight in both directions.

#### Scenario: Backwards compatibility

- **WHEN** a turn produces no attachments
- **THEN** `TurnResult.attachment_ids` is an empty vec
- **THEN** all existing behavior is unchanged

#### Scenario: Attachment type lifecycle

- **GIVEN** a tool returns `ToolOutput` with `attachments: Vec<Attachment>` (bytes in memory)
- **WHEN** the orchestrator processes the tool result
- **THEN** each `Attachment` is persisted via `AttachmentStore::store()`, producing an `AttachmentMeta`
- **THEN** the raw bytes are dropped from memory
- **THEN** only `AttachmentMeta` IDs are carried forward on `TurnResult`
