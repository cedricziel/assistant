# Implementation Plan: Cross-Platform Native App Frontend

**Branch**: `005-flutter-frontend` | **Date**: 2026-04-04 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/005-flutter-frontend/spec.md`

## Summary

Replace the existing server-rendered web UI (`assistant-web-ui`) with a Flutter
cross-platform application targeting web (browser) and macOS desktop, consuming the
existing and new JSON REST + SSE streaming API. The backend gains 6 new JSON API
endpoints (personas, skills, traces, logs) and CORS support. The Flutter app is a
new top-level `app/` directory outside the Rust workspace.

## Technical Context

**Language/Version**: Dart 3.x / Flutter stable (app); Rust 2021 edition (backend additions)
**Primary Dependencies**:

- Flutter: `flutter_riverpod` 2.x, `go_router`, `flutter_secure_storage`, `http` (SSE)
- Rust (new): `tower-http` CORS layer (already in workspace), `utoipa` (already present)
  **Storage**: `flutter_secure_storage` (platform keychain) for server profile; no new SQLite tables
  **Testing**: `flutter test` (unit + widget); `flutter test integration_test/`; `cargo test -p assistant-web-ui`
  **Target Platform**: Web (Chrome/Firefox/Safari via WASM) + macOS desktop (native)
  **Project Type**: Cross-platform mobile/desktop/web app + backend API additions
  **Performance Goals**: First streaming token ≤2s on local network; 60fps UI on macOS
  **Constraints**: Plain HTTP support (no forced HTTPS); single active server profile; offline N/A
  **Scale/Scope**: Single-user self-hosted; ~7 screens; 6 new Rust API handlers

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

| Principle                             | Status  | Notes                                                                                                                                                                                                                                         |
| ------------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| I. Crate-First Modularity             | ✅ PASS | Flutter app is outside the Rust workspace (`app/`). New backend endpoints go in `crates/web-ui/src/api/`. No new crates needed — justified: Flutter is a separate technology stack that cannot live in a Rust crate. See Complexity Tracking. |
| II. Trait-Based DI                    | ✅ PASS | New Rust API handlers follow the same pattern as existing `ApiState` — concrete types are contained within the `web-ui` crate boundary. Cross-crate dependencies (storage, orchestrator) are already `Arc<T>`.                                |
| III. Test Discipline                  | ✅ PASS | All new Rust handlers MUST follow the existing `api/mod.rs` test pattern: `StorageLayer::new_in_memory()`, `wiremock` for LLM, `#[tokio::test]`. Flutter widget tests use mock HTTP adapters.                                                 |
| IV. Observability                     | ✅ PASS | All new Rust handlers MUST use `tracing` macros. No `println!`.                                                                                                                                                                               |
| V. Simplicity/YAGNI                   | ✅ PASS | No abstractions beyond what the 5 user stories require. No multi-server aggregation, no offline mode, no plugin system.                                                                                                                       |
| VI. Interface Parity via Orchestrator | ✅ PASS | Flutter talks to the backend via the HTTP API, which routes through the existing `Orchestrator`. No direct Orchestrator coupling from the app.                                                                                                |
| VII. Code Quality Gate                | ✅ PASS | `dart analyze` (zero issues) + `flutter test` (all green) added to CI alongside existing `cargo clippy`/`fmt`/`machete`.                                                                                                                      |
| VIII. Dual-Mode Parity                | ✅ PASS | New API endpoints are added to `web-ui` crate, which works in both single-binary and distributed mode (it calls `orchestrator.submit_turn()` which is mode-agnostic).                                                                         |
| IX. Realtime API-First                | ✅ PASS | This feature is the direct implementation of principle IX. All 5 user stories are consumable via the JSON/SSE API.                                                                                                                            |
| X. Security & Safety Gates            | ✅ PASS | All new endpoints MUST be behind `require_auth` middleware. Token stored in platform keychain via `flutter_secure_storage`. CORS restricted to configured origin.                                                                             |
| XI. DB Migration Discipline           | ✅ PASS | No new database tables required. Existing schema is sufficient.                                                                                                                                                                               |

## Project Structure

### Documentation (this feature)

```text
specs/005-flutter-frontend/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── api-contracts.md # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
app/                                    # NEW: Flutter application
├── lib/
│   ├── main.dart                       # Entry point, ProviderScope, router
│   ├── api/
│   │   ├── client.dart                 # HTTP + SSE client, auth header injection
│   │   ├── models/                     # Dart data classes (ServerProfile, Persona, ...)
│   │   └── endpoints/
│   │       ├── conversations.dart
│   │       ├── personas.dart
│   │       ├── skills.dart
│   │       ├── traces.dart
│   │       └── logs.dart
│   ├── features/
│   │   ├── connection/                 # US2: server profile setup
│   │   │   ├── connection_screen.dart
│   │   │   └── connection_provider.dart
│   │   ├── chat/                       # US1: streaming chat
│   │   │   ├── chat_screen.dart
│   │   │   ├── conversation_list.dart
│   │   │   └── chat_provider.dart
│   │   ├── personas/                   # US3: persona picker
│   │   │   ├── persona_picker.dart
│   │   │   └── personas_provider.dart
│   │   ├── traces/                     # US4: trace viewer
│   │   │   ├── traces_screen.dart
│   │   │   └── traces_provider.dart
│   │   ├── logs/                       # US4: log viewer
│   │   │   ├── logs_screen.dart
│   │   │   └── logs_provider.dart
│   │   └── skills/                     # US5: skill discovery
│   │       ├── skills_screen.dart
│   │       └── skills_provider.dart
│   └── router/
│       └── app_router.dart             # go_router configuration
├── test/
│   ├── unit/                           # API model + client unit tests
│   └── widget/                         # Widget tests per feature
├── integration_test/                   # End-to-end tests (require running server)
├── web/                                # Flutter web target (auto-generated)
├── macos/                              # Flutter macOS target (auto-generated)
└── pubspec.yaml

crates/web-ui/src/api/                  # Backend API additions
├── mod.rs                              # EXISTING: conversation endpoints
├── personas.rs                         # NEW: GET /api/personas, POST /api/personas/active
├── skills.rs                           # NEW: GET /api/personas/{id}/skills
├── traces.rs                           # NEW: GET /api/traces, GET /api/traces/{id}
└── logs.rs                             # NEW: GET /api/logs
```

**Structure Decision**: Flutter app in `app/` at repo root (not inside `crates/`).
Backend additions are new files within the existing `crates/web-ui/src/api/` module,
co-located with the existing conversation API. This follows the `crates/web-ui` crate
boundary for all API handler code.

## Complexity Tracking

| Violation                                   | Why Needed                                                                                                                                           | Simpler Alternative Rejected Because                                                                                     |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Flutter app outside Rust workspace (`app/`) | Flutter/Dart is a separate technology stack with its own toolchain, package manager (pub), and build system. It cannot be expressed as a Rust crate. | A Rust-generated UI (e.g., Leptos/Yew) was considered but does not compile to native macOS desktop — contradicts FR-007. |
