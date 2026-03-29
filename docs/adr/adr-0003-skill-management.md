# ADR-0003: Skill Management Architecture

**Status**: Accepted
**Date**: 2026-03-29
**Feature**: `003-skill-management`

## Context

The assistant needs a way for users to create, read, update, and delete skills — both from a web UI and the CLI. Skills teach the assistant how to perform tasks by injecting text into the system prompt. Personas need the ability to restrict which skills are active.

## Decision

### Single Registry with Persona Access Modes

Skills are stored in a **single shared registry** (`SkillRegistry`). Personas do not have their own skill sets — instead, each persona has an **access mode** (`all` / `whitelist` / `blacklist`) that determines which skills from the global registry are visible when that persona is active.

- `all` (default): every skill is available
- `whitelist`: only skills explicitly listed in `persona_skill_list` are available
- `blacklist`: all skills except those listed in `persona_skill_list` are available

This avoids duplicating skill content across personas while giving administrators fine-grained control.

### Dual-Write (Disk + SQLite)

All skill writes persist to **both disk and SQLite** in lockstep, in this order:

1. Write `~/.assistant/skills/<name>/SKILL.md` to disk
2. Upsert to the `skills` SQLite table (including the `body_text` column)

Disk is canonical. If disk write fails, the operation aborts. If SQLite write fails after disk write, a warning is logged but the skill survives (it will be re-read from disk on next startup). The `body_text` column in SQLite exists purely for the web UI to populate edit forms without filesystem access.

### Source Types and Mutability

| Source    | Create | Edit | Delete | Notes                                     |
| --------- | ------ | ---- | ------ | ----------------------------------------- |
| Builtin   | No     | No   | No     | Embedded in binary                        |
| Project   | No     | No   | No     | From `.assistant/skills/` in project root |
| User      | Yes    | Yes  | Yes    | From `~/.assistant/skills/`               |
| Installed | No     | Yes  | Yes    | Installed from external source            |

Builtin and Project-scoped skills are read-only through the management UI and CLI.

### CLI Namespacing

Skill management is namespaced as `assistant skill <action>` (not `assistant skills`). Persona skill-access management is under `assistant persona skill-mode/skill-add/skill-remove`.

### Constitution Compliance

- **VIII (Dual-Mode Parity)**: `list_for_persona()` is called from the Orchestrator's system-prompt builder, so filtering works identically in single-binary and distributed-worker modes.
- **VII (Constitution)**: All writes go through `SkillRegistry` methods; no direct table mutations.

## Alternatives Considered

1. **Per-persona skill copies**: Rejected — duplicates content, makes updates to builtins harder.
2. **Single whitelist only** (no blacklist): Rejected — user clarified they want both modes so admins can either curate or exclude.
3. **Separate `PersonaSkillRegistry` type**: Rejected — the filtering logic is a thin wrapper around the global registry, not a separate store.

## Consequences

- Adding a new skill source type requires updating the `SkillSource` enum and the `create_user_skill` / `delete_user_skill` guards.
- The `body_text` column is a cache that may drift from disk in edge cases (e.g., manual file edits). The authoritative value is always the `SKILL.md` file.
- Persona skill lists must be maintained separately from the skill registry; deleting a skill does not automatically clean up `persona_skill_list` rows (foreign-key cascade handles this at the DB level via `ON DELETE CASCADE`).
