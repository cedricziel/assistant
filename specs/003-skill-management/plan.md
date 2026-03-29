# Implementation Plan: Skill Management via Web UI and CLI

**Branch**: `003-skill-management` | **Date**: 2026-03-29 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/003-skill-management/spec.md`

## Summary

Add full CRUD management of skills through both the web UI and CLI, with per-persona skill
access control (three modes: all / whitelist / blacklist), plus AI-assisted skill generation
powered by an embedded agentskills.io builtin skill. Skills are stored in a single shared
registry; all writes persist to both `~/.assistant/skills/<name>/SKILL.md` on disk and
the SQLite registry in lockstep.

## Technical Context

**Language/Version**: Rust 2021 edition, workspace resolver 2
**Primary Dependencies**: `sqlx` (SQLite, hand-rolled migrations), `axum` (HTTP), `askama`
(server-side templates), HTMX + Stimulus.js (frontend), `clap` (CLI), `tokio` (async runtime),
`gray_matter` (SKILL.md frontmatter parsing), `tokio::fs` (async file I/O)
**Storage**: SQLite — `~/.assistant/assistant.db`; skills also written to `~/.assistant/skills/<name>/SKILL.md`
**Testing**: `cargo test`, `#[tokio::test]`, `StorageLayer::new_in_memory()` for DB tests
**Target Platform**: Linux / macOS server (and desktop)
**Project Type**: Multi-crate workspace CLI + web-service
**Performance Goals**: Skill list page renders in < 500ms; skill create/edit persists in < 200ms
**Constraints**: No network access for AI generation (embedded spec); dual-write must be atomic from the user's perspective; builtin skills are read-only
**Scale/Scope**: Typical user has < 100 skills; design for correctness, not high throughput

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

| Principle                             | Status  | Notes                                                                                                                                                                                                                                                                  |
| ------------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| I. Crate-First Modularity             | ✅ Pass | New `PersonaSkillAccessStore` goes in `assistant-storage`. New skill CRUD methods extend existing `SkillRegistry`. Web UI routes go in `assistant-web-ui`. CLI commands extend `assistant-cli`. No new crates required — feature fits existing crate responsibilities. |
| II. Trait-Based DI                    | ✅ Pass | `SkillRegistry` is already shared as `Arc<SkillRegistry>`. Access control filtering can be added as a method on `SkillRegistry` that accepts a persona ID.                                                                                                             |
| III. Test Discipline                  | ✅ Pass | All async tests use `#[tokio::test]`; DB tests use `StorageLayer::new_in_memory()`. Disk write tests use `tempfile`.                                                                                                                                                   |
| IV. Observability                     | ✅ Pass | All new library code uses `tracing` macros; `println!` prohibited.                                                                                                                                                                                                     |
| V. Simplicity & YAGNI                 | ✅ Pass | No new abstractions beyond what the feature requires. Access mode stored as TEXT enum in personas table (no separate enum crate).                                                                                                                                      |
| VI. Interface Parity via Orchestrator | ✅ Pass | Skill filtering at load-time is done in the existing bootstrap/load path, not per-interface. All interfaces benefit automatically.                                                                                                                                     |
| VII. Code Quality Gate                | ✅ Pass | fmt, clippy, machete enforced by pre-commit hooks.                                                                                                                                                                                                                     |
| VIII. Dual-Mode Parity                | ✅ Pass | Skill registry and access filtering are used at startup in all modes (single-binary and distributed worker). No per-interface skill loading.                                                                                                                           |

**Post-design re-check**: No violations anticipated. All new storage is in `assistant-storage`; all new routes are in `assistant-web-ui`; all new CLI commands are in `assistant-cli`.

## Project Structure

### Documentation (this feature)

