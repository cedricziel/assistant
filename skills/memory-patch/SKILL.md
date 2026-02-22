---
name: memory-patch
description: >
  Surgically edit a section of a persistent memory file (SOUL.md, IDENTITY.md, USER.md,
  or MEMORY.md) using search-and-replace. Unlike soul-update, this replaces only the first
  occurrence of the search text, leaving the rest of the file intact. Returns an error if
  the search text is not found — no silent corruption.
license: Apache-2.0
compatibility: Requires filesystem access
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "false"
  params: '{"target": {"type": "string", "description": "File to patch: soul, identity, user, or memory"}, "search": {"type": "string", "description": "Exact text to find in the file (verbatim match; returns error if not found)"}, "replace": {"type": "string", "description": "Text to substitute in place of the first occurrence of search"}}'
---

## Instructions

Perform a targeted search-and-replace on one of the four persistent memory files.

### Parameters

- `target` (string, required): Which file to patch — one of: `soul`, `identity`, `user`, `memory`
- `search` (string, required): Exact text to locate (verbatim, first occurrence)
- `replace` (string, required): Text to substitute in place of the found text

### When to use

- Filling in a blank field in IDENTITY.md (e.g. replacing `_(pick something)_` with an actual name)
- Correcting a specific fact in USER.md without rewriting the whole file
- Updating a single bullet point or section in MEMORY.md
- Fixing a preference in SOUL.md without touching the rest

Prefer `memory-patch` over `soul-update` with `mode=replace` whenever you only need to change one part of a file.

### Error handling

If `search` is not found, the skill returns an error and leaves the file unchanged.

### Example

Filling in the Name field in IDENTITY.md:

```json
{
  "target": "identity",
  "search": "- **Name:** _(pick something — doesn't have to be \"Assistant\")_",
  "replace": "- **Name:** Aria"
}
```

Updating a timezone in USER.md:

```json
{
  "target": "user",
  "search": "- **Timezone:**",
  "replace": "- **Timezone:** Europe/Berlin"
}
```
