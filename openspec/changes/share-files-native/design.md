## Context

The assistant app runs on iOS, iPadOS, and macOS as a Flutter application with native Swift extensions (Siri Intents). The backend is a remote Rust server in 95% of deployments. Existing infrastructure includes:

- **Attachment pipeline**: multipart upload → filesystem storage → SQLite metadata → base64-encoded `ContentBlock::Image` for LLM history replay. Currently gated to 4 image MIME types and 10 MB.
- **Siri Intents Swift code**: `KeychainHelper` reads `flutter_secure_storage` credentials; `AssistantAPIClient` makes HTTP requests to the remote server. Both live in `app/ios/Runner/Intents/`.
- **Keychain credential sharing**: The Flutter app writes `assistant_siri_server_url` and `assistant_siri_auth_token` to the Keychain; native Swift code reads them for API calls.
- **API surface**: `GET /api/conversations`, `GET /api/personas`, `POST /api/conversations`, `POST /api/conversations/{id}/attachments`, `POST /api/conversations/{id}/messages` — all endpoints the share extension needs already exist.

The share extension is a **separate process** with its own bundle ID and ~120 MB memory limit on iOS. It cannot access the main app's Keychain items without a shared Keychain access group.

## Goals / Non-Goals

**Goals:**

- Enable sharing any supported file type (images, PDFs, text, markdown, CSV, JSON) or URL from any app into an assistant conversation via the native share sheet
- One SwiftUI view shared between iOS and macOS share extension targets
- Extension talks directly to the remote API (no app switching required)
- User can choose a target conversation (new or existing) and a persona
- Streaming multipart upload to handle large files within memory limits

**Non-Goals:**

- Offline queueing or background sync
- Deep linking into the conversation after sharing
- Android or web share target support
- Video/audio file type support
- Share extension for sending content _from_ the assistant to other apps

## Decisions

### D1: Extension talks directly to the remote API (Option B)

The share extension makes HTTP requests to the assistant server, rather than staging files to an App Group container for the main app to pick up.

**Why over Option A (stage + open app):** The user said the server is remote in 95% of cases. Staging to disk and opening the app interrupts the user's workflow. Direct API calls let the user share and return to their previous app instantly.

**Why over Option C (stage + background sync):** Creates confusion — "I shared it, where is it?" — and adds complexity for deferred sync without clear benefit.

**Trade-off:** The extension needs network access and server credentials. Credential sharing is already solved by the Keychain pattern established for Siri Intents.

### D2: Shared Swift package for common code

Extract `KeychainHelper` and `AssistantAPIClient` from `app/ios/Runner/Intents/` into a local Swift package (e.g., `app/packages/AssistantShared/`) that both the Intents target and the share extension target link against.

**Why:** Both targets need identical credential reading and HTTP client logic. Duplicating the code would create divergence risk. A local Swift package is the simplest way to share code between extension targets in an Xcode project.

**Alternative considered:** Embedding the shared code as an Xcode framework. Rejected because local Swift packages have less build configuration overhead and integrate naturally with both Flutter's Xcode project and CocoaPods.

### D3: Shared Keychain access group for credential sharing

Add a shared Keychain access group (e.g., `$(AppIdentifierPrefix)com.cedricziel.assistant.shared`) to the main app and share extension entitlements. Migrate the `flutter_secure_storage` writes to use this access group so the extension process can read credentials.

**Why:** iOS/macOS Keychain items are scoped to the writing process's bundle ID by default. The Siri Intents code works today because it runs in-process. The share extension runs as a separate binary and cannot read the main app's Keychain items without a shared group.

**Migration:** On first launch after the update, the Flutter app reads existing Keychain entries with the old access group and re-writes them with the shared group. Old entries are deleted after successful migration. This is a one-time operation.

### D4: ContentBlock::Document for PDFs, text inlining for text files

```
ContentBlock enum:
  ::Text(String)                        ← existing
  ::Image { media_type, data }          ← existing
  ::Document { media_type, data }       ← new
```

Attachment routing in `build_attachment_map()`:

| MIME type                    | ContentBlock                                          | Provider handling                                                                                        |
| ---------------------------- | ----------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `image/*`                    | `::Image` (resize + base64)                           | All providers (existing)                                                                                 |
| `application/pdf`            | `::Document` (base64, no resize)                      | Anthropic: native `"type":"document"`. OpenAI/Ollama: extract text via `pdf-extract`, inline as `::Text` |
| `text/*`, `application/json` | `::Text` (UTF-8 decode, wrapped with filename header) | All providers (universal)                                                                                |