```text
specs/003-skill-management/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
migrations/
├── 028_skill_body.sql              # Add body_text to skills table
└── 029_persona_skill_access.sql    # Persona access mode + skill list table

crates/storage/src/
├── registry.rs          # Extend: body_text persistence, create/update/delete with disk sync
├── persona_skill_access.rs  # New: PersonaSkillAccessStore (mode + list CRUD)
└── lib.rs               # Expose PersonaSkillAccessStore convenience constructor

crates/interface-cli/src/
└── main.rs              # Add: Skill subcommand (list/show/create/delete/generate)
                         # Add: PersonaCommand variants (skill-mode/skill-add/skill-remove)

crates/web-ui/src/
├── skills/
│   ├── mod.rs           # Router, state, handlers (list/show/new/create/edit/update/delete)
│   └── pages.rs         # Askama template handlers
├── main.rs              # Wire skills router into protected routes; add SkillRegistry to AppState
└── common.rs            # (no change expected)

crates/web-ui/templates/
├── skills/
│   ├── list.html        # Skills list page
│   ├── show.html        # Skill detail/view page
│   ├── new.html         # Create skill form
│   └── edit.html        # Edit skill form (body textarea + description)
└── personas/
    └── skill_access.html  # Persona skill access mode + list management page

skills/
└── agentskills-spec/
    └── SKILL.md         # New embedded builtin: agentskills.io spec knowledge for AI generation
```

**Structure Decision**: Feature spans three existing crates (`assistant-storage`, `assistant-web-ui`, `assistant-cli`) plus two new migration files and one new embedded builtin skill. No new crates are needed.

## Complexity Tracking

No constitution violations. No complexity justification required.

---

## Phase 0: Research

### research.md

See `research.md` for full findings. Key decisions summarised here:

**Decision 1 — Dual-write strategy**

- **Chosen**: Write disk first, then upsert to SQLite. If disk write fails, abort entirely. If SQLite upsert fails, log a warning but do not attempt to roll back the disk write (the registry reloads from disk on next startup, so the skill will not be lost).
- **Rationale**: Disk is the source of truth for the skills spec; SQLite is a cache/index. This ordering means the canonical file always exists if the user's write appeared to succeed.
- **Alternative**: Write SQLite first — rejected because if the process crashes after DB write but before disk write, the skill exists in DB but not on disk, causing confusing inconsistencies on reload.

**Decision 2 — Builtin skill protection**

- **Chosen**: Check `source_type = 'builtin'` in all write paths; return an error before touching disk or DB.
- **Rationale**: Builtins are embedded in the binary and synced to disk on startup. Editing them via UI would be overwritten on next launch — a confusing UX.

**Decision 3 — Persona access mode storage**

- **Chosen**: Add `skill_access_mode TEXT NOT NULL DEFAULT 'all'` to the `personas` table; add a separate `persona_skill_list` table with `(persona_id, skill_name)` pairs. No separate "mode" enum crate.
- **Rationale**: Avoids over-engineering. SQLite TEXT columns with application-level validation are sufficient for a 3-value enum. The separate list table allows clean multi-row management.

**Decision 4 — AI generation approach**

- **Chosen**: Add a `generate` subcommand to `assistant skill` (CLI) and a "Generate with AI" button on the web UI new-skill form. Both invoke the Orchestrator with a specialized prompt that references the `agentskills-spec` builtin skill. The output is printed to stdout (CLI) or pre-populated in the textarea (web UI).
- **Rationale**: Reuses the existing Orchestrator + LLM path. The `agentskills-spec` builtin is embedded at compile time (same pattern as all other builtins) so no network call is needed.

**Decision 5 — `body_text` in SQLite**

- **Chosen**: Add `body_text TEXT` column to the `skills` table. Populated on upsert; used to restore body in web edit form without reading disk.
- **Rationale**: The web UI needs to pre-populate the body textarea from the DB record (e.g., if the user is on a remote server). Reading the file is also fine, but the DB column provides a simpler path and keeps the registry self-contained for listing/display.

---

## Phase 1: Design & Contracts

### Data model

See `data-model.md` for full entity definitions.

**Migration 028 — `skills` table body column:**

```sql
ALTER TABLE skills ADD COLUMN body_text TEXT NOT NULL DEFAULT '';
```

**Migration 029 — Persona skill access:**

