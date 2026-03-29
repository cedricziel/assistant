# Data Model: Skill Management

**Feature**: 003-skill-management
**Date**: 2026-03-29

## Entities

### Skill (updated)

Stored in `skills` table. Corresponds to `SkillDef` in `crates/skills/src/skill.rs`.

| Field           | Type     | Validation                             | Notes                                         |
| --------------- | -------- | -------------------------------------- | --------------------------------------------- |
| `name`          | TEXT PK  | kebab-case, 1–64 chars                 | Directory name must match                     |
| `description`   | TEXT     | non-empty, ≤ 1024 chars                | Human-readable summary                        |
| `dir_path`      | TEXT     | valid path                             | `~/.assistant/skills/<name>/` for user skills |
| `tier`          | TEXT     | fixed `"knowledge"`                    | Legacy column; always "knowledge"             |
| `enabled`       | BOOLEAN  | always TRUE                            | Future use                                    |
| `source_type`   | TEXT     | `builtin`/`user`/`installed`/`project` | Determines mutability                         |
| `license`       | TEXT     | optional                               | From SKILL.md frontmatter                     |
| `metadata_json` | TEXT     | valid JSON                             | Arbitrary frontmatter extras                  |
| `body_text`     | TEXT     | non-empty for user skills              | Full SKILL.md body (NEW, migration 028)       |
| `created_at`    | DATETIME |                                        | Set on insert                                 |
| `updated_at`    | DATETIME |                                        | Updated on each upsert                        |

**Mutability rules**:

- `source_type = 'builtin'`: read-only in all write paths
- `source_type = 'user'` or `'installed'`: mutable via UI and CLI
- `source_type = 'project'`: read-only (filesystem-scanned, not writable via UI)

**Lifecycle**:

1. Create → disk write (`~/.assistant/skills/<name>/SKILL.md`) → DB upsert → in-memory insert
2. Update → disk write → DB upsert → in-memory update
3. Delete → DB delete → in-memory remove → disk remove (`remove_dir_all`)
4. Builtin → embedded at compile time → synced to disk on startup → read-only thereafter

### Persona (extended)

Existing entity in `personas` table, extended with skill access fields.

| Field               | Type     | Validation                    | Notes                                |
| ------------------- | -------- | ----------------------------- | ------------------------------------ |
| `id`                | TEXT PK  | letters, digits, `-`, `_`     |                                      |
| `name`              | TEXT     | non-empty                     | Display name                         |
| `is_default`        | BOOLEAN  | at most 1 true                | Unique partial index                 |
| `skill_access_mode` | TEXT     | `all`/`whitelist`/`blacklist` | DEFAULT `'all'` (NEW, migration 029) |
| `created_at`        | DATETIME |                               |                                      |
| `updated_at`        | DATETIME |                               |                                      |

**Mode semantics**:

- `all`: every skill in the registry is loaded for this persona; `persona_skill_list` is ignored
- `whitelist`: only skills with a matching `persona_skill_list` row are loaded; all others excluded
- `blacklist`: all skills are loaded except those with a matching `persona_skill_list` row

**Mode change behaviour**: When switching from `whitelist` to `blacklist` (or vice versa), the existing `persona_skill_list` rows are preserved but reinterpreted under the new mode. The CLI warns the user. When switching to `all`, the list rows are preserved but not consulted.

### PersonaSkillList (new)

Stored in `persona_skill_list` table (migration 029).

| Field        | Type    | Validation                                  | Notes                              |
| ------------ | ------- | ------------------------------------------- | ---------------------------------- |
| `persona_id` | TEXT FK | references `personas(id)` ON DELETE CASCADE |                                    |
| `skill_name` | TEXT    | non-empty, matches a known skill name       | Soft reference — no FK to `skills` |
| PK           |         | `(persona_id, skill_name)`                  |                                    |

**Notes**:

- No FK to `skills` — a skill can be in the list even if it was deleted (it simply won't match anything at filter time). This avoids cascade complexity.
- Removing a skill from the registry does not automatically clean up `persona_skill_list` rows. Orphaned rows are silently ignored at filter time.

## State Transitions

### Persona access mode transitions

```
         set mode=whitelist
     ┌──────────────────────┐
     │                      ▼
   [all] ◄──────────── [whitelist]
     │    set mode=all       │
     │                       │ set mode=blacklist
     │   set mode=blacklist  ▼
     └──────────────────► [blacklist]
                             │
                             └── set mode=all ──► [all]
```

### Skill lifecycle

```
[absent] ──create──► [present, user/installed]
                              │
                    ┌─────────┤
                    │         │
                  edit      delete
                    │         │
                    ▼         ▼
              [present, updated]  [absent]
```

Builtins:

```
[embedded in binary] ──startup──► [present, builtin, read-only]
```

## Filtering Algorithm

Given active persona `P` with `skill_access_mode = M` and skill list `L`:

```
match M:
  "all"       → return all_skills
  "whitelist" → return all_skills.filter(|s| L.contains(s.name))
  "blacklist" → return all_skills.filter(|s| !L.contains(s.name))
```

This filtering is applied in `SkillRegistry::list_for_persona(persona_id, pool)` before
the skills are assembled into the system prompt context.
