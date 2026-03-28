# HTTP Route Contracts: Persona Editor UI

**Branch**: `002-persona-editor-ui`
**Crate**: `assistant-web-ui` (`crates/web-ui`)
**Module**: `src/contexts.rs`
**Auth**: All routes require session authentication (existing `require_auth` middleware).

---

## Existing Routes (unchanged)

| Method | Path                 | Handler         | Description                                                     |
| ------ | -------------------- | --------------- | --------------------------------------------------------------- |
| `GET`  | `/personas`          | `show_contexts` | List all personas with ID, name, and current-session indicator. |
| `POST` | `/personas/{id}/use` | `use_context`   | Switch the active persona for the current web session.          |

**Change to `GET /personas`**: Add a "New Persona" button and link each persona's ID/name to `GET /personas/{id}`.

---

## New Routes

### `GET /personas/new`

**Purpose**: Display the new-persona creation form.

**Response**: `200 OK`, HTML page with form fields for `id` (text input) and `name` (text input).

**Template**: `templates/personas/new.html`

**Query params**:

- `error` (optional): URL-encoded error message to display if redirected back after a failed POST.

---

### `POST /personas`

**Purpose**: Create a new persona.

**Request body** (`application/x-www-form-urlencoded`):

| Field  | Required | Constraints                                                       |
| ------ | -------- | ----------------------------------------------------------------- |
| `id`   | Yes      | Passes `validate_agent_id()`: alphanumeric + `-` + `_`, non-empty |
| `name` | Yes      | Non-empty after trimming whitespace                               |

**Success response**: `303 See Other` → `Location: /personas/{id}`

**Error responses**:

| Condition                        | Response                                                                      |
| -------------------------------- | ----------------------------------------------------------------------------- |
| Missing or blank `id`            | `303 See Other` → `/personas/new?error=ID+is+required`                        |
| `id` fails `validate_agent_id()` | `303 See Other` → `/personas/new?error=Invalid+ID+format`                     |
| Missing or blank `name`          | `303 See Other` → `/personas/new?error=Name+is+required`                      |
| Duplicate `id`                   | `303 See Other` → `/personas/new?error=A+persona+with+this+ID+already+exists` |
| Storage error                    | `500 Internal Server Error`, plain text body                                  |

---

### `GET /personas/{id}`

**Purpose**: Show the detail view for a persona — lists all 8 canonical markdown file slots with present/absent indicator and create/edit links.

**Path params**:

- `id`: persona ID.

**Success response**: `200 OK`, HTML page.

**Template**: `templates/personas/detail.html`

**Error responses**:

| Condition                        | Response                      |
| -------------------------------- | ----------------------------- |
| Persona `id` not found in DB     | `404 Not Found`, plain text   |
| `id` fails `validate_agent_id()` | `400 Bad Request`, plain text |

---

### `GET /personas/{id}/files/{filename}`

**Purpose**: Open the editor for a specific markdown file. If the file does not exist, opens an empty editor (creating mode).

**Path params**:

- `id`: persona ID.
- `filename`: one of the 8 canonical filenames (case-sensitive, e.g., `SOUL.md`).

**Success response**: `200 OK`, HTML page with `<textarea>` pre-populated with file content (or empty for new files).

**Template**: `templates/personas/file_editor.html`

**Error responses**:

| Condition                             | Response                                |
| ------------------------------------- | --------------------------------------- |
| `filename` not in canonical whitelist | `400 Bad Request`, plain text           |
| Persona `id` not found                | `404 Not Found`, plain text             |
| File read error (not "not found")     | `500 Internal Server Error`, plain text |

---

### `POST /personas/{id}/files/{filename}`

**Purpose**: Save (create or overwrite) the content of a persona markdown file.

**Path params**:

- `id`: persona ID.
- `filename`: one of the 8 canonical filenames.

**Request body** (`application/x-www-form-urlencoded`):

| Field     | Required                  | Constraints                     |
| --------- | ------------------------- | ------------------------------- |
| `content` | Yes (may be empty string) | Maximum 2 MB (2,097,152 bytes). |

**Success response**: `303 See Other` → `Location: /personas/{id}/files/{filename}?saved=1`

**Query param on redirect**: `saved=1` causes the editor to display a success confirmation banner.

**Error responses**:

| Condition                             | Response                                                                        |
| ------------------------------------- | ------------------------------------------------------------------------------- |
| `filename` not in canonical whitelist | `400 Bad Request`, plain text                                                   |
| Persona `id` not found                | `404 Not Found`, plain text                                                     |
| Content exceeds 2 MB                  | `413 Content Too Large`, plain text                                             |
| Filesystem write error                | `303 See Other` → `/personas/{id}/files/{filename}?error={url-encoded-message}` |

**Note on filesystem write error**: Redirecting back to the editor with the error in a query param is preferred over a 500 page, so the user can see the error without losing the ability to copy their content from the browser's back cache. The editor template must re-display the textarea populated from the POST body when an error query param is present. Because the content is not passed in the redirect, the user must re-enter or use the browser back button — this is acceptable for the initial version.

---

## Template Summary

| Template path                         | Purpose                                                          |
| ------------------------------------- | ---------------------------------------------------------------- |
| `templates/personas/page.html`        | Existing list — updated to add New Persona button + detail links |
| `templates/personas/new.html`         | New persona creation form                                        |
| `templates/personas/detail.html`      | Persona detail: lists 8 file slots with status badges            |
| `templates/personas/file_editor.html` | File editor: textarea + save/cancel + unsaved-change JS          |
