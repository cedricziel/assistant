## Context

The assistant platform exposes a REST API (`crates/web-ui`) with SSE-streaming message endpoints and a Flutter app targeting macOS, iOS, and web. The iOS app connects to a remote server using a URL + bearer token stored in `flutter_secure_storage` (iOS Keychain).

The existing `POST /api/conversations/{id}/messages` endpoint returns an SSE stream (`text/event-stream`), which is incompatible with Apple Shortcuts and Siri App Intents — both expect a single JSON response. The orchestrator already produces a `TurnResult` with a complete `.answer` string; the SSE layer is a parallel streaming concern layered on top.

The iOS Runner (`app/ios/Runner/`) currently contains only the default `AppDelegate.swift` and `SceneDelegate.swift` — no native Swift features beyond what Flutter generates. The `Release.entitlements` already includes `keychain-access-groups`.

Key constraint: the orchestrator worker (`crates/runtime/src/orchestrator/worker.rs`) is bound to a single `agent_id` at construction and filters the message bus by that ID. Per-request persona routing is not feasible without orchestrator changes, so the sync endpoint uses the server's currently active persona.

## Goals / Non-Goals

**Goals:**

- Add a synchronous REST endpoint that creates a conversation, runs a full assistant turn, and returns the answer as JSON — usable from Apple Shortcuts, curl, webhooks, and any HTTP client.
- Implement a native iOS App Intent (`AskAssistantIntent`) that accepts a spoken question via Siri, calls the sync endpoint, and speaks the response aloud.
- Register the Intent with `AppShortcutsProvider` so it is discoverable via the Shortcuts app, Siri voice activation, and Action Button configuration.
- Bridge credentials between the Flutter app and native Swift code via a shared Keychain access group.

**Non-Goals:**

- Per-request persona routing — requires multi-worker or orchestrator refactoring; deferred.
- Audio passthrough to the server — Siri handles STT natively; no server-side transcription dependency.
- Streaming or partial responses via the sync endpoint.
- Android or non-Apple platform support.
- App Store submission or provisioning profile setup.

## Decisions

### Decision 1: Synchronous endpoint as a thin wrapper over `submit_turn`

**Chosen**: A new handler `quick_message` that calls `ConversationStore::create_conversation()`, then `orchestrator.submit_turn()` directly, awaits the `TurnResult`, and returns JSON. No SSE, no event channels, no streaming infrastructure.

**Rationale**: `submit_turn` already returns a `TurnResult { answer, attachments, message_id }` via a oneshot channel. The SSE streaming in `send_message` is a parallel forwarding layer — the synchronous result is already available. Wrapping it is ~50 lines.

**Alternative considered**: A separate "poll for result" pattern (POST to create, GET to poll) — rejected; adds client complexity and latency for no benefit when the server can simply block.

### Decision 2: Always create a new conversation

**Chosen**: `POST /api/quick-message` always creates a fresh conversation. No `conversation_id` parameter.

**Rationale**: The primary use case is quick fire-and-forget questions from Siri/Shortcuts. Continuing an existing thread requires context the user doesn't have while speaking to Siri. The conversation is persisted and visible in the app for follow-up.

**Alternative considered**: Optional `conversation_id` to continue a thread — rejected for v1; adds complexity and UX confusion in the voice context.

### Decision 3: Active persona only (no per-request override)

**Chosen**: The endpoint reads the shared `agent_id` from `ApiState` (the currently active persona) and creates the conversation under that persona.

**Rationale**: The orchestrator worker filters bus messages by `self.agent_id`. Publishing a turn request with a different `agent_id` would not be picked up by any worker. Supporting per-request persona routing requires either temporarily mutating global state (race-prone) or multi-worker architecture (out of scope). Using the active persona covers the primary use case.

**Alternative considered**: Temporarily lock and switch `agent_id` — rejected due to race conditions with concurrent requests.

### Decision 4: Native Swift App Intent (not Flutter MethodChannel)

**Chosen**: Implement `AskAssistantIntent` as pure Swift code in the iOS Runner with its own HTTP client. No Flutter engine dependency.

