# Data Model: Persona Editor UI

**Branch**: `002-persona-editor-ui` | **Phase**: 1 — Design

## Entities

### PersonaRecord (existing, in `assistant-storage`)

| Field        | Type            | Notes                                                                           |
| ------------ | --------------- | ------------------------------------------------------------------------------- |
| `id`         | `String`        | Primary key. Alphanumeric + `-` + `_` only. Validated by `validate_agent_id()`. |
| `name`       | `String`        | Human-readable display name.                                                    |
| `is_default` | `bool`          | Maintained by existing system; not surfaced in this feature's UI.               |
| `created_at` | `DateTime<Utc>` | Set by DB on insert.                                                            |
| `updated_at` | `DateTime<Utc>` | Updated by DB on modification.                                                  |

**Change**: Add `PersonaStore::create(id: &str, name: &str) -> Result<PersonaRecord>` method.

- Inserts a new row with `is_default = 0`.
- Returns `Err` (with descriptive message) if `id` already exists (SQLite `UNIQUE` violation).
- Does **not** set or change any default designation.

---

### PersonaFileSlot (view-model, not persisted)

This is a runtime view model constructed per request — not stored in the database.

| Field          | Type           | Notes                                                            |
| -------------- | -------------- | ---------------------------------------------------------------- |
| `filename`     | `&'static str` | One of the canonical filenames (see list below).                 |
| `exists`       | `bool`         | Whether the file is present on the filesystem.                   |
| `display_name` | `&'static str` | Human-readable label shown in the UI (e.g., "Soul", "Identity"). |
| `description`  | `&'static str` | One-line purpose description shown in the UI.                    |

**Canonical file slot list** (fixed, ordered for display):

| Filename       | Display Name | Description                                 |
| -------------- | ------------ | ------------------------------------------- |
| `SOUL.md`      | Soul         | Personality, values, and core truths        |
| `IDENTITY.md`  | Identity     | Name, role, and structured identity profile |
| `USER.md`      | User         | User profile, preferences, and timezone     |
| `MEMORY.md`    | Memory       | Curated long-term memory                    |
| `AGENTS.md`    | Agents       | Workspace rules and session startup ritual  |
| `TOOLS.md`     | Tools        | Environment-specific tool notes             |
| `BOOTSTRAP.md` | Bootstrap    | First-run onboarding ritual                 |
| `HEARTBEAT.md` | Heartbeat    | Periodic task checklist for the scheduler   |

---

### PersonaFileContent (view-model, not persisted)

Used by the file editor route only.

| Field        | Type           | Notes                                                          |
| ------------ | -------------- | -------------------------------------------------------------- |
| `persona_id` | `String`       | Parent persona ID.                                             |
| `filename`   | `&'static str` | One of the canonical filenames.                                |
| `content`    | `String`       | Raw markdown content of the file (empty string for new files). |
| `is_new`     | `bool`         | True when the file did not exist before opening the editor.    |

---

## State Transitions

### Persona Lifecycle (in scope for this feature)

```
[Not Exists] --create(id, name)--> [Exists, no files]
[Exists, no files] --create file--> [Exists, partial files]
[Exists, partial files] --edit/create files--> [Exists, all files]
[Exists, any state] --edit file--> [same state, file content updated]
```

### File Lifecycle

```
[Absent] --POST /personas/{id}/files/{filename} (new)--> [Present]
[Present] --POST /personas/{id}/files/{filename} (edit)--> [Present, updated content]
```

---

## Validation Rules

| Rule                   | Location                                  | Details                                                         |
| ---------------------- | ----------------------------------------- | --------------------------------------------------------------- |
| Persona ID format      | `validate_agent_id()` in `assistant-core` | Alphanumeric + `-` + `_`; no path separators.                   |
| Persona ID uniqueness  | `PersonaStore::create()`                  | DB-level UNIQUE constraint; returns error on conflict.          |
| Persona name non-empty | Route handler                             | Trim and reject blank name with HTTP 400.                       |
| Filename whitelist     | Route handler `persona_filename()` helper | Only the 8 canonical filenames accepted; all others → HTTP 400. |
| File content size      | Route handler                             | Reject content exceeding 2 MB to prevent OOM; return HTTP 413.  |

---

## File Path Resolution

```
base: dirs::home_dir() / ".assistant" / "agents" / {persona_id} /
file: base / {filename}
```

Example: `~/.assistant/agents/work/SOUL.md`

The directory is created automatically (`tokio::fs::create_dir_all`) when the first file is saved for a new persona. The directory may not exist for a persona that was just created (no error is shown for absent directories — all file slots are simply listed as "not yet created").
