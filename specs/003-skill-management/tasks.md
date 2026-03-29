# Tasks: Skill Management via Web UI and CLI

**Input**: Design documents from `specs/003-skill-management/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅, quickstart.md ✅

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.
**Tests**: Not included (not requested in spec). Add `#[tokio::test]` unit tests inline as you implement.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1–US4)

---

## Phase 1: Setup

**Purpose**: New files and directories that must exist before implementation work begins.

- [ ] T001 Create `migrations/028_skill_body.sql` — `ALTER TABLE skills ADD COLUMN body_text TEXT NOT NULL DEFAULT ''`
- [ ] T002 Create `migrations/029_persona_skill_access.sql` — add `skill_access_mode` to `personas` + create `persona_skill_list` table (see data-model.md)
- [ ] T003 Create `crates/storage/src/persona_skill_access.rs` as empty module with `pub struct PersonaSkillAccessStore { pool: SqlitePool }`
- [ ] T004 [P] Create `crates/web-ui/src/skills/mod.rs` as empty module stub
- [ ] T005 [P] Create `crates/web-ui/src/skills/pages.rs` as empty module stub
- [ ] T006 [P] Create `crates/web-ui/templates/skills/` directory with empty placeholder files: `list.html`, `show.html`, `new.html`, `edit.html`
- [ ] T007 [P] Create `crates/web-ui/templates/personas/skill_access.html` as empty template stub
- [ ] T008 [P] Create `skills/agentskills-spec/SKILL.md` — minimal valid frontmatter stub (name, description) to keep compile passing

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Storage schema and core CRUD methods that ALL user stories depend on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T009 Register migrations 028 and 029 in the `migrations` slice in `crates/storage/src/lib.rs` (append after `027_slack_active_threads`)
- [ ] T010 Update `SkillDef` in `crates/skills/src/skill.rs` — no struct change needed; confirm `body` field maps to `body_text` DB column (review only, document mapping)
- [ ] T011 Update `upsert_to_db()` in `crates/storage/src/registry.rs` to bind `skill.body` to the new `body_text` column in the INSERT/UPDATE statement
- [ ] T012 Add `PersonaRecord.skill_access_mode: String` field to `crates/storage/src/personas.rs`; update all `SELECT` queries that return `PersonaRecord` to include `skill_access_mode`
- [ ] T013 Implement `PersonaSkillAccessStore` in `crates/storage/src/persona_skill_access.rs`:
  - `get_mode(persona_id) -> Result<String>` — returns `skill_access_mode` from `personas`
  - `set_mode(persona_id, mode) -> Result<()>` — UPDATE `personas.skill_access_mode`; validate mode is `all`/`whitelist`/`blacklist`
  - `list_skill_names(persona_id) -> Result<Vec<String>>` — SELECT from `persona_skill_list`
  - `add_skill(persona_id, skill_name) -> Result<()>` — INSERT OR IGNORE into `persona_skill_list`
  - `remove_skill(persona_id, skill_name) -> Result<()>` — DELETE from `persona_skill_list`
- [ ] T014 Expose `StorageLayer::persona_skill_access_store()` convenience constructor in `crates/storage/src/lib.rs`; add `PersonaSkillAccessStore` to pub re-exports
- [ ] T015 [P] Add `create_user_skill(name, description, body) -> Result<SkillDef>` to `SkillRegistry` in `crates/storage/src/registry.rs`:
  - Validate name kebab-case and uniqueness
  - Resolve `~/.assistant/skills/<name>/` via `dirs::home_dir()`
  - `tokio::fs::create_dir_all` + `tokio::fs::write` SKILL.md with frontmatter + body
  - Call `self.register(parsed_def)` to upsert to DB
  - Return error if builtin name conflict or disk failure
- [ ] T016 [P] Add `update_user_skill(name, description, body) -> Result<()>` to `SkillRegistry` in `crates/storage/src/registry.rs`:
  - Reject if `source_type = 'builtin'` or `'project'`
  - Write updated SKILL.md to existing `def.dir`
  - Call `self.register(updated_def)` to upsert to DB
