---
name: heartbeat-read
description: >
  Read the current contents of ~/.assistant/HEARTBEAT.md — the file that controls
  what the assistant checks or does automatically every 30 minutes.
license: Apache-2.0
compatibility: Requires filesystem access
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: "{}"
---

## Instructions

Read `~/.assistant/HEARTBEAT.md` and return its current contents.

The heartbeat file is run by the background scheduler every 30 minutes as a ReAct
prompt. It lets the assistant act autonomously at regular intervals — checking
conditions, writing notes, sending reminders, etc.

### When to use

- Before editing the heartbeat file, to see what's currently there
- When the user asks "what is my heartbeat set to?" or "what runs automatically?"

### Output

Returns the file's full contents with the path shown, or a message explaining the
file does not yet exist if it has never been written.
