# Tasks: Matrix Interface

**Input**: Design documents from `/specs/002-matrix-interface/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Paths follow the workspace crate layout defined in `plan.md`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Scaffold the new crate and add workspace-level dependencies before any user story work begins.

- [ ] T001 Create `crates/interface-matrix/Cargo.toml` with `[package]`, `[lib]`, `[dependencies]`, and `[dev-dependencies]` sections matching workspace conventions (name = `assistant-interface-matrix`)
- [ ] T002 [P] Add `matrix-sdk` (with `tokio-handle` and `sqlite` features) to `[workspace.dependencies]` in root `Cargo.toml`
- [ ] T003 [P] Write ADR at `docs/adr/adr-0002-matrix-interface.md` documenting the choice of `matrix-sdk`, access-token auth, and room-ID conversation keying

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extend `assistant-core` with the `MatrixConfig` type and `Interface::Matrix` variant. Everything in Phase 3+ depends on these compiling.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T004 Add `MatrixConfig` struct to `crates/core/src/types.rs` with fields: `homeserver_url`, `username`, `password`, `access_token`, `device_id`, `state_store_path`, `allowed_rooms`, `allowed_users` (all matching the schema in `specs/002-matrix-interface/data-model.md`)
- [ ] T005 Add `Matrix` variant to the `Interface` enum in `crates/core/src/types.rs`
- [ ] T006 Add `pub matrix: Option<MatrixConfig>` field to `AssistantConfig` in `crates/core/src/types.rs` (with `/// Populated from the [matrix] section of config.toml` doc comment)
- [ ] T007 Export `MatrixConfig` from `crates/core/src/lib.rs` (add to the existing re-export list)
- [ ] T008 Create placeholder stub `crates/interface-matrix/src/lib.rs` declaring `pub mod config; pub mod runner; pub mod tools;` and the three empty source files so `cargo check --workspace` passes

**Checkpoint**: `cargo check --workspace` must compile cleanly before proceeding.

---

## Phase 3: User Story 1 — Chat with Assistant in Matrix Room (Priority: P1) 🎯 MVP

**Goal**: A user in any Matrix room where the bot is a member can send a message and receive a response from the assistant.

**Independent Test**: Start the bot with `make run-matrix`, invite it to a test room, send a plain text message, verify a reply appears in the same room.

### Implementation for User Story 1

- [ ] T009 [P] [US1] Implement `MatrixConfigExt` trait in `crates/interface-matrix/src/config.rs` with `resolved_homeserver_url()`, `resolved_username()`, `resolved_access_token()`, `resolved_password()`, `resolved_state_store_path()` (env var fallbacks: `MATRIX_HOMESERVER_URL`, `MATRIX_USERNAME`, `MATRIX_ACCESS_TOKEN`, `MATRIX_PASSWORD`)
- [ ] T010 [P] [US1] Add unit tests for `MatrixConfigExt` in `crates/interface-matrix/src/config.rs`: explicit values used verbatim, env var fallback, TOML round-trip, `AssistantConfig` with `[matrix]` section, without section is `None`
- [ ] T011 [US1] Implement `MatrixInterface` struct with `pub fn new(config: MatrixConfig, orchestrator: Arc<Orchestrator>) -> Self` in `crates/interface-matrix/src/runner.rs`
- [ ] T012 [US1] Implement `MatrixInterface::run()` in `crates/interface-matrix/src/runner.rs`: build `matrix_sdk::Client` from `homeserver_url`, login via `access_token` or `password` (error if neither present), fetch bot `user_id` via `client.user_id()`, initialise `LruCache<String, Uuid>` (cap 10 000), register room message event handler, call `client.sync(SyncSettings::default())` in a loop with exponential backoff (1 s → 60 s cap), graceful shutdown on SIGINT/SIGTERM
- [ ] T013 [US1] Implement the `OriginalSyncRoomMessageEvent` handler closure in `crates/interface-matrix/src/runner.rs`: extract `room_id`, `sender`, `body` text; skip if `sender == bot_user_id`; skip if `allowed_rooms` non-empty and room not listed; skip if `allowed_users` non-empty and sender not listed; resolve or create `conversation_id` from LRU map; call `orchestrator.submit_turn(&text, conversation_id, Interface::Matrix, None)` and send reply via `room.send()`; on orchestrator error send a user-visible error message
- [ ] T014 [P] [US1] Add unit tests for allowlist logic in `crates/interface-matrix/src/runner.rs`: empty `allowed_rooms` accepts all; non-empty blocks unknown room; empty `allowed_users` accepts all; non-empty passes known user
- [ ] T015 [P] [US1] Implement `build_matrix_tools()` stub returning `Vec<Arc<dyn ToolHandler>>` in `crates/interface-matrix/src/tools.rs` (empty for v1; mirrors `build_mattermost_tools` signature)
- [ ] T016 [US1] Wire public exports in `crates/interface-matrix/src/lib.rs`: `pub use assistant_core::MatrixConfig; pub use runner::MatrixInterface;`
- [ ] T017 [US1] Add `matrix = ["dep:assistant-interface-matrix"]` feature and `assistant-interface-matrix = { path = "../interface-matrix", optional = true }` dependency to `crates/interface-cli/Cargo.toml`; add `matrix` to the `default` features list
- [ ] T018 [US1] Add `Matrix` variant to the `Command` enum in `crates/interface-cli/src/main.rs` (with `#[cfg(feature = "matrix")]` guard, doc string, and `about` text matching Mattermost style)
- [ ] T019 [US1] Add matrix-only mode handler in `crates/interface-cli/src/main.rs`: `#[cfg(feature = "matrix")] if let Some(Command::Matrix) = &cli.command { ... }` — load config, construct `MatrixInterface`, spawn `matrix-worker` filtered worker, spawn scheduler worker, call `iface.run().await` (mirrors the Mattermost pattern at line 1180)
- [ ] T020 [US1] Add `run-matrix` target to `Makefile`: `cargo run -p assistant-cli --features matrix -- orchestrator run --interfaces matrix --no-repl` (mirrors `run-mattermost`)
- [ ] T021 [US1] Run `cargo test -p assistant-interface-matrix` and fix any failing tests

