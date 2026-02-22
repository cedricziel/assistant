---
name: file-glob
description: >
  Find files matching a glob pattern on the filesystem. Returns a newline-
  separated list of matching paths, up to a configurable limit.
license: Apache-2.0
compatibility: Requires filesystem access
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: '{"pattern": {"type": "string", "description": "Glob pattern, e.g. \"**/*.rs\" or \"~/notes/*.md\""}, "limit": {"type": "number", "description": "Maximum number of results to return (default: 200)"}}'
---

## Instructions

Find files and directories on disk that match a glob pattern.

### Parameters

- `pattern` (string, required): Glob pattern. Supports `*`, `**`, `?`, and `[ranges]`. Supports `~` expansion.
- `limit` (number, optional): Maximum results to return. Defaults to 200.

### Behavior

- Returns a newline-separated list of matching paths
- Results are in filesystem order (not sorted)
- If more matches exist than `limit`, a truncation note is appended
- Returns an error if the pattern is syntactically invalid

### When to use

- Discovering all files of a type in a directory tree
- Finding config files, logs, or notes matching a pattern
- Before reading or editing multiple files

### Example interactions

- "Find all Rust files in my project" → pattern: "~/project/\*_/_.rs"
- "List all markdown notes" → pattern: "~/notes/\*.md"
- "Find JSON configs" → pattern: "/etc/\*_/_.json", limit: 50
