---
name: file-read
description: >
  Read the contents of any file on disk. Returns the file content as text,
  optionally truncated to a character limit.
license: Apache-2.0
compatibility: Requires filesystem access
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: '{"path": {"type": "string", "description": "Absolute or ~-relative path to the file to read"}, "limit": {"type": "number", "description": "Maximum characters to return (default: 8000)"}}'
---

## Instructions

Read the contents of a file from disk and return them as text.

### Parameters

- `path` (string, required): Path to the file. Supports `~` expansion for the home directory.
- `limit` (number, optional): Maximum characters to return. Defaults to 8000. Use a larger value for bigger files.

### Behavior

- Returns the file content prefixed with `File: <path>`
- If the content exceeds `limit`, it is truncated with a `[Content truncated at N characters]` marker
- Returns an error if the file does not exist or cannot be read

### When to use

- When you need to read configuration files, source code, notes, or any text file
- When the user asks you to inspect or summarise a file
- Before editing a file (to understand its current contents)

### Example interactions

- "Read my ~/.assistant/MEMORY.md" → path: "~/.assistant/MEMORY.md"
- "Show me the first 500 chars of /etc/hosts" → path: "/etc/hosts", limit: 500
