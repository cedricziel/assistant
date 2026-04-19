## 1. Expand MIME type support and size limit (backend)

- [x] 1.1 Add new MIME types to `SUPPORTED_MIME_TYPES` in `crates/core/src/attachment.rs`: `application/pdf`, `text/plain`, `text/markdown`, `text/csv`, `application/json`
- [x] 1.2 Add `extension_for_mime()` mappings for new types: pdf, txt, md, csv, json
- [x] 1.3 Increase `MAX_ATTACHMENT_SIZE` from 10 MB to 25 MB
- [x] 1.4 Update the upload endpoint error message in `crates/web-ui/src/api/mod.rs` to list all supported types
- [x] 1.5 Update the body size limit on the upload route from 12 MB to ~27 MB to accommodate the new max
- [x] 1.6 Update existing tests in `attachment.rs` for new MIME types and size constant
- [ ] 1.7 Update OpenAPI spec (`openapi.json`) to reflect new supported types and size limit

## 2. Add ContentBlock::Document and provider serialization

- [x] 2.1 Add `ContentBlock::Document { media_type: String, data: String }` variant to `crates/llm/src/client.rs`
- [x] 2.2 Update Anthropic provider (`crates/provider-anthropic/src/provider.rs`) to serialize `ContentBlock::Document` as `{"type": "document", "source": {"type": "base64", "media_type": ..., "data": ...}}`
- [x] 2.3 Update OpenAI provider to handle `ContentBlock::Document` — skip or fall back to text (orchestrator handles text extraction before this point)
- [x] 2.4 Update Ollama provider (`crates/llm/src/client.rs` `build_json_messages`) to skip `ContentBlock::Document` blocks (like vision-disabled image handling)
- [x] 2.5 Add unit tests for new `ContentBlock::Document` serialization in each provider

## 3. Attachment-to-ContentBlock routing in runtime

- [ ] 3.1 Add PDF text extraction dependency (e.g., `pdf-extract`) to `crates/runtime/Cargo.toml` — deferred: PDFs go natively to Anthropic; text extraction fallback for other providers can be added later
- [x] 3.2 Refactor `build_attachment_map()` in `crates/runtime/src/orchestrator/mod.rs` to route by MIME type: images → `ContentBlock::Image`, PDFs → `ContentBlock::Document`, text files → `ContentBlock::Text`
- [x] 3.3 Implement text file inlining: read bytes, decode UTF-8, wrap with `--- file: {filename} ---` delimiters
- [ ] 3.4 Implement PDF text extraction fallback for non-Anthropic providers: extract text and inline as `ContentBlock::Text` — deferred: requires pdf-extract dependency
- [x] 3.5 Handle PDF extraction failure gracefully: inline a placeholder `ContentBlock::Text` with filename and size
- [x] 3.6 Update `history.rs` `messages_to_chat_history()` to handle the new attachment types in the `AttachmentMap` (type now includes `ContentBlock` variant info, not just base64 image data)
- [x] 3.7 Add unit tests for all routing paths: image, PDF (Anthropic vs fallback), text, extraction failure

## 4. Flutter app — expand file picker and attachment display

- [x] 4.1 Change `_pickImages()` in `chat_screen.dart` to use `FileType.custom` with all supported extensions (png, jpg, gif, webp, pdf, txt, md, csv, json) or `FileType.any` with validation
- [x] 4.2 Update `PendingAttachment` display to show file-type icons for non-image attachments (PDF icon, text icon, etc.) instead of thumbnail previews
- [x] 4.3 Update drag-and-drop handler to accept non-image MIME types
- [x] 4.4 Update MIME type validation in the attachment flow to match the expanded server-side list
- [ ] 4.5 Add widget tests for non-image attachment display

## 5. Shared Swift package

- [ ] 5.1 Create local Swift package at `app/packages/AssistantShared/` with `Package.swift`
- [ ] 5.2 Move `KeychainHelper.swift` from `app/ios/Runner/Intents/` into the shared package, updating the access group to use the shared Keychain group
- [ ] 5.3 Move `AssistantAPIClient.swift` into the shared package
- [ ] 5.4 Add conversation listing method to `AssistantAPIClient`: `func listConversations() async throws -> [ConversationSummary]`
- [ ] 5.5 Add persona listing method: `func listPersonas() async throws -> [PersonaSummary]`
- [ ] 5.6 Add streaming multipart upload method: `func uploadAttachment(conversationId: String, fileURL: URL, mimeType: String) async throws -> AttachmentResponse`
- [ ] 5.7 Add send message method: `func sendMessage(conversationId: String, text: String, attachmentIds: [String]) async throws`
- [ ] 5.8 Add create conversation method: `func createConversation(title: String?) async throws -> ConversationResponse`
- [ ] 5.9 Add switch persona method: `func switchPersona(personaId: String) async throws`
- [ ] 5.10 Update Siri Intents target to link against the shared package instead of the local Swift files
- [ ] 5.11 Verify Siri Intents still work with the shared package

