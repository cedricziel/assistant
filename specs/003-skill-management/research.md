# Research: Skill Management via Web UI and CLI

**Feature**: 003-skill-management
**Date**: 2026-03-29

## Finding 1 — Existing skill storage architecture

**Context**: The `SkillRegistry` in `crates/storage/src/registry.rs` already has `register()`, `remove()`, `get()`, and `list()`. It persists to a `skills` SQLite table (migration 002) and keeps an in-memory `HashMap` cache. The `body` field of `SkillDef` is **not** stored in the DB today — only metadata.

**Decision**: Add `body_text TEXT NOT NULL DEFAULT ''` via migration 028. Update `upsert_to_db()` to include the body. This lets the web UI read skill body from DB without touching disk.

**Alternative considered**: Read from `dir_path` on demand for the edit form. Rejected — requires filesystem access from the web server, which may not have the skills dir mounted in remote deployments.

## Finding 2 — Migration numbering

**Context**: Latest migration is `027_slack_active_threads.sql`. New migrations must be:

- `028_skill_body.sql`
- `029_persona_skill_access.sql`

Both are registered in `crates/storage/src/lib.rs` in the `migrations` slice.

## Finding 3 — Persona table location

**Context**: `personas` table was added in migration 026 (`ALTER TABLE assistant_agents RENAME TO personas`). Current columns: `id TEXT PK`, `name TEXT`, `is_default BOOLEAN`, `created_at`, `updated_at`.

**Decision**: Add `skill_access_mode TEXT NOT NULL DEFAULT 'all' CHECK(...)` via `ALTER TABLE` in migration 029. Add a new `persona_skill_list (persona_id, skill_name)` table in the same migration.

## Finding 4 — CLI subcommand pattern

**Context**: `crates/interface-cli/src/main.rs` uses clap `Subcommand` enums. Existing `Persona` command has `List`, `Create`, `Use` variants. Pattern is to match `Command::Persona { command }` and delegate.

**Decision**: Add `Command::Skill { command: SkillCommand }` (new top-level subcommand). Extend `PersonaCommand` with `SkillMode { persona_id, mode }`, `SkillAdd { persona_id, skill_name }`, `SkillRemove { persona_id, skill_name }`.

## Finding 5 — Web UI page pattern

**Context**: `crates/web-ui/src/a2a/pages.rs` shows the pattern: `AppState`/custom state struct → Askama template structs → `render_template()` helper → Axum handlers. Routes go in `main.rs` under `protected_routes`. Templates in `crates/web-ui/templates/<module>/`.

**Decision**: Create `crates/web-ui/src/skills/` module following the same pattern as `a2a/pages.rs`. Add `SkillRegistry` and `PersonaSkillAccessStore` (via the pool) to a new `SkillsPagesState`.

## Finding 6 — AppState does not currently hold SkillRegistry

**Context**: `AppState` in `main.rs` holds `pool`, `agent_id`, limits, and bus config — but not `SkillRegistry`. The registry is built in `run_with_args()` and not attached to state.

**Decision**: Add `registry: Arc<SkillRegistry>` to `AppState`. Wire it in `run_with_args()` after the existing registry construction block. The skills pages will use this registry directly for CRUD.

## Finding 7 — Skill filtering by persona access mode

**Context**: The runtime's skill loading path (`bootstrap::skill_dirs` + `registry.load_from_dirs()`) runs at startup. The active persona is known at that point. The Orchestrator holds the registry.

**Decision**: Add a `list_for_persona(persona_id, pool)` async method to `SkillRegistry` that queries `personas.skill_access_mode` and `persona_skill_list`, then filters `self.list()` accordingly. This is called by the Orchestrator when building the system prompt context instead of the plain `list()` call. The existing `list()` is retained for management UIs (where unfiltered listing is needed).

## Finding 8 — AI generation builtin

**Context**: Builtins are embedded via `include_dir!("../../skills")` in `crates/skills/src/parser.rs`. Each builtin is a directory with a `SKILL.md`.

**Decision**: Create `skills/agentskills-spec/SKILL.md` encoding the agentskills.io spec structure. This will be compiled into the binary. The `generate` handler will submit a prompt to the Orchestrator with an instruction to use this builtin and output a valid `SKILL.md` for the user's description.

## Finding 9 — Disk write location for user skills

**Context**: User skills live in `~/.assistant/skills/<name>/SKILL.md`. The `sync_builtins_to_disk` function already writes files here. The `default_workspace_dir` and home dir lookup patterns are established.

**Decision**: New `create_user_skill(name, description, body)` method on `SkillRegistry` will:

1. Compute `~/.assistant/skills/<name>/` via `dirs::home_dir()`
2. Create the directory with `tokio::fs::create_dir_all`
3. Write `SKILL.md` with `tokio::fs::write`
4. Call `self.register(parsed_def)` to upsert to DB
5. Return error if any step fails (disk error propagates before DB write)

Deleting a user skill: `tokio::fs::remove_dir_all` + `self.remove(name)`.
