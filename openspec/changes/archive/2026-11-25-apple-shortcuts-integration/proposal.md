## Why

The assistant app already supports iOS App Intents (Siri / Shortcuts) for quick Q&A, but macOS has no Shortcuts integration at all. macOS users cannot trigger assistant workflows, ask questions, or browse entities from Apple Shortcuts or Siri. Exposing the assistant to the Apple Shortcuts system on both platforms — with richer actions — lets users compose assistant capabilities into personal automations ("When I arrive at work, run my morning briefing", "Ask my code reviewer about [clipboard]").

## What Changes

- **New Swift Package `AssistantIntents`** — a local SPM package (`app/packages/AssistantIntents/`) containing all App Intents, App Entities, and a lightweight API client. Targets both iOS 16+ and macOS 13+. Replaces the current iOS-only `app/ios/Runner/Intents/` files.
- **App Entities** — `PersonaEntity`, `WorkflowEntity`, and `ConversationEntity` with `EntityQuery` implementations that fetch live data from the server API. Enables rich pickers and type-safe piping between Shortcut actions.
- **New Shortcut actions**:
  - `AskAssistant` (enhanced) — now accepts optional persona and context parameters
  - `RunWorkflow` — triggers a workflow by entity, returns run status
  - `ListPersonas` / `ListWorkflows` / `ListConversations` — query actions returning entity arrays for composition
- **Backend API enhancement** — extend `POST /api/quick-message` to accept optional `persona_id` and `context` fields so Shortcuts can route messages to specific personas with additional context without mutating server state.
- **Unified `AppShortcutsProvider`** — registers discoverable Siri phrases for all actions across both platforms.

## Non-goals

- Interactive / streaming conversation from within Shortcuts (SSE is not compatible with the sync App Intent model).
- Launching or managing the embedded server from Shortcuts — server availability is the user's responsibility.
- watchOS or tvOS support.
- Deep-link into specific conversation screens from Shortcut results (future enhancement).

## Capabilities

### New Capabilities

- `shortcuts-swift-package`: Cross-platform Swift Package containing App Intents, App Entities, API client, and Keychain helper. Shared by iOS and macOS runner targets.
- `shortcuts-app-entities`: PersonaEntity, WorkflowEntity, and ConversationEntity with dynamic queries backed by the assistant REST API.
- `shortcuts-actions`: The set of App Intent actions (AskAssistant, RunWorkflow, list queries) and their Siri phrase registrations.

### Modified Capabilities

- `context-management`: The `syncSiriCredentials()` mechanism is unchanged but must work correctly on macOS (already uses `flutter_secure_storage_darwin`; needs verification).

## Impact

- **Swift / Xcode**: Both `ios/Runner` and `macos/Runner` Xcode targets gain a local SPM dependency on `AssistantIntents`. The 4 files in `app/ios/Runner/Intents/` are removed (absorbed into the package).
- **Rust backend** (`crates/web-ui`): `QuickMessageRequest` struct gains two optional fields (`persona_id`, `context`). The handler routes to the specified persona instead of always using the server's active one. OpenAPI spec updated.
- **Flutter app**: No Dart code changes expected. The existing `syncSiriCredentials()` in `ContextRepository` already writes to Keychain keys that the Swift package reads.
- **Build**: `flutter build macos` and `flutter build ios` must resolve the local SPM package. CI may need Xcode version pinning for App Intents availability.
