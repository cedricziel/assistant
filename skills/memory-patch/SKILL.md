---
name: memory-patch
description: Surgically edit a section of a persistent memory file (SOUL.md, IDENTITY.md, USER.md, or MEMORY.md) using search-and-replace. Unlike soul-update, this replaces only the first occurrence of the search text, leaving the rest of the file intact.
version: "1.0"
author: assistant
tier: builtin
params:
  target:
    description: Which memory file to patch — "soul", "identity", "user", or "memory"
    type: string
    required: true
  search:
    description: Exact text to find in the file (must match verbatim; if not found, returns an error without modifying the file)
    type: string
    required: true
  replace:
    description: Replacement text that overwrites the first occurrence of the search text
    type: string
    required: true
---

# memory-patch

Perform a targeted search-and-replace on one of the four persistent memory files:
`SOUL.md`, `IDENTITY.md`, `USER.md`, or `MEMORY.md`.

## When to use

- Correcting a specific fact or preference without rewriting the whole file
- Updating a section heading or dated entry in place
- Appending to a specific bullet list within a file

## Error handling

If `search` is not found in the target file, the skill returns an error and **leaves the file unchanged** — no silent corruption.

## Parameters

| Parameter | Type   | Required | Description                                   |
| --------- | ------ | -------- | --------------------------------------------- |
| target    | string | yes      | `soul` / `identity` / `user` / `memory`       |
| search    | string | yes      | Exact text to locate (first occurrence)       |
| replace   | string | yes      | Text to substitute in place of the found text |

## Example

```json
{
  "target": "user",
  "search": "Timezone: UTC",
  "replace": "Timezone: Europe/Berlin"
}
```
