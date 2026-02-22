---
name: file-write
description: >
  Write content to any file on disk, creating it (and parent directories) if
  needed. Completely replaces the file's existing content.
license: Apache-2.0
compatibility: Requires filesystem access
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "false"
  params: '{"path": {"type": "string", "description": "Absolute or ~-relative path to the file to write"}, "content": {"type": "string", "description": "Content to write to the file"}}'
---

## Instructions

Write content to a file, replacing any existing content. Parent directories are
created automatically if they do not exist.

### Parameters

- `path` (string, required): Path to the file. Supports `~` expansion.
- `content` (string, required): The full content to write.

### Behavior

- Creates the file if it does not exist
- Creates parent directories as needed
- Completely replaces existing content (use `file-edit` for surgical edits)
- Returns the number of bytes written and the resolved path

### When to use

- Creating new files
- Replacing the full content of an existing file
- Writing configuration, notes, or generated output to disk

### Example interactions

- "Write a TODO list to ~/notes/todo.md" → path: "~/notes/todo.md", content: "# TODO\n..."
- "Save this JSON to /tmp/output.json" → path: "/tmp/output.json", content: "..."