**Checkpoint**: `make run-matrix` starts, connects to a homeserver, and replies to a message in a room.

---

## Phase 4: User Story 2 — Private Direct Message Conversations (Priority: P2)

**Goal**: A user can open a 1:1 DM with the bot in Matrix; the bot accepts the invite and responds in the private channel without cross-contaminating other room contexts.

**Independent Test**: Open a direct message with the bot from a Matrix client; send a message; verify the bot replies only in the DM thread and not in any group room.

### Implementation for User Story 2

- [ ] T022 [US2] Implement `StrippedRoomMemberEvent` handler in `crates/interface-matrix/src/runner.rs` to auto-accept room invitations: when the bot is invited (`membership == Invited && state_key == bot_user_id`) call `room.join()`, log success or failure; this enables both DM invites and group room invites at runtime
- [ ] T023 [P] [US2] Add unit test confirming conversation LRU map keys are isolated by `room_id`: insert two distinct room IDs, verify each maps to a different `Uuid`, verify a repeated room ID returns the same `Uuid`
- [ ] T024 [US2] Run `cargo test -p assistant-interface-matrix` after T022–T023 and fix any failing tests

**Checkpoint**: Bot auto-accepts DM invites and maintains a conversation context per DM room isolated from group rooms.

---

## Phase 5: User Story 3 — Multi-Room Deployment (Priority: P3)

**Goal**: A single bot instance handles multiple rooms simultaneously; context never leaks across rooms; new rooms can be joined at runtime via invite without restarting the process.

**Independent Test**: Invite the bot to two rooms simultaneously; send distinct messages in each; verify each room receives only its own reply and shares no context with the other.

### Implementation for User Story 3

- [ ] T025 [US3] Add background Matrix startup to the multi-interface orchestrator mode in `crates/interface-cli/src/main.rs`: `#[cfg(feature = "matrix")] if bs.config.matrix.is_some() && interface_selected(&orchestrator_interfaces, "matrix") { ... }` — spawn `matrix-worker` filtered worker and background `iface.run()` task (mirrors the Mattermost block at line 1278)
- [ ] T026 [P] [US3] Add `matrix` to the `--interfaces` selection text/documentation in `crates/interface-cli/src/main.rs` (help text for `--interfaces` flag already lists `slack`, `mattermost`, etc.)
- [ ] T027 [US3] Run `cargo test --workspace` after T025–T026 and fix any failing tests

**Checkpoint**: `assistant orchestrator run --interfaces matrix,slack` starts both interfaces; each handles messages independently with no context leakage.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Code quality, documentation, and final verification across all user stories.

