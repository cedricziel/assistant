# CLI Contract: Skill & Persona Skill Access Commands

**Feature**: 003-skill-management
**Date**: 2026-03-29

## `assistant skill` — Skill CRUD

### `assistant skill list [--persona <id>]`

Lists all skills in the registry.

- Without `--persona`: prints all skills (name, source, description).
- With `--persona <id>`: applies the persona's access mode filter and marks each skill with its effective access status for that persona.

**Output format** (table to stdout):

```
NAME              SOURCE     DESCRIPTION
git-commit        builtin    Teach the agent how to write...
my-custom         user       My personal skill
code-review       installed  Code review helper
```

With `--persona work` (in blacklist mode with `git-commit` denied):

```
NAME              SOURCE     ACCESS    DESCRIPTION
git-commit        builtin    denied    Teach the agent how to write...
my-custom         user       allowed   My personal skill
```

**Exit codes**: 0 on success, 1 on error (e.g., persona not found).

---

### `assistant skill show <name>`

Prints the full skill content: frontmatter + body.

**Output**: SKILL.md content to stdout.

**Errors**:

- Skill not found → exit 1 with message to stderr.

---

### `assistant skill create --name <name> --description <desc> [--body-file <path>]`

Creates a new user skill.

- `--name`: kebab-case, max 64 chars, must not already exist.
- `--description`: max 1024 chars.
- `--body-file`: path to a markdown file for the body. If omitted, opens `$EDITOR` (or prints an error if `$EDITOR` is unset).

**Behaviour**:

1. Validate name (kebab-case, not duplicate).
2. Write `~/.assistant/skills/<name>/SKILL.md` with YAML frontmatter + body.
3. Register in DB.
4. Print: `Created skill '<name>' at ~/.assistant/skills/<name>/`

**Errors**:

- Name already exists → exit 1.
- Name is not kebab-case → exit 1 with validation message.
- Disk write failure → exit 1.

---

### `assistant skill delete <name> [--yes]`

Deletes a user or installed skill.

- Without `--yes`: prompts `Delete skill '<name>'? [y/N]`.
- With `--yes`: skips confirmation.

**Behaviour**:

1. Reject if `source_type = 'builtin'` → exit 1 with message.
2. Remove DB record.
3. Remove `~/.assistant/skills/<name>/` directory.
4. Print: `Deleted skill '<name>'`

**Errors**:

- Skill not found → exit 1.
- Builtin skill → exit 1: `Cannot delete builtin skill '<name>'.`

---

### `assistant skill generate "<description>"`

Generates a SKILL.md draft using the AI agent.

- `<description>`: plain-language description of what the skill should do.

**Behaviour**:

1. Invoke Orchestrator with a prompt that loads the `agentskills-spec` builtin and asks for a SKILL.md for the given description.
2. Print the generated SKILL.md content to stdout.
3. User can pipe or redirect output: `assistant skill generate "..." > ~/.assistant/skills/my-skill/SKILL.md`

**Errors**:

- LLM not configured → exit 1.
- Generation fails or times out → exit 1 with error message.

---

## `assistant persona` — Skill Access Extensions

### `assistant persona skill-mode <persona-id> <all|whitelist|blacklist>`

Sets the skill access mode for a persona.

**Behaviour**:

1. Validate `persona-id` exists.
2. Validate mode is one of `all`, `whitelist`, `blacklist`.
3. Update `personas.skill_access_mode`.
4. If switching between `whitelist` and `blacklist` (either direction) and the persona has existing list entries: print a warning:
   `Warning: existing skill list will now be interpreted as a <new-mode>.`
5. Print: `Set skill access mode for '<persona-id>' to '<mode>'`

**Errors**:

- Persona not found → exit 1.
- Invalid mode → exit 1 with usage hint.

---

### `assistant persona skill-add <persona-id> <skill-name>`

Adds a skill to the persona's whitelist or blacklist.

**Behaviour**:

1. Validate persona exists.
2. Validate persona mode is `whitelist` or `blacklist` (not `all` — adding to list in `all` mode is a no-op that would confuse users).
3. Insert `(persona_id, skill_name)` into `persona_skill_list` (no-op if already present).
4. Print: `Added '<skill-name>' to <mode> list for '<persona-id>'`

**Errors**:

- Persona not found → exit 1.
- Persona in `all` mode → exit 1: `Persona '<id>' is in 'all' mode. Set a mode first with: assistant persona skill-mode <id> <whitelist|blacklist>`

---

### `assistant persona skill-remove <persona-id> <skill-name>`

Removes a skill from the persona's skill list.

**Behaviour**:

1. Validate persona exists.
2. Delete `(persona_id, skill_name)` row (silent if not present).
3. Print: `Removed '<skill-name>' from skill list for '<persona-id>'`

**Errors**:

- Persona not found → exit 1.
