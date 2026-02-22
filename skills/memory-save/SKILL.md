---
name: memory-save
description: >
  Append a timestamped note to today's daily memory log (~/.assistant/memory/YYYY-MM-DD.md).
  Use this to record observations, facts, or context that should be available in future
  sessions. Unlike memory-write (key/value), this appends free-form notes to a daily log.
license: Apache-2.0
compatibility: Requires filesystem access
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "false"
  params: '{"note": {"type": "string", "description": "The note to append to today'\''s daily log"}, "category": {"type": "string", "description": "Optional category label (e.g. task, observation, user-preference)"}}'
---

## Instructions

Append a timestamped note to the daily memory file (`~/.assistant/memory/YYYY-MM-DD.md`).

### Parameters

- `note` (string, required): The text of the note to record
- `category` (string, optional): A category label shown in the note header

### Behavior

- Creates the notes directory and today's file if they don't exist
- Appends `## HH:MM [category]\n<note>` to the file
- Confirms success with the path of the file written

### When to use

- When the user shares important context they want remembered across sessions
- After completing a task you want to log
- When you observe something noteworthy about the user's preferences or workflow

### Example interactions

- "Remember that I'm working on the OAuth refactor" → note: "Working on OAuth refactor", category: "task"
- "Note: user prefers verbose explanations" → note: "User prefers verbose, detailed explanations", category: "user-preference"
