---
description: "Task list for Observability UI Improvements"
---

# Tasks: Observability UI Improvements

**Feature**: `003-observability-ui-improvements`
**Input**: `specs/003-observability-ui-improvements/`
**Branch**: `003-observability-ui-improvements`

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- File paths relative to repo root

---

## Phase 1: Setup

**Purpose**: Verify environment and locate key files before implementation

- [ ] T001 Verify `crates/runtime/src/orchestrator/mod.rs` — locate `run_turn` result handling and `otel_turn` span variable
- [ ] T002 Verify `crates/storage/src/traces.rs` — locate `TraceSummary` struct and `list_recent_traces_for_agent` signature
- [ ] T003 [P] Verify `crates/storage/src/logs.rs` — locate `list_recent_for_agent` signature and existing WHERE clause

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: No foundational infrastructure changes required — existing crates, tables, and OTel wiring are in place. This phase is a pass-through checkpoint.

**⚠️ CRITICAL**: Confirm `make check` passes on the current branch before starting user stories.

- [ ] T004 Run `make check` to confirm workspace compiles cleanly on `003-observability-ui-improvements`

**Checkpoint**: All three user stories can begin in parallel after T004.

---

## Phase 3: User Story 1 — Propagate Worker Failure to Trace Status (P1) 🎯 MVP

**Goal**: Failed turns show ✗ in the Status column on `/traces` instead of appearing as successes.

**Independent Test**: Run a turn that deliberately errors; confirm `distributed_traces.attributes` contains `"otel.status_code":"Error"` for the root span.

### Implementation

- [ ] T005 [US1] Set `Status::Error` on `otel_turn` when `run_turn` returns `Err` in `crates/runtime/src/orchestrator/mod.rs`

### Tests

- [ ] T006 [US1] Add unit test in `crates/runtime/src/orchestrator/mod.rs` (or adjacent test module): simulate a failed turn, assert root span has `otel.status_code = "Error"` via `StorageLayer::new_in_memory()`

**Checkpoint**: `make test -p assistant-runtime` passes; failed turns now surface in `/traces` Status column.

---

## Phase 4: User Story 2 — Replied Badge + Interface Facet (P1)

**Goal**: Traces list shows whether each turn sent a reply, and traces can be filtered by originating interface (Slack / Scheduler / CLI).

**Independent Test**:

- Insert a trace with `tool_name = 'reply'` via `StorageLayer::new_in_memory()`, assert `has_reply = true`
- Insert a trace without reply tool, assert `has_reply = false`
- Insert a trace with `interface = 'Slack'` in attributes, assert `interface` is extracted correctly

### Storage

- [ ] T007 [P] [US2] Add `interface: Option<String>` and `has_reply: bool` fields to `TraceSummary` struct in `crates/storage/src/traces.rs`
- [ ] T008 [US2] Add `TraceFilter` struct (with `skill`, `status`, `conversation`, `min_duration_ms`, `interface`, `since`, `until` fields) to `crates/storage/src/traces.rs` and export from `crates/storage/src/lib.rs`
- [ ] T009 [US2] Replace `skill_filter: Option<&str>` with `filter: &TraceFilter` on `list_recent_traces_for_agent` in `crates/storage/src/traces.rs`; add `interface` + `has_reply` SELECT columns and `interface` WHERE clause to the query
- [ ] T010 [US2] Update all call sites of `list_recent_traces_for_agent` in `crates/web-ui/src/traces.rs` to pass a `&TraceFilter` (wire `None` for new fields until T013)

### Tests

- [ ] T011 [P] [US2] Add unit tests in `crates/storage/src/traces.rs` for `has_reply` (with/without reply tool) and `interface` extraction from JSON attributes

### Web UI

- [ ] T012 [US2] Add `has_reply: bool` and `interface: Option<String>` to the `TraceRow` view model in `crates/web-ui/src/traces.rs`; wire `interface` filter from `?interface=` query param through `TraceFilter`
- [ ] T013 [US2] Add "Interface" facet group (radio: All / Slack / Scheduler / Cli) wired with HTMX to `crates/web-ui/templates/traces/page.html`; add "Replied" column (✓ green / – amber for Slack no-reply / – grey otherwise)

**Checkpoint**: `make test -p assistant-storage` passes; `/traces?interface=Slack` filters correctly; Replied column visible in UI.

---

## Phase 5: User Story 3 — Conversation + Time Range Filters + Cross-link to Logs (P2)

**Goal**: Both `/traces` and `/logs` pages have visible sidebar inputs for conversation UUID and time range (since/until); trace detail page links to conversation-scoped logs.

**Independent Test**:

- Insert logs for two different conversation IDs, filter by one, assert only matching logs returned
- Verify `/logs?conversation=<uuid>` returns logs spanning all turns for that conversation
- Verify "View conversation logs →" link appears on trace detail when `conversation_id` is set

### Storage

