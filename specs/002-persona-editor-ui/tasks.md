# Tasks: Persona Editor UI

**Input**: Design documents from `/specs/002-persona-editor-ui/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/http-routes.md ✓

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Prepare the shared constants and imports that all handlers depend on. No new crates or migrations needed — working in existing `assistant-web-ui` and `assistant-storage`.

- [ ] T001 Add `PERSONA_FILE_SLOTS` constant (8-entry array of `(filename, display_name, description)` tuples) and ensure `use tokio::fs` and `use std::path::PathBuf` are present in `crates/web-ui/src/contexts.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared helpers and storage method that ALL user stories require. Must complete before any story work begins.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T002 Add `PersonaStore::create(id: &str, name: &str) -> Result<PersonaRecord>` method to `crates/storage/src/personas.rs` — inserts with `is_default = 0`, returns `Err` on duplicate `id` (SQLite UNIQUE violation)
- [ ] T003 [P] Add unit test `create_returns_error_on_duplicate_id` in `#[cfg(test)] mod tests` at the bottom of `crates/storage/src/personas.rs`
- [ ] T004 [P] Add `fn persona_filename(s: &str) -> Option<&'static str>` whitelist helper in `crates/web-ui/src/contexts.rs` — returns `Some(canonical_name)` for the 8 allowed filenames, `None` for all others
- [ ] T005 [P] Add `fn persona_agent_dir(id: &str) -> Option<PathBuf>` helper in `crates/web-ui/src/contexts.rs` — resolves `dirs::home_dir()/.assistant/agents/{id}/` without creating the directory

**Checkpoint**: Foundation ready — all user story handlers can now call `PersonaStore::create()`, `persona_filename()`, and `persona_agent_dir()`.

---

## Phase 3: User Story 1 — View and Edit Persona Markdown Files (Priority: P1) 🎯 MVP

**Goal**: Users can navigate to `/personas`, click a persona, see all 8 file slots, open an existing file in an editor, and save changes.

**Independent Test**: Navigate to `/personas`, click a persona row, confirm all 8 file slots are listed with correct present/absent indicators. Click "Edit" on an existing file, change content, save — confirm content persists. Reload the editor page and confirm updated content is shown.

- [ ] T006 [US1] Add `PersonaFileSlotView` struct (fields: `filename: &'static str`, `display_name: &'static str`, `description: &'static str`, `exists: bool`) and `PersonaDetailTemplate` Askama struct in `crates/web-ui/src/contexts.rs`
- [ ] T007 [US1] Add `show_persona_detail` handler (`GET /personas/{id}`) in `crates/web-ui/src/contexts.rs` — validates `id` via `validate_agent_id()`, fetches persona from `PersonaStore`, checks each of the 8 file slots for existence via `tokio::fs::try_exists`, returns 404 if persona not found
- [ ] T008 [P] [US1] Create `crates/web-ui/templates/personas/detail.html` — extends `base.html`, shows persona ID and name in header, renders a table of all 8 file slots with present/absent badge and "Edit" link (for present files) or greyed-out "—" (for absent); back link to `/personas`
- [ ] T009 [US1] Add `PersonaFileEditorTemplate` Askama struct (fields: `persona_id`, `filename`, `display_name`, `content`, `is_new`, `show_saved`, `error_msg`) and `show_file_editor` handler (`GET /personas/{id}/files/{filename}`) in `crates/web-ui/src/contexts.rs` — validates `id` and `filename` (whitelist), reads file content via `tokio::fs::read_to_string` (empty string if `NotFound`), sets `is_new` accordingly
- [ ] T010 [P] [US1] Create `crates/web-ui/templates/personas/file_editor.html` — extends `base.html`, shows file display name in header, `<textarea name="content">` with full file content, Save/Cancel buttons, success banner when `show_saved == true`, error banner when `error_msg` is non-empty, and an inline Stimulus controller that sets `data-dirty` on textarea change and fires `window.onbeforeunload` warning when dirty and user navigates away
- [ ] T011 [US1] Add `save_file` handler (`POST /personas/{id}/files/{filename}`) in `crates/web-ui/src/contexts.rs` — validates `id` and `filename` (whitelist), rejects content >2 MB with HTTP 413, creates directory via `tokio::fs::create_dir_all(persona_agent_dir(id))`, writes content via `tokio::fs::write`, redirects to `GET /personas/{id}/files/{filename}?saved=1` on success or `?error={msg}` on filesystem error
- [ ] T012 [US1] Register `GET /personas/{id}`, `GET /personas/{id}/files/{filename}`, and `POST /personas/{id}/files/{filename}` routes in `contexts_router()` in `crates/web-ui/src/contexts.rs` (ensure `/personas/new` literal route will be added in Phase 5 BEFORE the `{id}` wildcard)
- [ ] T013 [US1] Update `crates/web-ui/templates/personas/page.html` — wrap each persona's ID and name cells in `<a href="/personas/{{ row.id }}">` links to enable navigation to the detail view

