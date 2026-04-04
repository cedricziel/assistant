---
description: "Task list for 005-flutter-frontend"
---

# Tasks: Cross-Platform Native App Frontend

**Input**: Design documents from `/specs/005-flutter-frontend/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1–US5)
- Paths assume repo root

## Path Conventions

- Flutter app: `app/lib/`, `app/test/`
- New Rust API handlers: `crates/web-ui/src/api/`
- Backend registration: `crates/web-ui/src/main.rs`

---

## Phase 1: Setup

**Purpose**: Create the Flutter project scaffold and shared infrastructure.

- [x] T001 Create Flutter project in `app/` with web and macOS targets enabled (`flutter create --platforms=web,macos app`)
- [x] T002 Configure `app/pubspec.yaml` with dependencies: `flutter_riverpod`, `go_router`, `flutter_secure_storage`, `http`
- [x] T003 [P] Add CORS `tower-http` layer to `crates/web-ui/src/main.rs` — emit `Access-Control-Allow-Origin` on all `/api/*` routes with configurable origin via `--cors-origin` flag / `ASSISTANT_WEB_CORS_ORIGIN` env var
- [x] T004 [P] Scaffold `app/lib/router/app_router.dart` with `go_router` — placeholder routes for `/setup`, `/chat`, `/personas`, `/traces`, `/logs`, `/skills`
- [x] T005 [P] Add Flutter CI job to GitHub Actions (`.github/workflows/`): `dart analyze` (zero issues) + `flutter test`

**Checkpoint**: `flutter pub get` succeeds; `dart analyze` passes; `make lint` still passes with new CORS code.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Flutter API client layer and shared data models — required by every user story.

- [x] T006 Implement `app/lib/api/client.dart` — `AssistantClient` class: HTTP client with `Authorization: Bearer` injection, JSON deserialization helper, and SSE stream parser (`parseSSE(Stream<List<int>>)` → `Stream<StreamEvent>`)
- [x] T007 [P] Implement `app/lib/api/models/server_profile.dart` — `ServerProfile` data class (baseUrl, token, label) with JSON serialisation
- [x] T008 [P] Implement `app/lib/api/models/conversation.dart` — `ConversationSummary`, `ConversationDetail`, `Message` data classes
- [x] T009 [P] Implement `app/lib/api/models/stream_event.dart` — sealed `StreamEvent` hierarchy: `TokenEvent`, `DoneEvent`, `ErrorEvent`
- [x] T010 [P] Implement `app/lib/api/models/persona.dart` — `Persona` data class
- [x] T011 [P] Implement `app/lib/api/models/skill.dart` — `Skill` data class
- [x] T012 [P] Implement `app/lib/api/models/trace.dart` — `TraceSummary`, `TraceDetail`, `SpanEntry` data classes
- [x] T013 [P] Implement `app/lib/api/models/log_entry.dart` — `LogEntry` data class
- [x] T014 Implement `app/lib/api/endpoints/conversations.dart` — `ConversationsEndpoint`: `list()`, `create()`, `get()`, `delete()`, `rename()`, `sendMessage()` (returns `Stream<StreamEvent>`)

**Checkpoint**: Foundation ready — `flutter test test/unit/api/` passes. All user story implementations can now begin.

---

## Phase 3: User Story 2 — Server Connection & Profile Setup (Priority: P2)

**Goal**: Users can configure a server connection, validate it, and have credentials persisted across restarts. This phase gates all other stories — end-to-end testing of any other story requires this to be complete first.

**Independent Test**: Fresh install → enter `http://127.0.0.1:8080` + `dev-token` → tap Connect → app navigates to chat screen → re-launch → app goes directly to chat.

- [x] T015 [P] [US2] Implement `app/lib/features/connection/connection_provider.dart` — `ServerProfileNotifier` (Riverpod `AsyncNotifier`): load/save profile via `flutter_secure_storage`; expose `connect(baseUrl, token)` that calls `GET /health` to validate
- [x] T016 [P] [US2] Implement `app/lib/features/connection/connection_screen.dart` — form: server URL field, token field, Connect button; shows specific error on failure (`401` → "Invalid token", network error → "Server unreachable")
- [x] T017 [US2] Wire `connection_provider` and `connection_screen` into `app/lib/router/app_router.dart`: redirect unauthenticated users to `/setup`; redirect authenticated users away from `/setup` to `/chat`

**Checkpoint**: US2 independently testable — connection screen appears on fresh install, validates credentials, persists across restarts, shows actionable errors.

---

## Phase 4: User Story 1 — Real-Time Chat with the Assistant (Priority: P1)

**Goal**: Users can send messages and see streaming responses token-by-token, with tool call indicators and conversation history.

**Independent Test**: With US2 complete, open app → navigate to chat → create conversation → send "Hello" → response streams token by token → conversation appears in sidebar list → reopen conversation → history visible.

- [x] T018 [P] [US1] Implement `app/lib/features/chat/chat_provider.dart` — `ConversationListNotifier` (list + CRUD) and `ChatNotifier` (active conversation + `StreamProvider` for SSE stream; accumulates `TokenEvent` chunks; finalises on `DoneEvent`)
- [x] T019 [P] [US1] Implement `app/lib/features/chat/conversation_list.dart` — sidebar/panel widget: list of `ConversationSummary`, new chat button, tap to open, swipe-to-delete
- [x] T020 [US1] Implement `app/lib/features/chat/chat_screen.dart` — message list (streaming assistant bubble updates in place), text input + send button, tool call progress indicator (shown while `TokenEvent` stream is open before `DoneEvent`), error banner on stream failure
- [x] T021 [US1] Wire chat into `app/lib/router/app_router.dart`: `/chat` (no conversation), `/chat/:id` (specific conversation); update `app/lib/main.dart` to use `ProviderScope` + router

**Checkpoint**: US1 independently testable — streaming chat works end-to-end from a running backend; conversation list persists; tool call indicator appears mid-stream.

---

## Phase 5: User Story 3 — Persona Selection & Switching (Priority: P3)

**Goal**: Users can list personas from the server, switch the active persona, and see it reflected in the chat interface.

**Independent Test**: With US1 complete, open persona picker → multiple personas listed → switch → return to chat → active persona name shown → new conversation uses switched persona.

- [x] T022 Implement `crates/web-ui/src/api/personas.rs` — `GET /api/personas` (list all) and `POST /api/personas/active` (switch); add to OpenAPI doc; tests using `StorageLayer::new_in_memory()`
- [x] T023 Register persona routes in `crates/web-ui/src/main.rs` under the auth-protected scope
- [x] T024 [P] [US3] Implement `app/lib/api/endpoints/personas.dart` — `PersonasEndpoint`: `list()`, `setActive(id)`
- [x] T025 [P] [US3] Implement `app/lib/features/personas/personas_provider.dart` — `PersonasNotifier`: fetch list on mount; `switchPersona(id)` calls `POST /api/personas/active` and updates local active persona state
- [x] T026 [US3] Implement `app/lib/features/personas/persona_picker.dart` — modal bottom sheet or drawer panel: persona list with name + description, active indicator, tap to switch
- [x] T027 [US3] Integrate persona picker into `app/lib/features/chat/chat_screen.dart`: active persona name shown in app bar; tap opens `persona_picker.dart`

**Checkpoint**: US3 independently testable — persona picker opens from chat, switching updates the active persona label and routes subsequent messages correctly.

---

## Phase 6: User Story 4 — Observability: Traces & Logs (Priority: P4)

**Goal**: Operators can view recent traces with span breakdowns and filter logs by keyword.

**Independent Test**: Navigate to traces screen → 50 most-recent traces listed → expand one → spans with durations visible → navigate to logs → type keyword → list filters in real time.

- [x] T028 Implement `crates/web-ui/src/api/traces.rs` — `GET /api/traces` (with query params: limit, offset, since, until, skill, status, conversation) and `GET /api/traces/{trace_id}`; add to OpenAPI doc; tests using `StorageLayer::new_in_memory()`
- [x] T029 Implement `crates/web-ui/src/api/logs.rs` — `GET /api/logs` (with query params: limit, offset, search, severity, since, until, trace_id, conversation); add to OpenAPI doc; tests using `StorageLayer::new_in_memory()`
- [x] T030 Register traces + logs routes in `crates/web-ui/src/main.rs` under the auth-protected scope
- [x] T031 [P] [US4] Implement `app/lib/api/endpoints/traces.dart` — `TracesEndpoint`: `list({filters})`, `get(traceId)`
- [x] T032 [P] [US4] Implement `app/lib/api/endpoints/logs.dart` — `LogsEndpoint`: `list({filters})`
- [x] T033 [P] [US4] Implement `app/lib/features/traces/traces_provider.dart` — `TracesNotifier`: paginated list, expandable detail; `TraceDetailNotifier`: fetch single trace with spans
- [x] T034 [P] [US4] Implement `app/lib/features/logs/logs_provider.dart` — `LogsNotifier`: paginated list, debounced keyword filter state
- [x] T035 [P] [US4] Implement `app/lib/features/traces/traces_screen.dart` — list of `TraceSummary` rows (timestamp, persona, duration, status); expandable detail showing `SpanEntry` list with individual durations
- [x] T036 [P] [US4] Implement `app/lib/features/logs/logs_screen.dart` — list of `LogEntry` rows (timestamp, severity chip, target, message); keyword filter text field at top
- [x] T037 [US4] Wire traces + logs into navigation: add `/traces` and `/logs` routes in `app/lib/router/app_router.dart`; add navigation entries (drawer or bottom nav)

**Checkpoint**: US4 independently testable — both screens load data from a live backend; trace detail expands; log filter narrows results within 2 seconds.

---

## Phase 7: User Story 5 — Skill Discovery (Priority: P5)

**Goal**: Users can browse skills available to the active persona with names and enabled/disabled state.

**Independent Test**: Navigate to skills screen for the active persona → skills list with name, description, enabled/disabled badge visible → empty state shows when no skills configured.

- [x] T038 Implement `crates/web-ui/src/api/skills.rs` — `GET /api/personas/{id}/skills`; add to OpenAPI doc; tests using `StorageLayer::new_in_memory()`
- [x] T039 Register skills route in `crates/web-ui/src/main.rs` under the auth-protected scope
- [x] T040 [P] [US5] Implement `app/lib/api/endpoints/skills.dart` — `SkillsEndpoint`: `listForPersona(personaId)`
- [x] T041 [P] [US5] Implement `app/lib/features/skills/skills_provider.dart` — `SkillsNotifier`: fetch skills for active persona on mount; refresh on persona switch
- [x] T042 [US5] Implement `app/lib/features/skills/skills_screen.dart` — list of `Skill` rows (name, description, enabled/disabled chip); empty state widget when list is empty
- [x] T043 [US5] Wire skills into navigation: add `/skills` route; add navigation entry alongside traces + logs

**Checkpoint**: US5 independently testable — skills screen loads and displays read-only skill list for the active persona; empty state appears when persona has no skills.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Build verification, backend test coverage, and end-to-end validation.

- [x] T044 [P] Add unit tests for `app/lib/api/client.dart` SSE parser in `app/test/unit/api/client_test.dart` — verify `TokenEvent`, `DoneEvent`, `ErrorEvent` parsing from raw byte streams
- [x] T045 [P] Add widget tests for `app/lib/features/connection/connection_screen.dart` in `app/test/widget/connection_screen_test.dart` — verify error messages for invalid token and unreachable server
- [ ] T046 [P] Verify `flutter build web` produces a deployable static site in `app/build/web/`
- [ ] T047 [P] Verify `flutter build macos` produces a `.app` bundle in `app/build/macos/Build/Products/Release/`
- [ ] T048 Run `quickstart.md` validation end-to-end against a local backend: connection → chat → persona switch → traces → logs → skills
- [x] T049 Update `AGENTS.md` with Flutter development setup: Flutter SDK requirement, `flutter pub get`, `flutter run -d chrome|macos`, `flutter test`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 completion — BLOCKS all user stories
- **US2 Connection (Phase 3)**: Depends on Phase 2 — BLOCKS US1 end-to-end testing
- **US1 Chat (Phase 4)**: Depends on Phase 2 (foundational) + Phase 3 (connection) for E2E test; can start implementation after Phase 2
- **US3 Personas (Phase 5)**: Depends on Phase 2; requires T022–T023 (backend) before T024–T027 (Flutter)
- **US4 Observability (Phase 6)**: Depends on Phase 2; requires T028–T030 (backend) before T031–T037 (Flutter)
- **US5 Skills (Phase 7)**: Depends on Phase 2; requires T038–T039 (backend) before T040–T043 (Flutter)
- **Polish (Phase 8)**: Depends on all user stories complete

### User Story Dependencies

- **US2 (P2)**: Can start after Foundational — no dependency on other stories; but gates US1 E2E testing
- **US1 (P1)**: Can start implementation after Foundational; needs US2 complete for E2E test
- **US3 (P3)**: Can start after Foundational — no dependency on US1/US2 for implementation
- **US4 (P4)**: Can start after Foundational — independent of US1/US2/US3
- **US5 (P5)**: Can start after Foundational — independent; shares persona context with US3

### Within Each User Story

- Backend handlers → register routes → Flutter endpoint client → Flutter provider → Flutter screen → navigation wire-up
- Each story's backend work (Rust) can start in parallel with another story's Flutter work

### Parallel Opportunities

- All Phase 1 tasks marked [P] can run concurrently after T001 and T002 complete
- All Phase 2 model tasks (T007–T013) can run in parallel after T006 (client) is scaffolded
- US3/US4/US5 Flutter work can all start in parallel after Phase 2 completes (Rust backend and Flutter side independent per story)

---

## Parallel Example: Phase 2 Models

```bash
# Once T006 (client.dart) is complete, all models can be written simultaneously:
Task: "Implement app/lib/api/models/server_profile.dart"       # T007
Task: "Implement app/lib/api/models/conversation.dart"         # T008
Task: "Implement app/lib/api/models/stream_event.dart"         # T009
Task: "Implement app/lib/api/models/persona.dart"              # T010
Task: "Implement app/lib/api/models/skill.dart"                # T011
Task: "Implement app/lib/api/models/trace.dart"                # T012
Task: "Implement app/lib/api/models/log_entry.dart"            # T013
```

## Parallel Example: US3/US4/US5 Backend + Flutter Split

```bash
# After Phase 2 completes, backend and Flutter work for each story runs in parallel:
Task: "Implement crates/web-ui/src/api/personas.rs"            # T022 (backend)
Task: "Implement crates/web-ui/src/api/traces.rs"              # T028 (backend)
Task: "Implement crates/web-ui/src/api/logs.rs"                # T029 (backend)
Task: "Implement crates/web-ui/src/api/skills.rs"              # T038 (backend)

# Flutter providers can be written with mock data while backend is in progress:
Task: "Implement app/lib/features/personas/personas_provider.dart"  # T025
Task: "Implement app/lib/features/traces/traces_provider.dart"      # T033
Task: "Implement app/lib/features/logs/logs_provider.dart"          # T034
Task: "Implement app/lib/features/skills/skills_provider.dart"      # T041
```

---

## Implementation Strategy

### MVP First (US2 → US1 only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (API client + models)
3. Complete Phase 3: US2 — Connection & Profile Setup
4. Complete Phase 4: US1 — Streaming Chat
5. **STOP and VALIDATE**: Can connect, chat with streaming, view conversation history
6. Build `flutter build web` → deploy static site; `flutter build macos` → distribute `.app`

This MVP replaces the browser-based web UI for the primary use case.

### Incremental Delivery

1. Setup + Foundational → API client ready
2. US2 Connection → Can configure and validate server
3. US1 Chat → Streaming chat works on web + macOS (MVP!)
4. US3 Personas → Can switch personas
5. US4 Observability → Traces + logs visible
6. US5 Skills → Skill discovery complete (full feature parity with web UI)

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together (T001–T014)
2. Once Foundational is done:
   - Developer A: US2 + US1 (connection + chat — critical path)
   - Developer B: Backend endpoints T022 + T028 + T029 + T038 (all new Rust handlers)
   - Developer C: US3 Flutter side (personas provider + screen) once T022–T023 done

---

## Notes

- [P] tasks = different files, no blocking dependencies between them
- Backend tasks (Rust) MUST be done before the corresponding Flutter endpoint client can make real API calls; use mock data in Flutter during backend development
- Backend unit tests for new Rust handlers are required by Constitution Principle III — included in T022, T028, T029, T038 task descriptions
- Secrets (server token) MUST be stored via `flutter_secure_storage` only — never in plain SharedPreferences or logged (Constitution Principle X)
- The Flutter web build is a static site — it is NOT served by the Rust backend. Deploy separately (nginx, GitHub Pages, etc.)
- `flutter build macos` requires Xcode on macOS; CI must run on a macOS runner for the macOS build step
