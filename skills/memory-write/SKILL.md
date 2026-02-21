---
name: memory-write
description: >
  Store a key/value pair in persistent memory. Use when the user asks you to remember
  something, note something down, or save a piece of information for later. The value
  persists across conversations until explicitly overwritten or deleted.
license: Apache-2.0
compatibility: Requires SQLite storage
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "false"
  params: '{"key": {"type": "string", "description": "The memory key (snake_case recommended)"}, "value": {"type": "string", "description": "The value to store"}}'
---

## Instructions

Store a key/value pair in the assistant's persistent memory.

### Parameters
- `key` (string, required): The key to store the value under (use snake_case, e.g. `user_name`, `project_goal`)
- `value` (string, required): The value to store

### Behavior
- Overwrites any existing value for the same key
- Confirms the write to the user

### Key naming guidelines
- Use descriptive snake_case keys: `user_name`, `preferred_language`, `project_deadline`
- Group related info: `project_name`, `project_stack`, `project_status`
- For the user's personal info: prefix with `user_` (e.g. `user_name`, `user_timezone`)

### Example interactions
- "Remember my name is Alice" → `key: "user_name"`, `value: "Alice"`
- "Note that I prefer Python over JavaScript" → `key: "preferred_language"`, `value: "Python"`
- "Save that the project deadline is March 15" → `key: "project_deadline"`, `value: "March 15"`

### Output format
Confirm the write concisely: "Got it — I'll remember that your name is Alice."
