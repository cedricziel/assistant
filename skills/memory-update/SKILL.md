---
name: memory-update
description: >
  Update one of the assistant's persistent markdown memory files: SOUL.md (personality),
  IDENTITY.md (name and role), USER.md (user profile), or MEMORY.md (long-term memory).
  These files are loaded into every session's system prompt, so changes take effect immediately.
  Use this whenever you learn something about the user or need to record a lasting fact.
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
| `user`     | `~/.assistant/USER.md`     | User profile: name, timezone, language, preferences |
| `memory`   | `~/.assistant/MEMORY.md`   | Curated long-term memory — important facts          |

### Behavior

- `append` mode: adds `\n<content>` to the end of the file (creates if not exists)
- `replace` mode: completely replaces the file with `content`
- Confirms the update with the path that was written

### When to use

- When the user tells you their name, language, timezone, or preferences → target: `user`
- When you learn something important to remember permanently → target: `memory`
- When the user adjusts your personality or behavior → target: `soul`
- When IDENTITY.md still has blank placeholder fields, fill them in → target: `identity`, mode: `replace`
- When MEMORY.md grows stale or cluttered → use `replace` to prune and tidy it

For changing a single field within a file, prefer `memory-patch` (surgical search-and-replace) over `replace` mode here.

### Example interactions

- "My name is Alice and I'm in Berlin timezone" → target: `user`, mode: `append`, content: `"- Name: Alice\n- Timezone: Europe/Berlin"`
- "I like to speak German" → target: `user`, mode: `append`, content: `"- Language: German (preferred)"`
- "Always use metric units" → target: `soul`, mode: `append`, content: `"\n- Use metric units in all responses"`
