---
name: list-skills
description: >
  List all registered skills with their names, descriptions, tiers, and source directories.
  Use when the user asks what you can do, what skills or capabilities are available,
  or wants to browse the skill library.
license: Apache-2.0
compatibility: Requires skill registry
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: '{"filter": {"type": "string", "description": "Optional filter to match against skill names or descriptions"}}'
---

## Instructions

List all skills currently registered in the skill registry.

### Parameters
- `filter` (string, optional): If provided, only show skills whose name or description contains this string (case-insensitive)

### Behavior
- Retrieves all enabled skills from the SkillRegistry
- Shows for each skill:
  - Name (kebab-case)
  - One-line description
  - Execution tier (builtin / script / wasm / prompt)
  - Source directory (abbreviated to show ~/ or project paths)

### Example interactions
- "What can you do?" → list all skills
- "Do you have any memory skills?" → filter: "memory"
- "/skills" → list all skills (CLI command also shows this)

### Output format
Present as a formatted table or list:
```
Available skills (8):
  memory-read     Read from persistent memory            [builtin]
  memory-write    Store to persistent memory             [builtin]
  memory-search   Search all stored memories             [builtin]
  web-fetch       Fetch a URL and return page text       [builtin]
  shell-exec      Run a shell command (with confirmation) [builtin]
  list-skills     List all registered skills             [builtin]
  self-analyze    Analyze traces, propose improvements   [builtin]
  schedule-task   Register a recurring cron task         [builtin]
```
