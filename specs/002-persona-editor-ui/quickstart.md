# Quickstart: Persona Editor UI

**Branch**: `002-persona-editor-ui`

## What Changes Where

All changes are confined to two locations:

1. **`crates/storage/src/personas.rs`** — add `PersonaStore::create()` method.
2. **`crates/web-ui/src/contexts.rs`** — add 5 new route handlers.
3. **`crates/web-ui/templates/personas/`** — add 3 new templates, update 1 existing.

No new crates. No new dependencies. No database migrations (the `personas` table already exists).

## Build & Run

```sh
# Build the web-ui binary
make build

# Run the web UI (requires ASSISTANT_WEB_TOKEN)
ASSISTANT_WEB_TOKEN=dev make run-webui

# Navigate to http://localhost:8080/personas
```

## Running Tests

```sh
# Unit tests for the storage change
cargo test -p assistant-storage

# Unit tests for web-ui route handlers
cargo test -p assistant-web-ui

# Lint (must pass before commit)
make lint && make format
```

## Key Files

| File                                                | Change                                |
| --------------------------------------------------- | ------------------------------------- |
| `crates/storage/src/personas.rs`                    | Add `create(id, name)` method         |
| `crates/web-ui/src/contexts.rs`                     | Add 5 handlers + update router        |
| `crates/web-ui/templates/personas/page.html`        | Add New Persona button + detail links |
| `crates/web-ui/templates/personas/new.html`         | New: creation form                    |
| `crates/web-ui/templates/personas/detail.html`      | New: file slot list                   |
| `crates/web-ui/templates/personas/file_editor.html` | New: markdown editor                  |

## Route Map

```
GET  /personas               → list (updated: adds detail link + New Persona button)
GET  /personas/new           → new persona form
POST /personas               → create persona  →  303 /personas/{id}
GET  /personas/{id}          → persona detail (file slots)
GET  /personas/{id}/files/{filename}   → file editor
POST /personas/{id}/files/{filename}   → save file  →  303 back to editor
```

## Security Notes

- All routes are behind the existing `require_auth` session middleware.
- `{filename}` path parameter is validated against an 8-entry whitelist before any filesystem access. Any filename not in the whitelist returns HTTP 400 immediately — no filesystem access occurs.
- `{id}` is validated via `validate_agent_id()` before any DB or filesystem access.
- File content is capped at 2 MB.
