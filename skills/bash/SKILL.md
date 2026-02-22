---
name: bash
description: >
  Execute a bash command and return its output. Use this to run shell commands,
  scripts, inspect files, check system state, install packages, or perform any
  system operation. Prefer this over asking the user to run commands manually.
license: Apache-2.0
compatibility: Requires bash
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "false"
  params: '{"command": {"type": "string", "description": "The bash command to execute"}, "working_dir": {"type": "string", "description": "Working directory for the command (default: current dir)"}, "timeout_secs": {"type": "integer", "description": "Timeout in seconds (default: 120)"}}'
---

## Instructions

Execute a bash command as a subprocess and return its stdout/stderr output.

### Parameters

- `command` (string, required): The bash command to execute (run via `bash -c`)
- `working_dir` (string, optional): Working directory for the command
- `timeout_secs` (integer, optional): Timeout in seconds (default: 120)

### Behavior

- Executes via `bash -c` with a configurable timeout (default: 120 seconds)
- Captures and returns both stdout and stderr
- Returns the exit code along with output
- Works in both interactive and non-interactive (autonomous) contexts
- If the command times out: kills the process and reports the timeout

### Output format

```
Exit code: 0
[stdout output here]
```

### Example uses

- `ls -la /some/path` — inspect directory contents
- `cat file.txt` — read a file
- `grep -r "pattern" .` — search in files
- `cargo build` — build a project
- `python script.py` — run a script
