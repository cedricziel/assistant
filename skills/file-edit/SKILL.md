---
name: file-edit
description: >
  Surgically edit a file by replacing the first occurrence of a search string
  with a replacement string. Returns an error if the search string is not found —
  no silent corruption.
license: Apache-2.0
compatibility: Requires filesystem access
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "false"
  params: '{"path": {"type": "string", "description": "Absolute or ~-relative path to the file to edit"}, "old_string": {"type": "string", "description": "Exact text to find in the file (verbatim match; returns error if not found)"}, "new_string": {"type": "string", "description": "Text to substitute in place of the first occurrence of old_string"}}'
---

## Instructions

Perform a targeted search-and-replace on a file. Only the first occurrence of
`old_string` is replaced. If `old_string` is not found the file is left
unchanged and an error is returned.

### Parameters

- `path` (string, required): Path to the file. Supports `~` expansion.
- `old_string` (string, required): Exact text to locate (verbatim match).
- `new_string` (string, required): Replacement text.

### Behavior

- Reads the file, replaces first occurrence of `old_string` with `new_string`, writes back
- Returns an error (without modifying the file) if `old_string` is not found
- Use `file-write` when you want to replace the entire file

### When to use

- Updating a specific line or section in a config or markdown file
- Changing a single value without rewriting the whole file
- Any time you need a safe, targeted edit

### Example

Updating a version number in a config file:

- path: "~/project/config.toml"
- old_string: 'version = "1.0.0"'
- new_string: 'version = "1.1.0"'