**Rationale**: App Intents can run in the background without the app being launched. Starting a Flutter engine for a simple HTTP call adds 1-3 seconds of cold-start latency and memory overhead. A native HTTP client (`URLSession`) is trivial and instant. The Intent only needs two pieces of data: server URL and auth token, both readable from the shared Keychain.

**Alternative considered**: Flutter MethodChannel — rejected; requires the Flutter engine to be alive, which defeats the purpose of background Siri execution.

### Decision 5: Shared Keychain via access group

**Chosen**: Configure `flutter_secure_storage` with `IOSOptions(groupId: '$(AppIdentifierPrefix)$(PRODUCT_BUNDLE_IDENTIFIER)')` so tokens are written to a Keychain access group. Swift code reads from the same group using `SecItemCopyMatching`.

**Rationale**: Both the Flutter app and the Swift Intent code run in the same app bundle, so they share the same Keychain access group by default. The `Release.entitlements` already declares `keychain-access-groups`. The only change needed is telling `flutter_secure_storage` to use the group ID when writing, and having the Swift code use the matching query attributes.

**Alternative considered**: UserDefaults with App Groups — rejected; tokens are sensitive credentials and must not be stored in unencrypted storage.

### Decision 6: Timeout handling with graceful fallback

**Chosen**: The Swift Intent sets a 25-second HTTP timeout (5s buffer before Siri's ~30s limit). On timeout, the Intent returns a spoken dialog: "I'm still working on that. Check the app for the full answer." The server-side turn continues running regardless.

**Rationale**: Complex assistant turns (multi-step tool use) can exceed 30 seconds. The conversation is always persisted server-side, so the answer will be available in the app even if Siri times out. A graceful message is better than Siri's generic error.

**Alternative considered**: Server-side timeout with partial result — rejected; the orchestrator doesn't support mid-turn truncation, and a partial answer is worse than no answer with a redirect.

### Decision 7: Auto-title from message content

**Chosen**: The `quick-message` handler reuses the existing auto-title logic from `send_message` — first 57 characters of the message become the conversation title.

**Rationale**: Conversations created via Siri appear in the app's conversation list. A meaningful title (the question asked) is more useful than "New Chat" for finding them later.

## Risks / Trade-offs

- **Siri timeout for complex turns** → Mitigation: 25s client timeout with graceful fallback message. Conversation persists server-side.
- **Keychain access group mismatch** → Mitigation: Both Flutter and Swift use `$(AppIdentifierPrefix)$(PRODUCT_BUNDLE_IDENTIFIER)`, which is the default group. Unit test to verify credential round-trip.
- **`flutter_secure_storage` groupId change** → Mitigation: If existing tokens were written without a groupId, they may not be readable with the new groupId. On first launch after update, the app may need to re-write tokens. This only affects users who upgrade from a pre-Siri build. Verify with a migration test.
- **No persona selection** → Accepted limitation for v1. The active persona covers the common case. Document that persona must be switched in the app first.
- **Long-running turns block the HTTP connection** → Acceptable; the endpoint is designed for machine clients (Shortcuts, Siri) that expect a single response. No browser-tab concerns.

## Migration Plan

1. Add `POST /api/quick-message` handler to `crates/web-ui/src/api/mod.rs` with OpenAPI annotations.
2. Update `openapi.json` via `make dump-openapi`.
3. Add Swift Intent files to `app/ios/Runner/Intents/`.
4. Update `Info.plist` with `NSSiriUsageDescription`.
5. Configure `flutter_secure_storage` with shared Keychain group.
6. Update CI to build iOS with Intent support.

Rollback: all changes are additive. Removing the Swift files and the API handler reverts to current behavior. No database migrations, no schema changes.

## Open Questions

- Should the `quick-message` response include a `deep_link` URL (e.g., `assistant://chat/{id}`) for clients that want to open the conversation in the app? Low cost to add, but the URL scheme must be registered in `Info.plist`.
- What is the exact Keychain service name used by `flutter_secure_storage` on iOS? Need to verify to match in Swift code.
