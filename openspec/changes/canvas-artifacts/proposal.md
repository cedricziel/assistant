## Why

AI assistant UIs have evolved beyond linear chat. Claude.ai Artifacts, ChatGPT Canvas, and similar features let the AI present structured work (code, documents, diagrams) in a persistent side panel rather than burying it in scrolling chat history. Our assistant already defines `Artifact` in the A2A protocol with multi-part content and streaming updates — but neither the chat API nor the Flutter UI surfaces them. This leaves a significant UX gap: tool outputs, generated code, and structured results all render as flat text in the message stream.

## What Changes

- Surface A2A-style artifacts through the chat API as first-class SSE events
- Persist artifacts in SQLite alongside conversations (with version history per turn)
- Render artifacts in the Flutter UI in a side panel with type-specific viewers (code with syntax highlighting, markdown rendered, structured data as tables)
- Degrade gracefully across non-Flutter interfaces (Slack, Matrix, etc.) by formatting artifacts as code blocks, file uploads, or threaded messages

## Non-goals

- Collaborative human+AI co-editing (Level 3 canvas) — too complex for initial scope
- Live HTML/React preview sandboxing — security concerns, limited value for operational assistant use
- Artifact creation from the user side — initial scope is AI-produced artifacts only

## Capabilities

### New Capabilities

- `artifact-rendering`: Display AI-produced artifacts (code, markdown, structured data) in a dedicated panel alongside chat
- `artifact-versioning`: Track artifact evolution across conversation turns with diff view
- `artifact-persistence`: Store artifacts in SQLite with conversation association
- `artifact-streaming`: Stream artifact content via SSE during generation

### Modified Capabilities

- `chat-streaming`: Add `ArtifactEvent` and `ArtifactUpdateEvent` to SSE stream
- `tool-output-display`: Rich tool outputs (file-read, web-fetch, etc.) surface as artifacts instead of inline text
- `interface-formatting`: Each interface (Slack, Matrix, CLI, etc.) renders artifacts in its native best format

## Impact

- `crates/core/src/types.rs` — Add `Artifact` struct mirroring A2A artifact model
- `crates/storage/` — New `artifacts` table, migration, and query layer
- `crates/web-ui/src/` — New `ArtifactEvent`/`ArtifactUpdateEvent` SSE events in chat stream
- `crates/runtime/` — Orchestrator emits artifact events when tools produce structured output
- `crates/tool-executor/` — `ToolOutput` gains artifact metadata (name, type, language)
- `app/lib/features/chat/` — Artifact panel widget, type-specific renderers, version navigation
- `app/lib/features/chat/chat_provider.dart` — `ChatMessage` gains artifacts field
- Interface crates (slack, matrix, etc.) — Per-interface artifact formatting
