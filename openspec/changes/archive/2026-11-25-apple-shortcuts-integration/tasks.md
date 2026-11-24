## 1. Backend: Extend quick-message endpoint

- [x] 1.1 Add optional `persona_id` (String) and `context` (String) fields to `QuickMessageRequest` in `crates/web-ui/src/api/mod.rs`
- [x] 1.2 Update `quick_message` handler to resolve persona from `persona_id` when provided, falling back to `state.agent_id` if absent or invalid
- [x] 1.3 Update `quick_message` handler to prepend `context` to the user message when provided
- [x] 1.4 Update OpenAPI annotations (`utoipa`) on `QuickMessageRequest` to document the new optional fields
- [x] 1.5 Add tests: `quick_message_with_persona_id`, `quick_message_with_invalid_persona_fallback`, `quick_message_with_context`, `quick_message_backward_compatible`
- [x] 1.6 Run `make dump-openapi` to update `openapi.json`

## 2. Swift Package: Scaffold and API layer

- [x] 2.1 Create `app/packages/AssistantIntents/Package.swift` targeting `.iOS(.v16)` and `.macOS(.v13)` with no external dependencies
- [x] 2.2 Move `KeychainHelper.swift` from `app/ios/Runner/Intents/` into `Sources/AssistantIntents/API/`, remove `@available(iOS 16.0, *)` annotation (use package platform constraint instead)
- [x] 2.3 Move `AssistantAPIClient.swift` into `Sources/AssistantIntents/API/`, extend with `listPersonas()`, `listWorkflows()`, `listConversations()`, and `triggerWorkflow(id:)` methods
- [x] 2.4 Add `quickMessage(_:personaId:context:)` overload to `AssistantAPIClient` that sends the new optional fields
- [x] 2.5 Add unit tests in `Tests/AssistantIntentsTests/APIClientTests.swift` for JSON decoding of all response types

## 3. Swift Package: App Entities

- [x] 3.1 Create `PersonaEntity` conforming to `AppEntity` with `EntityQuery` that calls `listPersonas()`, supports `suggestedEntities()` and `entities(matching:)` with case-insensitive substring filter
- [x] 3.2 Create `WorkflowEntity` conforming to `AppEntity` with `EntityQuery` that calls `listWorkflows()`, supports `suggestedEntities()` and `entities(matching:)` with case-insensitive substring filter
- [x] 3.3 Create `ConversationEntity` conforming to `AppEntity` with `EntityQuery` that calls `listConversations()`, supports `suggestedEntities()` and `entities(matching:)` with case-insensitive substring filter

## 4. Swift Package: App Intents

- [x] 4.1 Move `AskAssistantIntent` into package, add optional `persona` (PersonaEntity) and `context` (String) parameters, update `perform()` to pass them through to the API client
- [x] 4.2 Create `RunWorkflowIntent` with required `workflow` (WorkflowEntity) parameter, calls `triggerWorkflow(id:)`, returns dialog with workflow name and run ID
- [x] 4.3 Create `ListPersonasIntent` returning `[PersonaEntity]` via `ReturnsValue`
- [x] 4.4 Create `ListWorkflowsIntent` with optional `activeOnly` (Bool) parameter, returning `[WorkflowEntity]` via `ReturnsValue`
- [x] 4.5 Create `ListConversationsIntent` with optional `limit` (Int, default 20) parameter, returning `[ConversationEntity]` via `ReturnsValue`
- [x] 4.6 Create `AssistantShortcutsProvider` registering Siri phrases for all intents

## 5. Xcode Integration

- [x] 5.1 Add local SPM dependency on `AssistantIntents` to the iOS runner Xcode target
- [x] 5.2 Add local SPM dependency on `AssistantIntents` to the macOS runner Xcode target
- [x] 5.3 Delete `app/ios/Runner/Intents/` directory (all 4 files absorbed into package)
- [ ] 5.4 Verify `flutter build ios` succeeds with the SPM package resolved
- [ ] 5.5 Verify `flutter build macos` succeeds with the SPM package resolved

## 6. Verification and Cleanup

- [ ] 6.1 Verify Keychain service name consistency: confirm `KeychainHelper` uses `Bundle.main.bundleIdentifier` (no hardcoded fallback like `com.example.assistantApp`) and that this matches `flutter_secure_storage_darwin`'s service name on macOS (`com.cedricziel.assistant.macos`) and iOS (`com.cedricziel.assistant`)
- [ ] 6.2 Verify macOS Keychain credential sync: Flutter writes, Swift package reads
- [ ] 6.3 Test AskAssistant via macOS Shortcuts app (with and without persona)
- [ ] 6.4 Test RunWorkflow via macOS Shortcuts app
- [ ] 6.5 Test entity pickers show dynamic data in Shortcuts editor
- [x] 6.6 Run `make lint` and `make format`
- [x] 6.7 Run `make lint-flutter` for both iOS and macOS
