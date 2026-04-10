## ADDED Requirements

### Requirement: SkillPlugin injects the skill catalog into context — skills are instructions, not tools

`SkillPlugin` SHALL implement the `Plugin` trait and use filesystem-discovered `SkillDef` records to inject an `<available_skills>` XML catalog into the message context via `transform_context`. Skills are _instructions_ that guide model behavior — they are NOT registered as tools in the `ToolExecutor`. The `load-skill` and `list-skills` builtin tools SHALL be deleted from `assistant-tool-executor`. The model activates skills by reading their `SKILL.md` file directly via its existing `file-read` tool using the `<location>` path provided in the catalog.

#### Scenario: Catalog injected into context before LLM call

- **WHEN** `SkillPlugin` is registered and skills are discovered on the filesystem
- **THEN** `transform_context` prepends a `System`-role message containing an `<available_skills>` XML block (name, description, location per skill) and a brief instruction telling the model to use its `file-read` tool on the `<location>` path to load a skill's full instructions

#### Scenario: Model activates a skill via file-read, not a tool call

- **WHEN** the model determines a skill is relevant and calls `file-read` with the path from `<location>`
- **THEN** it receives the raw `SKILL.md` content (frontmatter + body) and proceeds with those instructions — no `load-skill` tool invocation occurs

#### Scenario: No skills without the plugin

- **WHEN** `SkillPlugin` is not registered
- **THEN** no catalog is injected; the model has no knowledge of available skills

### Requirement: SkillPlugin discovers skills from the filesystem at session start, with no SQLite backing

`SkillPlugin` SHALL scan configured skill directories on `on_session_start` and build an in-memory catalog (`HashMap<String, SkillDef>`). It SHALL NOT read from or write to SQLite. The filesystem is the source of truth. Scan paths SHALL follow the agentskills.io convention:

- `<project>/.agents/skills/` (cross-client interoperability)
- `<project>/.<client>/skills/` (client-native, e.g. `.assistant/skills/`)
- `~/.agents/skills/` (user-level, cross-client)
- `~/.<client>/skills/` (user-level, client-native)

Project-level skills SHALL take precedence over user-level skills when names collide. A warning SHALL be logged on collision.

#### Scenario: Project skill shadows user skill of same name

- **WHEN** a skill named `tdd` exists in both `<project>/.agents/skills/` and `~/.agents/skills/`
- **THEN** the project-level skill is used; a `warn!` is logged indicating the shadowing

#### Scenario: Empty scan produces no catalog

- **WHEN** no skill directories exist or all directories are empty
- **THEN** `transform_context` returns the message list unmodified (no `<available_skills>` block injected)

### Requirement: SkillPlugin allowlists skill directories for file-read access

`SkillPlugin` SHALL register all discovered skill base directories with the loop's permission system (or equivalent allowlist) so the model can read `SKILL.md`, `scripts/`, and `references/` files without triggering user confirmation prompts. Without this, every skill activation and every referenced script causes a permission dialog.

#### Scenario: Model reads SKILL.md without permission prompt

- **WHEN** `SkillPlugin` is registered and the model calls `file-read` on a skill's `SKILL.md` path
- **THEN** the read succeeds without requiring user confirmation

#### Scenario: Model reads a bundled script without permission prompt

- **WHEN** a `SKILL.md` body references `scripts/run.py` and the model calls `file-read` on the absolute path
- **THEN** the read succeeds without requiring user confirmation

### Requirement: load-skill and list-skills builtin tools are deleted from assistant-tool-executor

The `LoadSkillHandler` (`load-skill`) and `ListSkillsHandler` (`list-skills`) builtins SHALL be removed from `crates/tool-executor/src/builtins/`. `list-skills` is redundant — the catalog in the system prompt already tells the model what skills are available. `load-skill` is redundant — the model reads `SKILL.md` directly via `file-read`. The `ToolExecutor` SHALL have no knowledge of skills.

#### Scenario: Tool list no longer contains load-skill or list-skills

- **WHEN** a `ToolExecutor` is constructed with `register_builtins()`
- **THEN** neither `load-skill` nor `list-skills` appear in the registered tool list

### Requirement: SkillRegistry is removed from StorageLayer and SQLite

The `SkillRegistry` struct backed by SQLite in `assistant-storage` SHALL be deleted. Skill discovery and storage is the responsibility of `SkillPlugin`'s in-memory `HashMap`. No skill data SHALL be written to `assistant.db`.

#### Scenario: Skills survive without a database

- **WHEN** `AgentLoop` runs with `SkillPlugin` configured but no `StoragePlugin`
- **THEN** skills are discovered, the catalog is injected, and the model activates skills correctly — no SQLite connection is required

### Requirement: SkillPlugin is constructed with scan paths and an optional persona filter

`SkillPlugin::new(scan_dirs: Vec<PathBuf>, persona_filter: Option<PersonaFilter>) -> Self` SHALL be the constructor. The `persona_filter` allows restricting which skills are visible to a given persona (allowlist/blocklist), preserving the existing per-persona access control without SQLite.

#### Scenario: Persona blocklist hides a skill from the catalog

- **WHEN** a `PersonaFilter` blocks skill `bash-scripting` for persona `safe-mode`
- **THEN** `bash-scripting` does not appear in the `<available_skills>` catalog injected for that session
