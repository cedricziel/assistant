---
name: soul-update
description: >
  Update one of the assistant's persistent markdown identity files: SOUL.md (personality),
  IDENTITY.md (name and role), USER.md (user profile), or MEMORY.md (long-term memory).
  Supports append (add content) or replace (overwrite entire file) modes.
license: Apache-2.0
compatibility: Requires filesystem access
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "false"
  params: '{"target": {"type": "string", "description": "File to update: soul, identity, user, or memory"}, "content": {"type": "string", "description": "Content to write"}, "mode": {"type": "string", "description": "Write mode: append (default) or replace"}}'
---

## Instructions

Update one of the assistant's persistent memory markdown files.

### Parameters

- `target` (string, required): Which file to update — one of: `soul`, `identity`, `user`, `memory`
- `content` (string, required): The text to write
- `mode` (string, optional): `append` (default) adds content to the end; `replace` overwrites the entire file

### Target files

| target     | File                       | Purpose                                             |
| ---------- | -------------------------- | --------------------------------------------------- |
| `soul`     | `~/.assistant/SOUL.md`     | Core personality, values, and behavioral truths     |
| `identity` | `~/.assistant/IDENTITY.md` | Name, role, goals — the agent's structured identity |
| `user`     | `~/.assistant/USER.md`     | User profile: name, timezone, preferences           |
| `memory`   | `~/.assistant/MEMORY.md`   | Curated long-term memory — important facts          |

### Behavior

- `append` mode: adds `\n<content>` to the end of the file (creates if not exists)
- `replace` mode: completely replaces the file with `content`
- Confirms the update with the path that was written

### When to use

- **First session**: if IDENTITY.md still contains blank placeholder fields, fill them in (use `replace` mode with the completed form)
- When the user tells you their name, timezone, or preferences → update `user`
- When you learn something important to remember permanently → update `memory`
- When the user adjusts your personality or behavior → update `soul`
- After a significant project completes and context should persist → update `memory`
- When MEMORY.md grows stale or cluttered → use `replace` to prune and tidy it

For changing a single field within a file, prefer `memory-patch` (surgical search-and-replace) over `replace` mode here.

### Example interactions

- "My name is Alice and I'm in Berlin timezone" → target: "user", mode: "append", content: "- Name: Alice\n- Timezone: Europe/Berlin"
- "Always use metric units" → target: "soul", mode: "append", content: "\n- Use metric units in all responses"
