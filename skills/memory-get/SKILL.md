---
name: memory-get
description: >
  Read one of the persistent memory files: SOUL.md, IDENTITY.md, USER.md,
  MEMORY.md, or a daily note (memory/YYYY-MM-DD.md). Returns the full file content.
metadata:
  tier: builtin
  params: '{"type":"object","properties":{"target":{"type":"string","description":"Which file to read: soul, identity, user, memory, or notes/YYYY-MM-DD"}},"required":["target"]}'
---