- [ ] T017 [P] Add `delete_user_skill(name) -> Result<()>` to `SkillRegistry` in `crates/storage/src/registry.rs`:
  - Reject if `source_type = 'builtin'`
  - Call `self.remove(name)` (DB delete + in-memory remove)
  - `tokio::fs::remove_dir_all(def.dir)` — log warning but do not fail if dir already gone
- [ ] T018 Add `list_for_persona(persona_id: &str, pool: &SqlitePool) -> Result<Vec<SkillDef>>` to `SkillRegistry` in `crates/storage/src/registry.rs`:
  - Query `personas.skill_access_mode` and `persona_skill_list` for the given persona
  - Apply filtering algorithm from data-model.md: all → return all; whitelist → filter in; blacklist → filter out
  - Returns filtered list; falls back to full list if persona not found (warn)

**Checkpoint**: `cargo check -p assistant-storage` passes. Migrations 028/029 exist. SkillRegistry has create/update/delete/list_for_persona. PersonaSkillAccessStore is complete.

---

## Phase 3: User Story 1 — Browse and Manage Global Skills via Web UI (Priority: P1) 🎯 MVP

**Goal**: Web UI Skills section with full CRUD for global skills (list, view, create, edit, delete). Builtin skills are read-only.

**Independent Test**: Navigate to `/skills` in the running web UI; create a new skill, edit it, view it, delete it — all without touching CLI or persona settings.

- [ ] T019 [US1] Add `registry: Arc<SkillRegistry>` to `AppState` struct in `crates/web-ui/src/main.rs`; wire it from the existing `registry` variable in `run_with_args()`
- [ ] T020 [P] [US1] Define `SkillsPagesState` struct and `skills_router()` function in `crates/web-ui/src/skills/mod.rs`; wire `skills_router()` into `protected_routes` in `crates/web-ui/src/main.rs`
- [ ] T021 [P] [US1] Implement `list` handler in `crates/web-ui/src/skills/pages.rs` — calls `registry.list()`, renders `templates/skills/list.html`; `list.html` shows name, source badge, description, Edit/Delete links per row, "New Skill" button
- [ ] T022 [US1] Implement `show` handler in `crates/web-ui/src/skills/pages.rs` — looks up skill by name, renders `templates/skills/show.html`; `show.html` displays all fields + body; shows Edit/Delete only if not builtin
- [ ] T023 [US1] Implement `new_form` and `create` handlers in `crates/web-ui/src/skills/pages.rs`:
  - `new_form` renders `templates/skills/new.html` (name, description, body textarea)
  - `create` validates form fields, calls `registry.create_user_skill()`, redirects to `/skills/:name` or re-renders with error
- [ ] T024 [US1] Implement `edit_form` and `update` handlers in `crates/web-ui/src/skills/pages.rs`:
  - `edit_form` renders `templates/skills/edit.html` pre-populated from DB (description + body_text)
  - `update` validates, calls `registry.update_user_skill()`, redirects or re-renders with error
  - Returns 403 if skill is builtin
- [ ] T025 [US1] Implement `delete` handler in `crates/web-ui/src/skills/pages.rs`:
  - Calls `registry.delete_user_skill()`
  - Returns 400 with message if builtin; redirects to `/skills` on success
  - HTMX-friendly: returns 200 with empty body (row removal) or redirect
- [ ] T026 [P] [US1] Complete `templates/skills/list.html` — extend base layout, table with source badge, HTMX delete button with confirm dialog, "New Skill" button
- [ ] T027 [P] [US1] Complete `templates/skills/show.html` — skill detail view, metadata block, body rendered in `<pre>`, conditional Edit/Delete buttons
- [ ] T028 [P] [US1] Complete `templates/skills/new.html` — form with name, description, body textarea, submit button, client-side kebab-case hint on name field
- [ ] T029 [P] [US1] Complete `templates/skills/edit.html` — same as new form but name is read-only, pre-populated from DB values
- [ ] T030 [US1] Add Skills link to navigation in `templates/base.html` (or equivalent nav partial)

