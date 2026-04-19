## ADDED Requirements

### Requirement: Server accepts document and text file attachments

The attachment upload endpoint SHALL accept PDF, plain text, markdown, CSV, and JSON files in addition to existing image types.

#### Scenario: PDF upload accepted

- **WHEN** a user uploads a file with MIME type `application/pdf` to `POST /api/conversations/{id}/attachments`
- **THEN** the server validates the file, stores it on disk, inserts metadata into SQLite
- **THEN** the server responds with `201 Created` and `AttachmentMeta` JSON

#### Scenario: Text file upload accepted

- **WHEN** a user uploads a file with MIME type `text/plain`, `text/markdown`, or `text/csv`
- **THEN** the server accepts and stores the file
- **THEN** the server responds with `201 Created`

#### Scenario: JSON file upload accepted

- **WHEN** a user uploads a file with MIME type `application/json`
- **THEN** the server accepts and stores the file
- **THEN** the server responds with `201 Created`

#### Scenario: Unsupported file type still rejected

- **WHEN** a user uploads a file with an unsupported MIME type (e.g., `application/zip`, `video/mp4`)
- **THEN** the server responds with `400 Bad Request` and `{"error": "Unsupported file type. Supported: image/png, image/jpeg, image/gif, image/webp, application/pdf, text/plain, text/markdown, text/csv, application/json"}`

### Requirement: Document attachments are forwarded to the LLM as appropriate content blocks

The runtime SHALL convert non-image attachments into LLM-compatible content blocks based on their MIME type and the active provider's capabilities.

#### Scenario: PDF sent to Anthropic provider

- **WHEN** a `TurnRequest` includes a PDF attachment and the active provider is Anthropic
- **THEN** the orchestrator loads the PDF bytes from `AttachmentStore`
- **THEN** the orchestrator builds a `ContentBlock::Document { media_type: "application/pdf", data: "<base64>" }`
- **THEN** the Anthropic provider formats it as `{"type": "document", "source": {"type": "base64", "media_type": "application/pdf", "data": "..."}}`

#### Scenario: PDF sent to OpenAI or Ollama provider (text extraction fallback)

- **WHEN** a `TurnRequest` includes a PDF attachment and the active provider is OpenAI or Ollama
- **THEN** the orchestrator extracts text from the PDF on the server
- **THEN** the extracted text is inlined as `ContentBlock::Text` wrapped with `--- file: {filename} ---\n{text}\n--- end file ---`

#### Scenario: PDF text extraction fails

- **WHEN** text extraction from a PDF fails (e.g., scanned image PDF with no OCR)
- **THEN** the orchestrator inlines a `ContentBlock::Text` with `--- file: {filename} (PDF, {size_bytes} bytes — text extraction failed) ---`

#### Scenario: Text file sent to any provider

- **WHEN** a `TurnRequest` includes a text, markdown, CSV, or JSON attachment
- **THEN** the orchestrator reads the file bytes and decodes as UTF-8
- **THEN** the content is inlined as `ContentBlock::Text` wrapped with `--- file: {filename} ---\n{contents}\n--- end file ---`

#### Scenario: History replay with document attachments

- **WHEN** a conversation is loaded that contains messages with document attachments
- **THEN** the orchestrator rebuilds the chat history with the appropriate `ContentBlock` for each attachment type
- **THEN** PDFs use `ContentBlock::Document` on Anthropic, text extraction on others
- **THEN** text files are always inlined as `ContentBlock::Text`

### Requirement: ContentBlock enum supports document content

The `ContentBlock` enum in `assistant-llm` SHALL include a `Document` variant for non-image file content that providers handle natively.

#### Scenario: Document variant construction

- **WHEN** a PDF attachment is loaded for LLM submission
- **THEN** a `ContentBlock::Document { media_type: "application/pdf", data: "<base64>" }` is constructed
- **THEN** providers that support native document blocks serialize it accordingly

#### Scenario: Providers without document support receive text fallback

- **WHEN** a provider does not support native document blocks (OpenAI, Ollama)
- **THEN** the `ContentBlock::Document` is never sent to the provider
- **THEN** the orchestrator converts it to `ContentBlock::Text` with extracted content before provider serialization

### Requirement: Extension-to-MIME mapping covers new file types

The `extension_for_mime()` function SHALL return correct file extensions for all newly supported MIME types.

#### Scenario: PDF extension mapping

- **WHEN** `extension_for_mime("application/pdf")` is called
- **THEN** it returns `"pdf"`

#### Scenario: Text extension mapping

- **WHEN** `extension_for_mime("text/plain")` is called
- **THEN** it returns `"txt"`

#### Scenario: Markdown extension mapping

- **WHEN** `extension_for_mime("text/markdown")` is called
- **THEN** it returns `"md"`

#### Scenario: CSV extension mapping

- **WHEN** `extension_for_mime("text/csv")` is called
- **THEN** it returns `"csv"`

#### Scenario: JSON extension mapping

- **WHEN** `extension_for_mime("application/json")` is called
- **THEN** it returns `"json"`

### Requirement: Flutter app supports picking and displaying non-image attachments

The Flutter app file picker SHALL accept all supported file types, and non-image attachments SHALL display with file-type icons rather than image thumbnails.

#### Scenario: File picker accepts all supported types

- **WHEN** the user taps the attachment button in the chat input
- **THEN** the file picker opens with a filter allowing images, PDFs, text files, markdown, CSV, and JSON
- **THEN** selecting a non-image file adds it to the pending attachments

#### Scenario: Non-image attachment display

- **WHEN** a non-image file is added as a pending attachment
- **THEN** it displays with a file-type icon (e.g., PDF icon, text icon) and the filename
- **THEN** it does NOT attempt to render a thumbnail preview

#### Scenario: Drag-and-drop for non-image files

- **WHEN** the user drags a PDF or text file onto the chat input area on desktop
- **THEN** the file is accepted and added to pending attachments with a file-type icon
