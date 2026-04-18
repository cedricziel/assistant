## 1. Sync Message API (Rust)

- [x] 1.1 Add `QuickMessageRequest` and `QuickMessageResponse` structs with `utoipa::ToSchema` derives in `crates/web-ui/src/api/mod.rs`
- [x] 1.2 Implement `quick_message` handler: create conversation, submit turn, await `TurnResult`, return JSON (201 on success, 400/401/500 on error)
- [x] 1.3 Add auto-title logic (first 57 chars + "..." if truncated) to the handler
- [x] 1.4 Register `POST /api/quick-message` route in `api_router()`
- [x] 1.5 Add `utoipa::path` OpenAPI annotation with `operationId: create_quick_message`, tag `conversations`, request/response schemas, and security requirement
- [x] 1.6 Write unit tests: successful message, empty message (400), orchestrator error (500)
- [x] 1.7 Run `make dump-openapi` to update `openapi.json`

## 2. Keychain Bridge (Flutter)

- [x] 2.1 Identify the exact Keychain service name and attribute keys used by `flutter_secure_storage` on iOS
- [x] 2.2 Update `flutter_secure_storage` calls to include `IOSOptions(groupId: ...)` with the shared Keychain access group
- [x] 2.3 Add migration logic: on app launch, detect if credentials exist without group ID and re-write them with the group ID
- [x] 2.4 Verify `DebugProfile.entitlements` and `Release.entitlements` both contain the `keychain-access-groups` entry

## 3. Swift App Intent

- [x] 3.1 Create `app/ios/Runner/Intents/KeychainHelper.swift` — reads server URL and auth token from shared Keychain using `SecItemCopyMatching`
- [x] 3.2 Create `app/ios/Runner/Intents/AssistantAPIClient.swift` — native `URLSession` client that calls `POST /api/quick-message` with Bearer auth and 25s timeout
- [x] 3.3 Create `app/ios/Runner/Intents/AskAssistantIntent.swift` — `AppIntent` with `@Parameter question: String`, calls `AssistantAPIClient`, returns spoken dialog. Handles timeout ("still working") and missing credentials ("open the app first") gracefully
- [x] 3.4 Create `app/ios/Runner/Intents/AppShortcutsProvider.swift` — registers `AskAssistantIntent` with phrases "Ask {applicationName} about {question}" and "Ask {applicationName} {question}"
- [x] 3.5 Add `NSSiriUsageDescription` to `app/ios/Runner/Info.plist`

## 4. Build & Lint

- [x] 4.1 Run `make lint && make format` to verify Rust changes pass clippy and formatting
- [x] 4.2 Run `make lint-flutter` to verify Flutter changes pass analysis
- [x] 4.3 Verify `flutter build ios --no-codesign` succeeds with the new Swift files
