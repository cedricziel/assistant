# Web UI Contract: Skill Management Routes

**Feature**: 003-skill-management
**Date**: 2026-03-29

## Route Table

| Method | Path                          | Handler                    | Description                       |
| ------ | ----------------------------- | -------------------------- | --------------------------------- |
| GET    | `/skills`                     | `skills::list`             | List all skills                   |
| GET    | `/skills/new`                 | `skills::new_form`         | New skill form                    |
| POST   | `/skills`                     | `skills::create`           | Create skill                      |
| GET    | `/skills/:name`               | `skills::show`             | Skill detail                      |
| GET    | `/skills/:name/edit`          | `skills::edit_form`        | Edit skill form                   |
| POST   | `/skills/:name`               | `skills::update`           | Update skill (HTMX `_method=PUT`) |
| POST   | `/skills/:name/delete`        | `skills::delete`           | Delete skill (HTMX confirm)       |
| POST   | `/skills/generate`            | `skills::generate`         | AI generation                     |
| GET    | `/personas/:id/skills`        | `personas::skill_access`   | Persona skill access page         |
| POST   | `/personas/:id/skills/mode`   | `personas::set_skill_mode` | Set access mode                   |
| POST   | `/personas/:id/skills/add`    | `personas::add_skill`      | Add to persona list               |
| DELETE | `/personas/:id/skills/:skill` | `personas::remove_skill`   | Remove from persona list          |

## Request / Response Contracts

### GET `/skills`

**Response**: HTML page
**Template**: `templates/skills/list.html`
**Data**:

- List of all skills: name, source_type, description, enabled
- Each row links to `/skills/:name` (view) and `/skills/:name/edit` (edit)
- Delete button triggers HTMX `DELETE /skills/:name/delete` with confirm dialog
- "New Skill" button links to `/skills/new`

---

### GET `/skills/new`

**Response**: HTML form page
**Template**: `templates/skills/new.html`
**Form fields**:

- `name` (text, required, kebab-case hint)
- `description` (text, required, max 1024)
- `body` (textarea, required, markdown)
- "Generate with AI" button → triggers HTMX POST `/skills/generate` → populates `body` textarea

---

### POST `/skills`

**Request body** (form-encoded):

```
name=my-skill&description=Does+something&body=---\nname: my-skill\n...
```

**Success**: Redirect to `GET /skills/:name`
**Error**: Re-render form with validation error inline

**Validation**:

- `name`: non-empty, kebab-case, ≤ 64 chars, not already taken
- `description`: non-empty, ≤ 1024 chars
- `body`: non-empty, must parse as valid SKILL.md (frontmatter + body)

---

### GET `/skills/:name`

**Response**: HTML skill detail page
**Template**: `templates/skills/show.html`
**Data**:

- name, description, source_type, license, allowed_tools, body_text
- "Edit" link if not builtin
- "Delete" button if not builtin

---

### GET `/skills/:name/edit`

**Response**: HTML edit form
**Template**: `templates/skills/edit.html`
**Pre-populated from DB**: description, body_text
**Form fields**: `description`, `body`
**"Generate with AI" button**: as on new form

---

### POST `/skills/:name` (update)

**Request body** (form-encoded):

```
description=Updated+description&body=---\nname: ...
```

**Success**: Redirect to `GET /skills/:name`
**Error**: Re-render edit form with validation error

**Validation**: Same as create; name is not editable.

---

### POST `/skills/:name/delete`

**Request**: No body required (HTMX hx-delete or form POST)
**Success**: HTMX response removes the row from DOM (or redirect to `/skills`)
**Error**: Returns 400 with message if builtin

---

### POST `/skills/generate`

**Request body** (JSON or form-encoded):

```json
{ "description": "Teach the agent to write conventional commits" }
```

**Response** (JSON):

```json
{ "body": "---\nname: conventional-commits\n..." }
```

**Behaviour**: Invokes Orchestrator with `agentskills-spec` builtin in scope. Returns generated SKILL.md text. The frontend pre-populates the `body` textarea via HTMX `hx-swap`.

**Errors**:

- LLM error → 500 with JSON `{ "error": "..." }`
- Timeout → 504

---

### GET `/personas/:id/skills`

**Response**: HTML page
**Template**: `templates/personas/skill_access.html`
**Data**:

- Persona id, name, current `skill_access_mode`
- Mode selector (radio or select: all / whitelist / blacklist)
- List of all skills with checkboxes / toggle for current list membership
- Mode change form POSTs to `/personas/:id/skills/mode`
- Each skill row has add/remove buttons posting to `/personas/:id/skills/add` or `DELETE /personas/:id/skills/:skill`

---

### POST `/personas/:id/skills/mode`

**Request body**: `mode=whitelist`
**Success**: Redirect to `GET /personas/:id/skills`
**Warning**: If switching between whitelist/blacklist with existing list entries, renders a warning banner on redirect

---

### POST `/personas/:id/skills/add`

**Request body**: `skill_name=git-commit`
**Validation**: Persona must not be in `all` mode
**Success**: HTMX response updates the skill's row status
**Error**: 400 if persona in `all` mode

---

### DELETE `/personas/:id/skills/:skill`

**Success**: HTMX response removes/updates the skill row
**Error**: 404 if persona not found