**Checkpoint**: US1 fully functional. Users can list personas, open detail view, and edit/save existing files. Test independently per Independent Test above.

---

## Phase 4: User Story 2 — Create and Edit New Persona Markdown Files (Priority: P2)

**Goal**: When a persona has no SOUL.md (or any other file slot), users can click "Create" for that slot, type content, and save — the file is created on disk.

**Independent Test**: Open a persona detail view. Confirm absent file slots show a "Create" link. Click "Create" next to SOUL.md. Confirm an empty editor opens with heading "Create new file". Type content and click "Create File". Confirm success banner appears. Navigate back to detail view and confirm SOUL.md now shows as present with an "Edit" link.

- [ ] T014 [US2] Update `crates/web-ui/templates/personas/detail.html` — replace the "—" placeholder for absent file slots with a `<a href="/personas/{{ row.persona_id }}/files/{{ slot.filename }}" class="action-btn">Create</a>` link, making all 8 slots actionable
- [ ] T015 [P] [US2] Update `crates/web-ui/templates/personas/file_editor.html` — conditionally render "Create new file: {{ display_name }}" vs "Edit: {{ display_name }}" heading, and "Create File" vs "Save Changes" as the submit button label, based on the `is_new` boolean
- [ ] T016 [US2] Verify in `show_file_editor` handler in `crates/web-ui/src/contexts.rs` that `is_new` is set to `true` when the file is absent (caught by `std::io::ErrorKind::NotFound`) and `false` when the file is present — add comment explaining the `is_new` semantics

**Checkpoint**: US2 fully functional. "Create" links appear for absent files; editor is correctly labelled; saved content creates the file on disk.

---

## Phase 5: User Story 3 — Create a New Persona (Priority: P2)

**Goal**: Users can click "New Persona" on the personas list page, enter an ID and name, and the persona appears in the list immediately.

**Independent Test**: Click "New Persona". Submit form with ID `test-persona` and name `Test Persona`. Confirm redirect to `/personas/test-persona` detail page with all 8 slots absent. Navigate to `/personas` list and confirm `test-persona` appears. Attempt to create another persona with ID `test-persona` and confirm an error message is shown without creating a duplicate.

- [ ] T017 [US3] Add `PersonaNewFormTemplate` Askama struct (fields: `error_msg: Option<String>`) and `show_new_persona_form` handler (`GET /personas/new`) in `crates/web-ui/src/contexts.rs`
- [ ] T018 [P] [US3] Create `crates/web-ui/templates/personas/new.html` — extends `base.html`, form with `id` text input (placeholder: `work`, pattern hint shown), `name` text input, submit button "Create Persona", back link to `/personas`, error banner when `error_msg` is `Some`
- [ ] T019 [US3] Add `create_persona` handler (`POST /personas`) in `crates/web-ui/src/contexts.rs` — parses `id` and `name` from form body, validates `id` via `validate_agent_id()` and `name` non-empty, calls `PersonaStore::create()`, redirects to `/personas/{id}` on success or to `/personas/new?error={msg}` on validation/duplicate error
- [ ] T020 [US3] Register `GET /personas/new` and `POST /personas` in `contexts_router()` in `crates/web-ui/src/contexts.rs` — IMPORTANT: `GET /personas/new` MUST appear before `GET /personas/{id}` in the router definition to prevent "new" being matched as an `{id}` parameter
- [ ] T021 [US3] Update `crates/web-ui/templates/personas/page.html` — add "New Persona" button in the panel header (next to the count pill) linking to `GET /personas/new`

