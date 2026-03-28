# Implementation Plan: Persona Editor UI

**Branch**: `002-persona-editor-ui` | **Date**: 2026-03-28 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-persona-editor-ui/spec.md`

## Summary

Add a web UI to the existing `/personas` section of `assistant-web-ui` that lets administrators view, create, and edit the markdown files (SOUL.md, IDENTITY.md, USER.md, etc.) that define each persona's behaviour. Implementation is entirely within `crates/web-ui` (route handlers + Askama templates) and `crates/storage` (one new `PersonaStore::create()` method). No new crates, no new dependencies, no database migrations.

## Technical Context

**Language/Version**: Rust 2021 edition
**Primary Dependencies**: Axum (HTTP server), Askama (server-side templates), tokio::fs (async file I/O), sqlx + SQLite (PersonaStore), HTMX + Stimulus.js (frontend interactivity)
**Storage**: SQLite (existing `personas` table) + local filesystem (`~/.assistant/agents/{id}/`)
**Testing**: `cargo test` with `#[tokio::test]`, `StorageLayer::new_in_memory()`, `tempfile` crate for filesystem tests
**Target Platform**: Linux / macOS server (same as existing web-ui binary)
**Project Type**: web-service (server-side rendered)
**Performance Goals**: Standard page load (<500ms); file save round-trip <200ms for files up to 2 MB
**Constraints**: 2 MB max file size; filename whitelist enforced before any filesystem access
**Scale/Scope**: Single-user or small-team admin UI; not designed for concurrent editing

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

| Principle                             | Status      | Notes                                                                                                              |
| ------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------ |
| I. Crate-First Modularity             | ✅ Pass     | Feature lives in existing `assistant-web-ui`; no new crate created (YAGNI: no independent testability benefit).    |
| II. Trait-Based DI                    | ✅ Pass     | No new cross-crate concrete types. PersonaStore is already a concrete struct used within `web-ui`.                 |
| III. Test Discipline                  | ✅ Pass     | All async tests use `#[tokio::test]`; DB tests use `StorageLayer::new_in_memory()`; file I/O tests use `tempfile`. |
| IV. Observability                     | ✅ Pass     | All route handlers use `tracing` macros; no `println!`.                                                            |
| V. Simplicity/YAGNI                   | ✅ Pass     | No new service abstractions. `PersonaStore::create()` is the only new method. File I/O is inline in handlers.      |
| VI. Interface Parity via Orchestrator | ✅ N/A      | Admin UI page; no LLM turn routing required.                                                                       |
| VII. Code Quality Gate                | ✅ Required | `cargo fmt --all`, `cargo clippy --workspace -D warnings`, `cargo machete --with-metadata` before merge.           |
| VIII. Dual-Mode Parity                | ✅ N/A      | Web-UI management page; uses no MessageBus; works identically in single-binary and distributed modes.              |

No violations. Complexity Tracking table is empty.

## Project Structure

### Documentation (this feature)

```text
specs/002-persona-editor-ui/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── http-routes.md   # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
crates/storage/src/
└── personas.rs          # Add PersonaStore::create(id, name) method

crates/web-ui/
├── src/
│   └── contexts.rs      # Add 5 new route handlers + router entries
└── templates/
    └── personas/
        ├── page.html         # Updated: New Persona button + detail links per row
        ├── new.html          # New: persona creation form
        ├── detail.html       # New: file slot list for one persona
        └── file_editor.html  # New: markdown textarea editor
```

**Structure Decision**: Single-project, server-side rendered web service. Modifications are confined to two existing crates. Templates follow the established Askama pattern (`{% extends "base.html" %}`).

## Phase 0 Research

See [research.md](research.md) for full findings. Summary of decisions:

1. **No new crate** — implement in existing `assistant-web-ui`.
2. **`PersonaStore::create()`** — new method that fails on duplicate ID (vs. idempotent `ensure_exists`).
3. **`tokio::fs` inline** — no service layer abstraction for two file operations.
4. **Askama + HTMX + Stimulus** — consistent with all existing pages; plain `<textarea>` sufficient for markdown editing.
5. **Filename whitelist** — 8 canonical filenames; reject all others at HTTP 400; prevents path traversal.
6. **PRG pattern** — all POST handlers redirect to GET on success; error messages passed as query params on redirect.

## Phase 1 Design

See [data-model.md](data-model.md) and [contracts/http-routes.md](contracts/http-routes.md) for full details.

### New Routes

| Method | Path                              | Description                              |
| ------ | --------------------------------- | ---------------------------------------- |
| `GET`  | `/personas/new`                   | New persona form                         |
| `POST` | `/personas`                       | Create persona (PRG → `/personas/{id}`)  |
| `GET`  | `/personas/{id}`                  | Persona detail: lists 8 file slots       |
| `GET`  | `/personas/{id}/files/{filename}` | File editor (read or empty)              |
| `POST` | `/personas/{id}/files/{filename}` | Save file (PRG → editor with `?saved=1`) |

### Key Implementation Notes

- `{filename}` must be resolved through a `persona_filename(s: &str) -> Option<&'static str>` whitelist helper before any filesystem access.
- Directory creation (`tokio::fs::create_dir_all`) happens on first file save, not on persona creation.
- The unsaved-change warning uses a small inline Stimulus controller on the editor page — consistent with the `workflow-editor` controller pattern.
- The `POST /personas` handler calls `PersonaStore::create()`, which returns an `Err` containing the SQLite UNIQUE violation message on duplicate; the handler converts this to a redirect with an `error` query param.
- The `show_contexts` handler (`GET /personas`) is updated to add a "New" button and make persona IDs/names into links to `GET /personas/{id}`.

### Constitution Re-check (Post-Design)

All gates continue to pass. No new violations introduced by the detailed design.