```sql
ALTER TABLE personas ADD COLUMN skill_access_mode TEXT NOT NULL DEFAULT 'all'
    CHECK(skill_access_mode IN ('all', 'whitelist', 'blacklist'));

CREATE TABLE IF NOT EXISTS persona_skill_list (
    persona_id  TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
    skill_name  TEXT NOT NULL,
    PRIMARY KEY (persona_id, skill_name)
);
```

**Updated `skills` table shape** (after migrations 002, 008, 028):

| Column          | Type     | Notes                                  |
| --------------- | -------- | -------------------------------------- |
| `name`          | TEXT PK  | kebab-case, max 64 chars               |
| `description`   | TEXT     | max 1024 chars                         |
| `dir_path`      | TEXT     | absolute path to skill dir             |
| `tier`          | TEXT     | fixed `"knowledge"` (legacy column)    |
| `enabled`       | BOOLEAN  | always TRUE for active skills          |
| `source_type`   | TEXT     | `builtin`/`user`/`installed`/`project` |
| `license`       | TEXT     | optional                               |
| `metadata_json` | TEXT     | JSON frontmatter extras                |
| `body_text`     | TEXT     | full SKILL.md body (markdown)          |
| `created_at`    | DATETIME |                                        |
| `updated_at`    | DATETIME |                                        |

**`personas` table** (after migration 026, 029):

| Column              | Type     | Notes                                        |
| ------------------- | -------- | -------------------------------------------- |
| `id`                | TEXT PK  |                                              |
| `name`              | TEXT     |                                              |
| `is_default`        | BOOLEAN  |                                              |
| `skill_access_mode` | TEXT     | `all`/`whitelist`/`blacklist`; default `all` |
| `created_at`        | DATETIME |                                              |
| `updated_at`        | DATETIME |                                              |

**`persona_skill_list` table**:

| Column       | Type    | Notes                                                                             |
| ------------ | ------- | --------------------------------------------------------------------------------- |
| `persona_id` | TEXT FK | references `personas(id)` ON DELETE CASCADE                                       |
| `skill_name` | TEXT    | references a skill name (soft reference — no FK to avoid cascade on skill delete) |
| PK           |         | `(persona_id, skill_name)`                                                        |

### Interface contracts

See `contracts/` directory for full details.

**CLI interface** (`assistant skill`, `assistant persona`):

```
assistant skill list [--persona <id>]
    → table: name | source | description | (access status if --persona given)

assistant skill show <name>
    → print: frontmatter + body

assistant skill create --name <name> --description <desc> [--body-file <path>]
    → creates ~/.assistant/skills/<name>/SKILL.md + upserts to DB
    → error if name exists or is builtin

assistant skill delete <name> [--yes]
    → removes ~/.assistant/skills/<name>/ + removes from DB
    → error if source_type = 'builtin'

assistant skill generate "<description>"
    → invokes Orchestrator with agentskills-spec builtin in context
    → prints generated SKILL.md content to stdout

assistant persona skill-mode <persona-id> <all|whitelist|blacklist>
    → updates personas.skill_access_mode; warns if list becomes misinterpreted

assistant persona skill-add <persona-id> <skill-name>
    → inserts into persona_skill_list (no-op if already present)
    → error if persona mode is 'all'

assistant persona skill-remove <persona-id> <skill-name>
    → deletes from persona_skill_list
```

**Web UI routes** (`assistant-web-ui`):

```
GET  /skills               → list all skills
GET  /skills/new           → new skill form (+ "Generate with AI" button)
POST /skills               → create skill
GET  /skills/:name         → skill detail view
GET  /skills/:name/edit    → edit skill form
PUT  /skills/:name         → update skill (HTMX form)
DELETE /skills/:name       → delete skill (HTMX confirm)

GET  /personas/:id/skills  → persona skill access page (mode selector + list)
POST /personas/:id/skills/mode   → set access mode
POST /personas/:id/skills/add    → add skill to list
DELETE /personas/:id/skills/:skill_name → remove from list

POST /skills/generate      → AI generation endpoint (returns SKILL.md draft as JSON)
```

### Agent context update

Updated via `.specify/scripts/bash/update-agent-context.sh claude` — see `CLAUDE.md` additions below.
