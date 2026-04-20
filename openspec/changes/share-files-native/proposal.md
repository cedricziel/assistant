## Why

Users currently can only attach images through in-app mechanisms (file picker, drag-and-drop, clipboard paste). There is no way to share content from other apps — a PDF in Safari, a markdown file in Files, a link in Mail — into an assistant conversation without manually copying text or switching apps. On Apple platforms, the share sheet is the universal gesture for this, and its absence makes the assistant feel disconnected from the rest of the OS.

## What Changes

- **Expand supported attachment types** beyond the four image MIME types to include PDFs, plain text, markdown, CSV, and JSON files. Text-based files are inlined as text content for the LLM; PDFs use native document content blocks where the provider supports it.
- **Add `ContentBlock::Document` variant** to the LLM content model so PDF attachments can be sent natively to Anthropic (with text-extraction fallback for providers that lack document support).
- **Increase the attachment size limit** from 10 MB to 25 MB to accommodate typical document sizes.
- **Build a native iOS/macOS share extension** (Swift/SwiftUI) that appears in the system share sheet. The extension talks directly to the remote assistant API — fetching the conversation list and persona list, uploading the shared file, and sending a message — without opening the main app.
- **Share extension UI**: file preview, conversation picker (new conversation default, recent conversations listed), persona selector, and an optional message field.
- **Extract shared Swift infrastructure** (`KeychainHelper`, `AssistantAPIClient`) from the Siri Intents code into a shared Swift package so both the Intents target and the share extension can reuse it.
- **Add App Group and shared Keychain entitlements** on both iOS and macOS so the extension process can read server credentials written by the Flutter app.
- **Flutter file picker** expanded from image-only to all supported attachment types, with file-type icons for non-image attachments.

## Non-goals

- **Offline queueing** — the extension requires network access to the remote server. Offline share-and-sync-later is out of scope.
- **Deep link back into conversation** — after sharing, the extension dismisses. Opening the app to the specific conversation is a future enhancement.
- **Video or audio file support** — only document and text file types are added. Media beyond images is out of scope.
- **Receiving shares on Android or web** — Apple platforms only (iOS, iPadOS, macOS).

## Capabilities

### New Capabilities

- `share-extension`: Native iOS/macOS share extension with SwiftUI UI, conversation picker, persona selector, streaming file upload, and credential sharing via App Group + shared Keychain.
- `document-attachments`: Expand attachment pipeline to accept non-image file types (PDF, text, markdown, CSV, JSON), route them to the correct `ContentBlock` variant, and handle provider-specific serialization.

### Modified Capabilities

- `image-upload`: Attachment size limit increases from 10 MB to 25 MB. MIME type allowlist expands. `extension_for_mime()` gains new mappings. The spec scenarios for "unsupported file type" and "file too large" change thresholds.
- `image-vision`: `ContentBlock` enum gains a `Document` variant. `build_attachment_map()` routes non-image attachments to `ContentBlock::Document` or inlined `ContentBlock::Text` instead of skipping them. Provider serialization handles the new variant.

## Impact

- **Rust crates**: `assistant-core` (MIME list, size limit), `assistant-llm` (ContentBlock enum), `assistant-runtime` (attachment map routing, history replay), `assistant-provider-anthropic` (document block serialization), `assistant-provider-openai`/`assistant-provider-ollama` (text-extraction fallback), `assistant-web-ui` (upload endpoint validation).
- **Flutter app**: File picker filter, attachment display (file-type icons), `pubspec.yaml` if new dependencies needed.
- **Native iOS/macOS**: New share extension targets in both Xcode projects, shared Swift package, entitlements changes, `Info.plist` updates for supported content types.
- **OpenAPI spec**: No new endpoints required — the share extension uses existing `GET /api/conversations`, `GET /api/personas`, `POST /api/conversations`, `POST /api/conversations/{id}/attachments`, `POST /api/conversations/{id}/messages`.