- [ ] T028 [P] Update `AGENTS.md` workspace table to include `assistant-interface-matrix | crates/interface-matrix | Matrix bot interface`
- [ ] T029 [P] Add `run-matrix` to the `run` targets list at the top of `Makefile` `.PHONY` declaration
- [ ] T030 Run `make lint` (`cargo clippy --workspace -- -D warnings`) and fix all warnings
- [ ] T031 Run `make format` (`cargo fmt --all`) and fix any formatting issues
- [ ] T032 Run `cargo machete --with-metadata` and remove any unused dependencies
- [ ] T033 Validate `specs/002-matrix-interface/quickstart.md` steps against the finished implementation; update any paths or commands that changed

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — T002 and T003 can start immediately in parallel
- **Foundational (Phase 2)**: Requires T001 (Cargo.toml exists) — BLOCKS all user stories
- **US1 (Phase 3)**: Requires Phase 2 completion
- **US2 (Phase 4)**: Requires Phase 3 completion (invitation handler extends the runner)
- **US3 (Phase 5)**: Requires Phase 3 completion (background startup extends the CLI)
- **Polish (Phase 6)**: Requires all desired story phases

### User Story Dependencies

- **US1 (P1)**: Starts after Foundational — no dependency on other stories
- **US2 (P2)**: Extends the runner built in US1 — depends on US1 completion
- **US3 (P3)**: Extends the CLI multi-interface wiring from US1 — depends on US1 completion; US2 and US3 can be done in parallel after US1

### Within Each Phase

- Models/config before services/runners
- Config implementation before runner implementation
- Runner complete before CLI wiring
- CLI wiring before Makefile target

### Parallel Opportunities

- T002 and T003 (Phase 1) are independent and can run in parallel
- T009 and T010 (config + config tests) are independent of T011+ (runner) and can start in parallel
- T014 (allowlist unit tests) and T015 (tools stub) are independent of each other and of T012–T013
- T017 and T018 (CLI Cargo.toml + Command enum) can run in parallel
- After US1 is complete: T022–T024 (US2) and T025–T027 (US3) can run in parallel

---

## Parallel Example: User Story 1

```bash
# These can start in parallel once Phase 2 is complete:
Task T009: "Implement MatrixConfigExt trait in crates/interface-matrix/src/config.rs"
Task T010: "Add unit tests for MatrixConfigExt in crates/interface-matrix/src/config.rs"

# After T009 is done, these can start in parallel:
Task T011: "Implement MatrixInterface struct in crates/interface-matrix/src/runner.rs"
Task T014: "Add allowlist unit tests in crates/interface-matrix/src/runner.rs"
Task T015: "Implement build_matrix_tools() stub in crates/interface-matrix/src/tools.rs"

# After T011-T013 are done, these can start in parallel:
Task T017: "Add matrix feature flag to crates/interface-cli/Cargo.toml"
Task T018: "Add Matrix variant to Command enum in crates/interface-cli/src/main.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001–T003)
2. Complete Phase 2: Foundational (T004–T008) — **must compile**
3. Complete Phase 3: US1 (T009–T021)
4. **STOP and VALIDATE**: Run `make run-matrix`, connect to a homeserver, send a message, verify reply
5. Demo / ship if ready

### Incremental Delivery

1. Setup + Foundational → `cargo check --workspace` passes
2. US1 complete → Basic bot works in any joined room (`make run-matrix`)
3. US2 complete → Bot auto-accepts DM invites and isolates DM contexts
4. US3 complete → Bot runs alongside other interfaces in multi-interface mode
5. Polish → CI green, docs updated

### Parallel Team Strategy

With two developers after Phase 2:

- Developer A: US1 (T009–T021) — the core bot loop
- Developer B: ADR + docs (T003, T028, T029) — can proceed independently

After US1:

- Developer A: US2 (T022–T024)
- Developer B: US3 (T025–T027)

---

## Notes

- `[P]` tasks touch different files and have no blocking dependency on in-progress tasks
- Each user story is independently runnable after Phase 2 (US1) or US1 (US2/US3)
- Commit after each checkpoint (T008, T021, T024, T027, T033)
- The invitation handler (T022) is the only net-new behaviour beyond copy-paste from the Mattermost pattern; all other tasks are structural parallels
- E2E encryption, voice transcription, and thread-scoped contexts are explicitly out of scope per `specs/002-matrix-interface/spec.md` Assumptions
