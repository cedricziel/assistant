---
name: heartbeat-update
description: >
  Write new contents to ~/.assistant/HEARTBEAT.md — the prompt the scheduler runs
  automatically every 30 minutes. Replaces the entire file.
license: Apache-2.0
compatibility: Requires filesystem access
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "false"
  params: '{"content": {"type": "string", "description": "Full Markdown content to write to HEARTBEAT.md. This becomes the prompt run every 30 minutes."}}'
---

## Instructions

Replace the entire contents of `~/.assistant/HEARTBEAT.md` with the provided text.

The scheduler reads this file every 30 minutes and runs it as a ReAct prompt,
giving the assistant a chance to check conditions, write memory entries, or take
autonomous actions without user input.

### Parameters

- `content` (string, required): The full Markdown text to write. This becomes the
  prompt that runs on every heartbeat tick.

### Format guidelines

The file should be written as a set of instructions the assistant can act on
autonomously. For example:

```markdown
Check the following every 30 minutes:

1. If no user message has been sent today, write a friendly reminder in MEMORY.md
   noting that the user may need to be re-engaged.
2. If it is between 08:00 and 09:00 local time, write today's date and a blank
   agenda entry in memory.
3. If any scheduled task failed in the last run, summarise the error in MEMORY.md.
```

### When to use

- When the user says "run X automatically every 30 minutes"
- When the user wants to set up a recurring autonomous check or action
- To disable the heartbeat: call with `content: ""` (empty string)

### Example

```json
{
  "content": "Check if today's date entry exists in MEMORY.md. If not, create one."
}
```
