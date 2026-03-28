# Research: Persona Editor UI

**Branch**: `002-persona-editor-ui` | **Phase**: 0 — Research

## Decisions

### 1. No New Crate Required

**Decision**: Implement entirely within the existing `assistant-web-ui` crate (`crates/web-ui`).

**Rationale**: The feature is a UI extension of the existing `/personas` page already in `contexts.rs`. Creating a new crate for UI pages within the same web server violates YAGNI and the "no organizational-only crates" rule in the constitution. All dependencies (Axum, Askama, PersonaStore, tokio::fs) are already present.

**Alternatives considered**:

- New crate `assistant-persona-manager`: Rejected — no independent compile/test benefit; would just wrap existing storage and filesystem calls already accessible from `web-ui`.

---

### 2. Existing PersonaStore Covers List/Get; Needs `create()` Addition

**Decision**: Add a `PersonaStore::create(id, name)` method to `assistant-storage` that returns `Err` if the ID already exists (as opposed to the existing `ensure_exists` which is idempotent).

**Rationale**: `ensure_exists` silently does nothing on duplicate IDs; FR-010 requires an explicit error when a duplicate is submitted. A new `create()` method with a `UNIQUE` constraint violation check gives clean, testable semantics.

**Alternatives considered**:

- Reuse `ensure_exists` and add a pre-check `get()`: Rejected — introduces a TOCTOU race and requires two round-trips.

---

### 3. File I/O via `tokio::fs` Directly in Route Handlers

**Decision**: Route handlers in `contexts.rs` read and write persona markdown files using `tokio::fs::read_to_string` and `tokio::fs::write` directly. No new service abstraction is introduced.

**Rationale**: The file operations are straightforward (read one file, write one file). Adding a `PersonaFileService` trait for two operations would be a premature abstraction. The constitution principle V (Simplicity/YAGNI) applies.

**File path resolution**: `dirs::home_dir().join(".assistant/agents/{id}/{filename}")` — consistent with the existing `ensure_agent_dirs` helper already in `contexts.rs`.

**Filename validation**: Only the fixed set of known filenames (SOUL.md, IDENTITY.md, USER.md, MEMORY.md, AGENTS.md, TOOLS.md, BOOTSTRAP.md, HEARTBEAT.md) is accepted. Any other filename is rejected with HTTP 400, preventing path traversal attacks.

**Alternatives considered**:

- `std::fs` (synchronous): Rejected — violates constitution async rule; `tokio::fs` is required for file I/O in route handlers.
- Generic "any filename" support: Rejected — spec explicitly scopes to fixed file slots; allows whitelist-based security.

---

### 4. Askama Templates + HTMX for Editor UI

**Decision**: Use Askama server-side templates (consistent with all existing pages) with HTMX for the unsaved-change warning and save confirmation feedback. No client-side framework is added.

**Rationale**: Every existing page in the web-ui uses Askama + HTMX + Stimulus. Introducing a separate client-side editor library (e.g., CodeMirror) would violate YAGNI for a plain-text markdown editor. A `<textarea>` with standard form POST is sufficient for the MVP.

**Unsaved-change warning**: Handled via a Stimulus controller (`beforeunload` event) or a small inline `<script>` in the template — consistent with the existing pattern for workflow editors.

**Alternatives considered**:

- CodeMirror or Monaco: Rejected — adds significant JS bundle weight; plain textarea is sufficient for markdown.
- Full SPA approach: Rejected — inconsistent with project conventions.

---

### 5. New Persona Creation: Form-based POST

**Decision**: A "New Persona" button on the personas list page links to `GET /personas/new` (form page), which POSTs to `POST /personas` (create handler). On success, redirect to `GET /personas/{id}`. On duplicate ID error, re-render the form with an error message.

**Rationale**: Consistent with the existing webhook and workflow creation patterns (form → POST → redirect).

**ID validation**: Reuse the existing `validate_agent_id()` function from `assistant-core` (already used in `use_context`).

---

### 6. Constitution Gates — All Clear

| Gate                              | Status      | Notes                                                                         |
| --------------------------------- | ----------- | ----------------------------------------------------------------------------- |
| Crate-First Modularity            | ✅ Pass     | Feature stays in existing `assistant-web-ui`                                  |
| Trait-Based DI                    | ✅ Pass     | No new cross-crate concrete dependencies                                      |
| Test Discipline                   | ✅ Pass     | Unit tests use `StorageLayer::new_in_memory()`; file I/O tested via temp dirs |
| Observability                     | ✅ Pass     | `tracing` macros in handlers                                                  |
| Simplicity/YAGNI                  | ✅ Pass     | No new abstractions beyond `PersonaStore::create()`                           |
| Interface Parity via Orchestrator | ✅ N/A      | Admin UI page; no LLM turn routing                                            |
| Code Quality Gate                 | ✅ Required | `cargo fmt`, `clippy -D warnings`, `machete` before merge                     |
| Dual-Mode Parity                  | ✅ N/A      | Web-UI admin page uses no MessageBus; works in both modes by definition       |
