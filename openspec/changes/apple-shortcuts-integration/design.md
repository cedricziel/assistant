## Context

The assistant app has a working iOS App Intents integration: a single `AskAssistantIntent` that sends a question to `POST /api/quick-message` and returns the answer via Siri dialog. This lives in `app/ios/Runner/Intents/` as 4 Swift files (`AskAssistantIntent.swift`, `AssistantAPIClient.swift`, `KeychainHelper.swift`, `AppShortcutsProvider.swift`).

macOS has no Shortcuts integration. The macOS runner (`app/macos/Runner/`) is a minimal Flutter wrapper with no native Swift intents.

Credentials flow: Flutter writes the active context's server URL and auth token to well-known Keychain keys (`assistant_siri_server_url`, `assistant_siri_auth_token`) via `flutter_secure_storage`. The Swift code reads these directly from the Keychain using `Security.framework`. This mechanism already works on macOS — `flutter_secure_storage_darwin` is in the macOS Podfile.

The Rust backend exposes REST endpoints for conversations, personas, and workflows — all authenticated via Bearer token.

## Goals / Non-Goals

**Goals:**

- Share all Shortcuts Swift code between iOS and macOS via a single Swift Package.
- Expose App Entities (Persona, Workflow, Conversation) with dynamic queries so Shortcuts can show rich pickers.
- Add new actions: RunWorkflow, ListPersonas, ListWorkflows, ListConversations.
- Enhance AskAssistant to accept an optional persona and context string.
- Extend `POST /api/quick-message` to route to a specific persona without mutating server state.

**Non-Goals:**

- Streaming/SSE responses from Shortcuts (the App Intent model is synchronous).
- Managing the embedded server lifecycle from Shortcuts.
- Siri Domains or SiriKit (using App Intents framework only).

## Decisions

### D1: Swift Package Manager (local package) for shared code

**Decision**: Create `app/packages/AssistantIntents/` as a local SPM package targeting `.iOS(.v16)` and `.macOS(.v13)`.

**Alternatives considered**:

- _Duplicate files in ios/ and macos/_: Simple but leads to drift. The existing iOS code is small (4 files, ~160 lines total), but we're adding entities and new intents — duplication becomes painful.
- _Shared directory with multi-target Xcode setup_: Fragile with Flutter's Xcode project generation.

**Rationale**: SPM packages are the standard Swift code-sharing mechanism. This package has zero external dependencies (Foundation + Security only), so no SPM/CocoaPods conflicts. Both runner targets add it as a local path dependency. The 4 existing iOS files are deleted and absorbed into the package.

### D2: App Entities with `EntityQuery` backed by REST API

**Decision**: Each entity type (`PersonaEntity`, `WorkflowEntity`, `ConversationEntity`) conforms to `AppEntity` and provides a `defaultQuery` that calls the assistant API.

**Alternatives considered**:

- _String-only parameters (no entities)_: Works but gives poor UX — users type raw IDs instead of picking from a list.
- _Static/hardcoded entity lists_: No value; the whole point is dynamic server data.

**Rationale**: App Entities are the native way to make Shortcuts composable. The `EntityQuery` calls `GET /api/personas`, `GET /api/workflows`, or `GET /api/conversations` respectively. If the server is unreachable at editing time, queries return empty arrays (not crashes).

### D3: Extend `quick-message` instead of creating a new endpoint

**Decision**: Add optional `persona_id` (string) and `context` (string) fields to `QuickMessageRequest`. When `persona_id` is set, the handler resolves that persona for the turn instead of using `state.agent_id`. The `context` string is prepended to the user message as context.

**Alternatives considered**:

- _New `/api/shortcuts/ask` endpoint_: Unnecessary endpoint proliferation; the existing endpoint semantics match.
- _Switch server active persona via `POST /api/personas/active`_: Side effect — changes persona for all clients, not just the Shortcut invocation.
- _Use the full `POST /api/conversations/{id}/messages` flow_: Requires creating a conversation first; `quick-message` already handles the create+ask+respond cycle.

**Rationale**: The `quick-message` endpoint is purpose-built for one-shot Q&A. Adding optional fields is backward-compatible — existing clients that send only `message` are unaffected.

### D4: Package structure

```
app/packages/AssistantIntents/
├── Package.swift
├── Sources/AssistantIntents/
│   ├── API/
│   │   ├── AssistantAPIClient.swift    # HTTP client (extended from iOS version)
│   │   └── KeychainHelper.swift        # Keychain reader (from iOS, unchanged)
│   ├── Entities/
│   │   ├── PersonaEntity.swift         # AppEntity + EntityQuery
│   │   ├── WorkflowEntity.swift        # AppEntity + EntityQuery
│   │   └── ConversationEntity.swift    # AppEntity + EntityQuery
│   ├── Intents/
│   │   ├── AskAssistantIntent.swift    # Enhanced with persona + context params
│   │   ├── RunWorkflowIntent.swift     # Triggers workflow test-run
│   │   ├── ListPersonasIntent.swift    # Returns [PersonaEntity]
│   │   ├── ListWorkflowsIntent.swift   # Returns [WorkflowEntity]
│   │   └── ListConversationsIntent.swift
│   └── AssistantShortcutsProvider.swift # Siri phrase registration
└── Tests/AssistantIntentsTests/
    └── APIClientTests.swift            # Unit tests for JSON decoding
```

### D5: RunWorkflow uses authenticated `test-run` endpoint

**Decision**: `RunWorkflowIntent` calls `POST /api/workflows/{id}/test-run` (authenticated, Bearer token) rather than the public webhook endpoint (`POST /workflow-hooks/{id}/{token}`).

**Alternatives considered**:

- _Public webhook URL_: Would require storing/retrieving the webhook token per workflow. The Intent already has the auth token from Keychain.

**Rationale**: The auth token is already available. The `test-run` endpoint returns `{ workflow_id, run_id, status }` which maps cleanly to the Intent result.

## Risks / Trade-offs

- **[Server unreachable at Shortcut-edit time]** → Entity queries return empty arrays. The picker shows "No items" rather than crashing. Users see a helpful message.
- **[Keychain access differences macOS vs iOS]** → Both use `kSecClassGenericPassword` with bundle ID as service. `flutter_secure_storage_darwin` handles both platforms. Mitigation: verify in integration testing.
- **[App Sandbox (macOS debug builds)]** → Debug entitlements enable App Sandbox. Network client/server entitlements are already present. App Intents run in the app process, so no additional entitlements needed.
- **[quick-message persona_id validation]** → If an invalid persona_id is sent, the handler should fall back to the server's active persona (not error). This matches the "optional enhancement" semantics.
- **[SPM + Flutter build integration]** → `flutter build` invokes `xcodebuild` which resolves SPM dependencies automatically. No manual `swift package resolve` step needed. Risk: first build may be slower due to SPM resolution.

## Open Questions

- Should `AskAssistantIntent` return the `conversation_id` as metadata so users can chain it into a "Continue Conversation" action in a future iteration?
- Should entity queries support a `limit` parameter, or always return all items? (Relevant for users with many conversations.)