**Checkpoint**: `assistant webui serve` → navigate to `/skills` → full CRUD cycle works for user skills; builtin skills are read-only.

---

## Phase 4: User Story 2 — Manage Skills via CLI (Priority: P2)

**Goal**: `assistant skill` subcommand with `list`, `show`, `create`, `delete`. `assistant persona skill-mode/skill-add/skill-remove`.

**Independent Test**: In a terminal, run `assistant skill list`, `assistant skill create --name test-skill --description "Test" --body-file /tmp/body.md`, `assistant skill show test-skill`, `assistant skill delete test-skill --yes` — all succeed. Also `assistant persona skill-mode default blacklist`.

- [ ] T031 [US2] Add `Command::Skill { command: SkillCommand }` top-level variant to `Cli` in `crates/interface-cli/src/main.rs`; define `enum SkillCommand { List { persona: Option<String> }, Show { name: String }, Create { name: String, description: String, body_file: Option<PathBuf> }, Delete { name: String, yes: bool }, Generate { description: String } }`
- [ ] T032 [US2] Implement `SkillCommand::List` handler in `crates/interface-cli/src/main.rs`:
  - Without `--persona`: call `registry.list()`, print formatted table to stdout
  - With `--persona <id>`: call `registry.list_for_persona()`, add ACCESS column to output
- [ ] T033 [US2] Implement `SkillCommand::Show` handler — look up skill from registry, print full SKILL.md content (frontmatter + body) to stdout; exit 1 if not found
- [ ] T034 [US2] Implement `SkillCommand::Create` handler:
  - If `--body-file` provided: read file; otherwise open `$EDITOR` or exit 1 if unset
  - Call `registry.create_user_skill()`
  - Print success message; exit 1 on error
- [ ] T035 [US2] Implement `SkillCommand::Delete` handler:
  - If not `--yes`: prompt `Delete skill '<name>'? [y/N]` using stdin
  - Call `registry.delete_user_skill()`; print success; exit 1 on error (builtin or not found)
- [ ] T036 [US2] Add `PersonaCommand::SkillMode { persona_id: String, mode: String }`, `PersonaCommand::SkillAdd { persona_id: String, skill_name: String }`, `PersonaCommand::SkillRemove { persona_id: String, skill_name: String }` variants to `enum PersonaCommand` in `crates/interface-cli/src/main.rs`
- [ ] T037 [US2] Implement `PersonaCommand::SkillMode` handler:
  - Validate persona exists; validate mode string
  - Call `persona_skill_access_store.set_mode()`
  - Print warning if switching whitelist↔blacklist with existing list entries
- [ ] T038 [US2] Implement `PersonaCommand::SkillAdd` handler:
  - Validate persona exists; check mode is not `all` (exit 1 with hint if it is)
  - Call `persona_skill_access_store.add_skill()`; print confirmation
- [ ] T039 [US2] Implement `PersonaCommand::SkillRemove` handler:
  - Validate persona exists; call `persona_skill_access_store.remove_skill()`; print confirmation

**Checkpoint**: `assistant skill --help` shows all subcommands. Full CLI CRUD cycle works end-to-end. `assistant persona --help` shows new skill-mode/skill-add/skill-remove variants.

---

## Phase 5: User Story 3 — Persona Skill Access Control (Priority: P3)

**Goal**: Each persona's active skill set is filtered by its access mode (all/whitelist/blacklist). Web UI page to manage mode and list per persona.

**Independent Test**: Set persona "default" to blacklist mode, add `agentskills-spec` to its list, run the agent with `--persona default`, confirm `agentskills-spec` skill is not referenced in context.