- [ ] T014 [P] [US3] Add `conversation_id: Option<&str>`, `since: Option<DateTime<Utc>>`, `until: Option<DateTime<Utc>>` parameters to `list_recent_for_agent` in `crates/storage/src/logs.rs`; add corresponding WHERE clauses to query
- [ ] T015 [US3] Update call site in `crates/web-ui/src/logs.rs` to pass `None` for all three new params (no-op until T018)

### Tests

- [ ] T016 [P] [US3] Add unit tests in `crates/storage/src/logs.rs`: insert logs for two conversations, filter by one, assert correct scoping; test `since`/`until` range filtering

### Web UI — Traces

- [ ] T017 [US3] Add `since`/`until` parsing (`%Y-%m-%dT%H:%M` → `DateTime<Utc>`) to `crates/web-ui/src/traces.rs`; wire through `TraceFilter.since`/`TraceFilter.until`; add conversation UUID input to sidebar wired through `TraceFilter.conversation`
- [ ] T018 [US3] Add two `<input type="datetime-local">` fields (`since`, `until`) and conversation UUID `<input type="text">` to traces sidebar in `crates/web-ui/templates/traces/page.html`

### Web UI — Logs

- [ ] T019 [US3] Add `since`/`until` + `conversation_id` parsing to `crates/web-ui/src/logs.rs`; wire to `list_recent_for_agent` call
- [ ] T020 [US3] Add `since`/`until` datetime inputs and conversation UUID input to logs sidebar in `crates/web-ui/templates/logs/page.html`

### Web UI — Trace Detail Cross-link

- [ ] T021 [US3] Add "View conversation logs →" link (`/logs?conversation={{ conv }}`) to header bar in `crates/web-ui/templates/traces/detail.html` (conditional on `conversation_id` being Some)

**Checkpoint**: `make test -p assistant-storage` passes; `/logs?conversation=06effb79-9644-41f0-9e21-a3312c9d408c` returns all conversation logs; time range inputs functional on both pages; cross-link present on trace detail.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T022 [P] Run `make lint` (`cargo clippy --workspace -- -D warnings`) and fix any new warnings
- [ ] T023 [P] Run `make format` (`cargo fmt --all`) to ensure formatting is clean
- [ ] T024 Validate Definition of Done: silent-failure scenario (Slack turn, no reply) shows amber `–`; failed turns show ✗; `/logs?conversation=<uuid>` works; time range inputs filter correctly

---

## Dependencies & Execution Order

### Story Dependencies

```
T001-T004  (Setup + Foundation)  → no deps; start immediately
T005-T006  (US1 — Runtime fix)   → depends on T004; no deps on US2 or US3
T007-T013  (US2 — Replied+Iface) → depends on T004; no deps on US1 or US3
T014-T021  (US3 — Filters+Link)  → depends on T004; no deps on US1 or US2
T022-T024  (Polish)              → depends on all stories complete
```

### Within US2

```
T007 (TraceSummary fields) ──┐
T008 (TraceFilter struct)  ──┤→ T009 (query update) → T010 (call site) → T012 (view model) → T013 (template)
T011 (storage tests)         │ [parallel with T009 once T007+T008 done]
```

### Within US3

```
T014 (logs storage) → T015 (call site update) → T019 (logs web-ui) → T020 (logs template)
T016 (storage tests)  [parallel with T015]
T017 (traces web-ui) → T018 (traces template)  [parallel with T019 once T014 done]
T021 (detail link)    [parallel with T018/T020 — no backend deps]
```

### Parallel Opportunities

- US1, US2, US3 can be worked in parallel by separate developers after T004
- T007 and T008 within US2 can be done in parallel
- T011 (storage tests) can start as soon as T009 is done
- T014 and T016 within US3 can proceed in parallel
- T017 and T019 can proceed in parallel (different files) once T014 is done

---

## Parallel Example: US2 + US3 Simultaneously

```bash
# Developer A: US2 storage
Task T007: Add fields to TraceSummary in crates/storage/src/traces.rs
Task T008: Add TraceFilter struct in crates/storage/src/traces.rs
# Then: T009 → T010 → T012 → T013

# Developer B: US3 storage
Task T014: Extend list_recent_for_agent in crates/storage/src/logs.rs
# Then: T015 → T019 → T020, T017 → T018, T021
```

---

## Implementation Strategy

### MVP First (US1 only — 2 tasks)

1. Complete Phase 1: Setup (T001–T003)
2. Complete Phase 2: Foundation check (T004)
3. Complete Phase 3: US1 — T005 + T006
4. **STOP and VALIDATE**: Failed turns now show ✗ in `/traces`
5. Merge US1 to branch, then proceed to US2 + US3

### Incremental Delivery

1. Setup + Foundation → T001–T004
2. US1 (T005–T006) → Test → Commit `fix(runtime): set error status on failed turn spans`
3. US2 (T007–T013) → Test → Commit `feat(storage+web-ui): interface facet + replied badge`
4. US3 (T014–T021) → Test → Commit `feat(storage+web-ui): conversation + time range filters`
5. Polish (T022–T024) → Commit `chore: lint and format pass`
