## Why

The assistant has a full REST API and an iOS app, but there is no way to quickly fire off a question from the iPhone lock screen or home screen without opening the app, navigating to chat, and typing. The iPhone Action Button (iPhone 15 Pro+) and Siri voice integration offer a zero-friction entry point: press a button, speak a question, hear the answer — all without unlocking or switching apps.

The current `POST /api/conversations/{id}/messages` endpoint returns an SSE stream, which is unusable from Apple Shortcuts and Siri App Intents. A synchronous "ask and wait" endpoint is the missing primitive.

## What Changes

- **New sync API endpoint** `POST /api/quick-message` that creates a conversation, submits a turn, awaits the full `TurnResult`, and returns the answer as plain JSON. Uses the server's currently active persona.
- **Native Swift App Intents** in the iOS Runner that let Siri accept a spoken question and call the sync endpoint. Registered via `AppShortcutsProvider` so phrases like "Ask Assistant about X" work out of the box.
- **Shared Keychain credential bridge** so the Swift Intent code can read the server URL and auth token written by the Flutter app, without requiring the Flutter engine to be running.
- **OpenAPI spec update** for the new endpoint.

## Non-goals

- **Per-request persona routing** — the orchestrator worker is bound to one `agent_id`; supporting per-turn persona overrides requires orchestrator changes that are out of scope. The Intent uses whatever persona is currently active.
- **Audio passthrough** — Siri handles speech-to-text natively; the endpoint accepts text only. No dependency on server-side transcription.
- **Streaming responses** — the endpoint is intentionally synchronous. Streaming is handled by the existing SSE endpoint.
- **Android / non-Apple platforms** — iOS-only for this change.

## Capabilities

### New Capabilities

- `sync-message-api`: Synchronous "fire and forget" message endpoint that creates a conversation, runs a turn, and returns the completed answer as JSON.
- `siri-app-intent`: Native iOS App Intent that accepts a voice question via Siri, calls the sync API, and speaks the response. Discoverable via Action Button and Shortcuts app.
- `ios-keychain-bridge`: Shared Keychain access group so Swift native code and Flutter `flutter_secure_storage` can share server credentials.

### Modified Capabilities

_(none — no existing spec-level requirements change)_

## Impact

- **Rust / `assistant-web-ui`**: New handler + route in `crates/web-ui/src/api/mod.rs`, new request/response types, OpenAPI annotation.
- **Swift / iOS Runner**: New `Intents/` directory with `AskAssistantIntent`, `AppShortcutsProvider`, `AssistantAPIClient`, `KeychainHelper`. `Info.plist` gains `NSSiriUsageDescription`.
- **Flutter / `flutter_secure_storage`**: Configuration change to use a shared Keychain access group (`groupId` in `IOSOptions`).
- **OpenAPI spec**: `openapi.json` updated with the new endpoint.
