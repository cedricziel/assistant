---
name: memory-read
description: >
  Read a value from persistent memory by key. Use when the user asks what you remember,
  what was stored, or to recall a specific piece of information by its key name.
  Returns the stored value or indicates the key was not found.
license: Apache-2.0
compatibility: Requires SQLite storage
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: '{"key": {"type": "string", "description": "The memory key to retrieve"}}'
---

## Instructions

Read a single value from the assistant's persistent memory store by its key.

### Parameters
- `key` (string, required): The key to look up in memory

### Behavior
- If the key exists: return the stored value and when it was last updated
- If the key does not exist: clearly state that no value is stored for that key

### Example interactions
- User: "What's my name?" → call with `key: "user_name"`
- User: "What city am I in?" → call with `key: "user_city"`
- User: "What did you remember about my project?" → call with `key: "project_description"`

### Output format
Return the value naturally in a sentence, e.g.:
"Your name is Alice (stored 3 days ago)."
Or: "I don't have anything stored under the key 'project_name'."