- [ ] T040 [US3] Wire `list_for_persona()` into the Orchestrator's skill-loading path in `crates/runtime` — find where `registry.list()` is called when building system prompt context and replace with `registry.list_for_persona(active_persona_id, pool)` (pool available via `StorageLayer`)
- [ ] T041 [P] [US3] Add persona skill access routes to web UI: define `PersonaSkillPagesState`, `persona_skill_router()` in a new section of `crates/web-ui/src/main.rs` (or in an extended `personas` module); wire into `protected_routes`
- [ ] T042 [P] [US3] Implement `skill_access` page handler in `crates/web-ui/src/` (personas module or new file):
  - Load persona record (id, name, skill_access_mode)
  - Load all skills + persona skill list
  - Render `templates/personas/skill_access.html`
- [ ] T043 [US3] Implement `set_skill_mode` handler — POST `/personas/:id/skills/mode`; calls `persona_skill_access_store.set_mode()`; redirects with flash warning if switching whitelist↔blacklist with existing entries
- [ ] T044 [US3] Implement `add_skill` handler — POST `/personas/:id/skills/add`; validates mode is not `all`; calls `persona_skill_access_store.add_skill()`; returns HTMX partial updating skill row
- [ ] T045 [US3] Implement `remove_skill` handler — DELETE `/personas/:id/skills/:skill`; calls `persona_skill_access_store.remove_skill()`; returns HTMX response removing/updating row
- [ ] T046 [US3] Complete `templates/personas/skill_access.html` — mode radio/select (all/whitelist/blacklist), table of all skills with add/remove buttons per row, mode-change form with HTMX, warn banner when switching modes with existing list

**Checkpoint**: Persona in blacklist mode with one skill listed — that skill is absent from agent context when that persona is active. Web UI access page loads and mode/list changes persist.

---

## Phase 6: User Story 4 — AI-Assisted Skill Generation (Priority: P4)

**Goal**: Users can describe a skill and the agent generates a valid SKILL.md draft, which they can review and save via web UI or CLI.

**Independent Test**: Run `assistant skill generate "Teach the agent to write ADR documents"` — output is a valid SKILL.md string with frontmatter. In web UI new-skill form, click "Generate with AI", fill description, confirm body textarea is populated.

- [ ] T047 [US4] Write `skills/agentskills-spec/SKILL.md` with full agentskills.io specification content:
  - Frontmatter: `name: agentskills-spec`, `description: ...`, `license: MIT`
  - Body: complete SKILL.md spec (frontmatter fields, validation rules, body conventions, examples)
  - This is the knowledge injected into the AI when generating skill drafts
- [ ] T048 [US4] Implement `SkillCommand::Generate` handler in `crates/interface-cli/src/main.rs`:
  - Build a prompt: "Using the agentskills-spec builtin, generate a valid SKILL.md for: <description>"
  - Submit to Orchestrator (same pattern as other CLI turns)
  - Print result to stdout; exit 1 on error
- [ ] T049 [US4] Implement `POST /skills/generate` handler in `crates/web-ui/src/skills/pages.rs`:
  - Accept JSON body `{ "description": "..." }`
  - Submit to Orchestrator with generation prompt
  - Return JSON `{ "body": "<generated SKILL.md content>" }` or `{ "error": "..." }`
  - Timeout at 30s → 504
- [ ] T050 [US4] Add "Generate with AI" button to `templates/skills/new.html`:
  - Text input for description
  - HTMX `hx-post="/skills/generate"` that swaps generated content into the `body` textarea on success
  - Show spinner while loading; show error message inline on failure
- [ ] T051 [US4] Add same "Generate with AI" section to `templates/skills/edit.html`

