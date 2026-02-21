---
name: skill-file-read
description: >
  Read a file bundled with a skill (from its references/, scripts/, or assets/ directory).
  Use when a skill's instructions reference an external file you need to read, e.g.
  "see references/FORMS.md for field definitions".
license: Apache-2.0
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: '{"skill": {"type": "string", "description": "The skill name (kebab-case)"}, "path": {"type": "string", "description": "Relative path within the skill directory, e.g. references/FORMS.md"}}'
---

## Instructions

Read a file from a skill's auxiliary directory (references/, scripts/, or assets/).

### Parameters

- `skill` (string, required): The skill name in kebab-case (e.g. `memory-read`)
- `path` (string, required): Relative path within the skill directory (e.g. `references/FORMS.md`)

### Behavior

- Validates the path stays within the skill directory (no `..` traversal)
- Returns the file contents as plain text
- Returns a clear error if skill or file is not found

### Example

- "What fields does the form have?" -> read `references/FORMS.md` from the relevant skill