**Checkpoint**: US3 fully functional. New personas can be created from the UI and immediately appear in the list and with a navigable detail page.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Quality gate and cleanup before merge.

- [ ] T022 Run `make lint && make format` from repo root, fix all `cargo clippy -D warnings` issues in `crates/web-ui/src/contexts.rs` and `crates/storage/src/personas.rs`
- [ ] T023 [P] Manual smoke test against running server per `specs/002-persona-editor-ui/quickstart.md` — verify all 6 new routes respond correctly and all 3 user story Independent Tests pass
- [ ] T024 [P] Update `specs/002-persona-editor-ui/checklists/requirements.md` to mark all items complete post-implementation

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories; T003, T004, T005 are parallelizable after T002
- **US1 (Phase 3)**: Depends on Phase 2 — T006→T007→(T008∥T009)→(T010∥T011)→T012→T013
- **US2 (Phase 4)**: Depends on Phase 3 (reuses US1 handlers); T014, T015, T016 are mostly independent of each other
- **US3 (Phase 5)**: Depends on Phase 2 (uses `PersonaStore::create()`); T017→T018∥T019→T020→T021
- **Polish (Phase 6)**: Depends on all user stories complete

### User Story Dependencies

- **US1 (P1)**: Can start after Phase 2 — core MVP, no dependency on other user stories
- **US2 (P2)**: Depends on US1 (reuses same route handlers, adds template differentiation only)
- **US3 (P2)**: Can start after Phase 2 — independent of US1/US2

### Within Each User Story

- Models/structs before handlers
- Handlers before templates (need to know fields to render)
- Templates before route registration (need template to exist for compilation)
- Route registration last (enables end-to-end testing)

### Parallel Opportunities

- T003, T004, T005 can all run in parallel after T002
- T008 (template) can be drafted in parallel with T007 (handler) since both are new files
- T010 (file editor template) can be drafted in parallel with T009 (file editor handler)
- T018 (new persona template) can be drafted in parallel with T019 (create persona handler)
- T014, T015, T016 in Phase 4 are all independent of each other
- T023 and T024 in Polish can run in parallel

---

## Parallel Example: US1

```
# These can be worked on simultaneously (different files):
Task T007: "Add show_persona_detail handler in crates/web-ui/src/contexts.rs"
Task T008: "Create crates/web-ui/templates/personas/detail.html"

# These can be worked on simultaneously:
Task T009: "Add show_file_editor handler in crates/web-ui/src/contexts.rs"
Task T010: "Create crates/web-ui/templates/personas/file_editor.html"
```

## Parallel Example: US3

```
# These can be worked on simultaneously (different files):
Task T018: "Create crates/web-ui/templates/personas/new.html"
Task T019: "Add create_persona handler in crates/web-ui/src/contexts.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001)
2. Complete Phase 2: Foundational (T002–T005)
3. Complete Phase 3: US1 (T006–T013)
4. **STOP and VALIDATE**: Run manual smoke test for US1 — list personas, view detail, edit and save a file
5. Demo / deploy as MVP

### Incremental Delivery

1. Phase 1 + 2 → Foundation ready
2. Phase 3 (US1) → File editing works → **MVP**
3. Phase 4 (US2) → Create new files from UI
4. Phase 5 (US3) → Create new personas from UI
5. Phase 6 → Polish + merge

### Key Implementation Note: Route Order

When registering routes in `contexts_router()`, the literal route `/personas/new` MUST come before the parameterized route `/personas/{id}`. In Axum, routes are matched in registration order for same-prefix patterns; registering `{id}` first would match "new" as a persona ID.

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [Story] label maps task to specific user story
- No test tasks included for web handlers (not explicitly requested in spec); unit test for `PersonaStore::create()` included per constitution Test Discipline principle
- Commit after each phase checkpoint using semantic commits: `feat(web-ui): ...`, `feat(storage): ...`
- Run `make lint && make format` before each commit
- Total tasks: **24** across 6 phases