**Checkpoint**: `assistant skill generate "..."` prints a valid SKILL.md. Web UI generate button populates the body textarea. Generated SKILL.md passes `parse_skill_content()` validation.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T052 Write `docs/adr/` entry for skill management architecture (single registry, three-mode access, dual-write) per constitution requirement (VII)
- [ ] T053 [P] Update `AGENTS.md` "Recent Changes" section to document new `PersonaSkillAccessStore`, `SkillRegistry` CRUD methods, and web UI skills routes
- [ ] T054 [P] Run `make lint && make format` and resolve any clippy warnings or fmt diffs introduced by this feature
- [ ] T055 Verify dual-mode parity (constitution VIII): confirm skill filtering works when running `assistant orchestrator run` (single-binary) AND when using `assistant worker` with external bus

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — create files immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 — BLOCKS all user stories
- **Phase 3 (US1 Web UI CRUD)**: Depends on Phase 2 only
- **Phase 4 (US2 CLI)**: Depends on Phase 2 only — can run in parallel with Phase 3
- **Phase 5 (US3 Persona Access)**: Depends on Phase 2 + T018 (`list_for_persona`) — can start after Phase 2
- **Phase 6 (US4 AI Generation)**: Depends on Phase 2 (registry), T023/T028 (new form template) — can start after Phase 3
- **Phase 7 (Polish)**: Depends on all prior phases

### User Story Dependencies

- **US1 (P1)**: After Phase 2 — no story dependencies
- **US2 (P2)**: After Phase 2 — no story dependencies, parallelisable with US1
- **US3 (P3)**: After Phase 2 + T018 — no story dependencies
- **US4 (P4)**: After Phase 2 + T028/T029 (templates) — depends on US1 templates existing

### Within Each Phase

- Storage tasks (T009–T018) must be sequential where one builds on another
- T011 depends on T009 (migration must be registered before the upsert code can reference new column)
- T015–T017 can run in parallel (different methods, same file)
- Web UI handlers (T021–T025) can run in parallel (different handler functions)
- Templates (T026–T029) can run in parallel (different files)

---

## Parallel Example: Phase 2 (Foundational)

```
# These can run in parallel once T009 is done:
T011 — update upsert_to_db() in registry.rs
T012 — update PersonaRecord in personas.rs
T013 — implement PersonaSkillAccessStore in persona_skill_access.rs

# These can run in parallel once T013 is done:
T015 — create_user_skill()
T016 — update_user_skill()
T017 — delete_user_skill()
```

## Parallel Example: Phase 3 (US1 Web UI)

```
# After T019 (AppState) and T020 (router wired):
T021 — list handler + list.html
T022 — show handler + show.html
T023 — new_form/create handlers + new.html
T024 — edit_form/update handlers + edit.html
T025 — delete handler
# All template files (T026–T029) in parallel
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (file stubs)
2. Complete Phase 2: Foundational (CRITICAL — storage schema + SkillRegistry CRUD)
3. Complete Phase 3: US1 Web UI CRUD
4. **STOP and VALIDATE**: Navigate to `/skills`, create/edit/delete a user skill, verify builtin is read-only
5. Ship — this is the full MVP

### Incremental Delivery

1. Phase 1 + 2 → Foundation + dual-write storage
2. Phase 3 → Web UI CRUD (ship MVP)
3. Phase 4 → CLI skill management (ship US2)
4. Phase 5 → Persona access control filtering (ship US3)
5. Phase 6 → AI-assisted generation (ship US4)
6. Phase 7 → ADR + lint + parity verification

### Parallel Team Strategy

With two developers after Phase 2:

- Developer A: Phase 3 (US1 web UI) → Phase 6 (US4 AI generation)
- Developer B: Phase 4 (US2 CLI) → Phase 5 (US3 persona access)

---

## Notes

- `[P]` tasks = different files, no dependencies on incomplete tasks in same phase
- `[Story]` label maps each task to a user story for traceability
- Builtin protection must be checked in every write path (T015–T017, T023–T025, T034–T035)
- `cargo check -p assistant-storage` after Phase 2 before starting any UI/CLI work
- `make lint && make format` after each phase — constitution VII is non-negotiable
- Each checkpoint should be manually verified before advancing