## 6. Entitlements and Keychain migration

- [ ] 6.1 Add App Group entitlement (`group.com.cedricziel.assistant`) to iOS main app entitlements (Debug and Release)
- [ ] 6.2 Add shared Keychain access group (`$(AppIdentifierPrefix)com.cedricziel.assistant.shared`) to iOS main app entitlements
- [ ] 6.3 Add matching entitlements to macOS main app entitlements (Debug and Release)
- [ ] 6.4 Update `flutter_secure_storage` configuration in Dart to write credentials using the shared Keychain access group (`iOSOptions` with `groupId`)
- [ ] 6.5 Implement Keychain migration in `context_repository.dart`: on startup, read from old scope, write to shared scope, delete old entries
- [ ] 6.6 Write credentials to both old and new access groups during transition period for backwards compatibility with Siri Intents that may not yet use the shared package
- [ ] 6.7 Add unit test for migration logic

## 7. iOS share extension target

- [ ] 7.1 Add share extension target to `app/ios/Runner.xcodeproj` with bundle ID `$(PRODUCT_BUNDLE_IDENTIFIER).ShareExtension`
- [ ] 7.2 Configure share extension `Info.plist` with `NSExtension` key: `NSExtensionPointIdentifier: com.apple.share-services`, supported UTIs for URLs, images, PDFs, text files
- [ ] 7.3 Add App Group and shared Keychain entitlements to the share extension target
- [ ] 7.4 Link the shared `AssistantShared` Swift package to the share extension target
- [ ] 7.5 Create `ShareViewController.swift` — the extension entry point that hosts the SwiftUI view
- [ ] 7.6 Create `ShareExtensionView.swift` — SwiftUI view with file preview, conversation picker, persona selector, message field, and Send/Cancel buttons
- [ ] 7.7 Implement `NSItemProvider` content extraction: detect URL vs file, extract file data and MIME type, handle HEIC-to-PNG conversion
- [ ] 7.8 Implement the send flow: create conversation (if new) → switch persona (if different) → upload attachment → send message → dismiss
- [ ] 7.9 Add progress indicator during upload and error handling with user-facing messages
- [ ] 7.10 Handle no-credentials state: show setup prompt, disable Send button

## 8. macOS share extension target

- [ ] 8.1 Add share extension target to `app/macos/Runner.xcodeproj` with bundle ID `$(PRODUCT_BUNDLE_IDENTIFIER).ShareExtension`
- [ ] 8.2 Configure share extension `Info.plist` with `NSExtension` for macOS share services and supported UTIs
- [ ] 8.3 Add App Group and shared Keychain entitlements to the macOS share extension target
- [ ] 8.4 Link the shared `AssistantShared` Swift package to the macOS share extension target
- [ ] 8.5 Reuse `ShareExtensionView.swift` from iOS (shared SwiftUI view with platform-adaptive layout)
- [ ] 8.6 Create macOS-specific `ShareViewController.swift` entry point (NSViewController-based for macOS share extensions)
- [ ] 8.7 Verify share extension appears in macOS share menu and functions correctly

## 9. Integration testing and polish

- [ ] 9.1 End-to-end test: share a PDF from Files.app on iOS simulator → verify it appears in the conversation with correct content
- [ ] 9.2 End-to-end test: share a URL from Safari → verify URL appears as message text
- [ ] 9.3 End-to-end test: share an image from Photos → verify image upload and LLM vision
- [ ] 9.4 Test conversation picker: verify new conversation creation and existing conversation selection
- [ ] 9.5 Test persona selector: verify persona switching works from the share extension
- [ ] 9.6 Test error states: no credentials, server unreachable, file too large, unsupported type
- [ ] 9.7 Test on macOS: verify share extension in Finder/Safari share menu
- [ ] 9.8 Run `make lint` and `make format` across all Rust changes
- [ ] 9.9 Run `make lint-flutter` and `make test-flutter` for Flutter changes
- [ ] 9.10 Update `openapi.json` with `make dump-openapi` after all API changes
