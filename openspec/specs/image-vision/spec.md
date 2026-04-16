## ADDED Requirements

### Requirement: User-provided images are forwarded to the LLM as vision content

The runtime SHALL convert persisted image attachments into `ContentBlock::Image` blocks when building the LLM request, enabling vision-capable models to see and reason about user-provided images.

#### Scenario: Image sent to Anthropic provider

- **WHEN** a `TurnRequest` includes `attachment_ids` and the active provider is Anthropic
- **THEN** the orchestrator loads the image bytes from `AttachmentStore`
- **THEN** the orchestrator builds a `ChatHistoryMessage::MultimodalUser` with `ContentBlock::Text` and `ContentBlock::Image` (base64, with media_type)
- **THEN** the Anthropic provider formats it as a `{"type": "image", "source": {"type": "base64", ...}}` block

#### Scenario: Image sent to OpenAI provider

- **WHEN** a `TurnRequest` includes `attachment_ids` and the active provider is OpenAI
- **THEN** the orchestrator loads and encodes the image as above
- **THEN** the OpenAI provider formats it as a `data:{media_type};base64,...` data-URI

#### Scenario: Image sent to Ollama provider

- **WHEN** a `TurnRequest` includes `attachment_ids` and the active provider is Ollama
- **THEN** the orchestrator includes the image in the request
- **THEN** the runtime emits a `warn!` log: "Ollama model may not support vision. Image attachments included but may be ignored by the model."

#### Scenario: Image resize for provider limits

- **WHEN** an image exceeds a provider's size limit (e.g. Anthropic ~5 MB)
- **THEN** the runtime resizes the image to fit within the provider's limit before encoding
- **THEN** the original file on disk is NOT modified — resizing is transient, in-memory only

#### Scenario: History replay with images

- **WHEN** a conversation is loaded that contains messages with linked attachments
- **THEN** the orchestrator rebuilds the chat history with `ContentBlock::Image` blocks for each attachment
- **THEN** the LLM receives the full multimodal conversation context

### Requirement: TurnRequest carries attachment IDs

The `TurnRequest` message bus type SHALL include an `attachment_ids: Vec<Uuid>` field for lightweight attachment references.

#### Scenario: Text-only message (backwards compatible)

- **WHEN** a message is sent without attachments
- **THEN** `TurnRequest.attachment_ids` is an empty vec
- **THEN** the orchestrator processes the message as text-only, identical to current behavior
