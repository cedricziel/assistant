---
name: shell-exec
description: >
  Run a shell command and return its output. ONLY use when the user explicitly asks to
  run a command, execute a script, or perform a system operation. Always shows the command
  to the user and requires explicit confirmation before execution. Use with caution.
license: Apache-2.0
compatibility: Requires shell access (bash/sh)
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "true"
  params: '{"command": {"type": "string", "description": "The shell command to execute"}, "working_dir": {"type": "string", "description": "Working directory for the command (default: current dir)"}}'
---

## Instructions

Execute a shell command in a subprocess and return its stdout/stderr output.

### Parameters
- `command` (string, required): The shell command to execute (run via `/bin/sh -c`)
- `working_dir` (string, optional): Working directory for the command

### Behavior
- **ALWAYS** shows the exact command to the user and asks for confirmation before running
- Executes in a subprocess with a 30-second timeout
- Captures and returns both stdout and stderr
- Returns the exit code along with output
- If the command times out: kills the process and reports the timeout

### Safety constraints
- Confirmation is MANDATORY (metadata.confirmation-required = "true")
- This skill is **disabled** for the Signal interface to prevent unattended system access
- Avoids running commands that could cause data loss without extra user confirmation

### Example interactions
- "Run `ls -la` in my home directory" → execute with confirmation
- "Check if docker is running" → `command: "docker ps"`
- "What's the disk usage?" → `command: "df -h"`

### Output format
Show exit code on first line, then stdout, then stderr (if any):
```
Exit code: 0
[stdout output here]
```