**Why not a single `::File` variant for everything?** Different LLM providers have fundamentally different capabilities. Anthropic supports native PDF document blocks. OpenAI and Ollama do not. The three-variant model lets each provider implementation handle what it natively supports while the routing layer handles fallbacks.

**Text wrapping format:** Text files are inlined as `--- file: {filename} ---\n{contents}\n--- end file ---` to give the LLM clear boundaries when multiple files are attached.

### D5: Streaming multipart upload from extension

Use `URLSession.uploadTask(withStreamedRequest:)` to stream file bytes from the NSItemProvider directly to the server without buffering the entire file in memory.

**Why:** iOS share extensions have a ~120 MB memory limit. A 25 MB file buffered in memory alongside the SwiftUI UI, conversation list, and persona list could approach the limit. Streaming avoids this.

**Trade-off:** Slightly more complex Swift code than a simple `Data` upload, but the memory safety is worth it for reliability.

### D6: URL items shared as message text

When the share sheet provides a URL (e.g., from Safari), include it in the message text rather than attempting to download and upload the page content.

**Why:** The assistant already has tools to fetch URLs. Downloading in the extension adds latency, complexity, and may fail for authenticated pages. The user explicitly preferred this approach.

### D7: App Group for non-credential shared state (future-proofing)

Add an App Group entitlement (`group.com.cedricziel.assistant`) even though credential sharing uses the Keychain. The App Group provides a shared filesystem container that could be used later for offline queueing, caching conversation lists, or passing large files between processes.

**Why now:** Adding entitlements later requires a new provisioning profile and app update. Adding it now is zero-cost and avoids a future migration.

### D8: Conversation picker fetches from API with caching

The share extension fetches the recent conversation list from `GET /api/conversations` on open. Results are cached in the App Group's `UserDefaults` so subsequent opens within the same session are instant. The persona list is fetched from `GET /api/personas` similarly.

**Why not pre-caching from the main app?** Adds inter-process coordination complexity. The API call is lightweight (returns an array of summaries) and completes in under a second on typical connections. Caching within the extension session handles the "user opens share sheet twice quickly" case.

**Fallback:** If the API is unreachable, the extension shows only "New conversation" and uses the server's default persona. An error banner explains the connection issue.

## Risks / Trade-offs

**[Risk] Keychain migration breaks existing Siri Intents** → Write credentials to both the old and new access groups during a transition period. Remove the old group write path after one release cycle.

**[Risk] Extension killed by iOS for exceeding memory limit** → Streaming upload (D5) mitigates this. Add memory monitoring and abort gracefully with user-facing error if approaching the limit.

**[Risk] Server unreachable when share sheet opens** → Show "New conversation" as only option with error banner. Let user type a message and retry. Don't block the UI on the network call.

**[Risk] Large PDF text extraction is slow** → Text extraction from PDFs (for non-Anthropic providers) runs on the server, not in the extension. The orchestrator handles this asynchronously during the ReAct loop. If extraction fails, the LLM receives a `ContentBlock::Text` saying "Attached file: {filename} (PDF, {size})" as a graceful degradation.

**[Risk] flutter_secure_storage access group customization** → The `flutter_secure_storage` package supports custom `kSecAttrAccessGroup` via its iOS options. Verify this works with the shared group before implementation.

**[Trade-off] No offline support** → Users on poor connections may fail to share. This is acceptable for v1 given the 95% remote server topology. Offline queueing can be added later using the App Group filesystem container (D7).

## Open Questions

- **PDF text extraction library**: `pdf-extract` vs `lopdf` for server-side PDF-to-text fallback on non-Anthropic providers. Need to evaluate quality and performance.
- **macOS share extension bundling**: Flutter's macOS build uses a different Xcode project structure than iOS. Need to verify share extension target integration with `flutter build macos`.
- **Conversation creation with persona**: `POST /api/conversations` currently only accepts `title`. The share extension needs to create a conversation targeting a specific persona. May need to add an optional `agent_id` field to `CreateConversationRequest`, or rely on `POST /api/personas/active` to switch before creating.
