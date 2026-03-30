---
name: claude-code-agent
description: >
  Run tasks on the local machine using Claude Code CLI as a background agent.
  Use this when the user wants to execute code, edit files, run shell commands,
  build projects, analyse repositories, or do any agentic work on the device —
  especially multi-step tasks that benefit from Claude Code's tool-use loop.
  Supports fire-and-forget async jobs (non-blocking, via the native process tool)
  as well as quick blocking one-shot tasks and follow-up questions in the same session.
license: MIT
compatibility: "Requires: claude CLI (claude --version)"
---

# Claude Code Agent Skill

Run an agentic task on the local device via the `claude` CLI (Claude Code).

## How to choose between blocking and async mode

| Use **blocking** (`async: false`) | Use **async** (`async: true`)           |
| --------------------------------- | --------------------------------------- |
| Quick one-shot tasks (<30s)       | Long builds, refactors, multi-file work |
| Single follow-up questions        | Parallel agents / multiple worktrees    |
| Simple shell automation           | Tasks that may take minutes             |

**Default heuristic:** if the task sounds like it will take more than ~20 seconds (build, analyse a big repo, write many files), use async mode.

---

## Mode A — Blocking (quick tasks)

```bash
cd "${workdir:-$HOME}" && \
claude \
  --print \
  --output-format json \
  --model "${model:-sonnet}" \
  --max-budget-usd "${budget_usd:-2.0}" \
  ${session_id:+--resume "$session_id"} \
  ${skip_permissions:+--dangerously-skip-permissions} \
  ${worktree:+-w "$worktree"} \
  --allowedTools "Bash,Edit,Read,Write,Glob,Grep,LS,Task,TodoRead,TodoWrite,WebFetch,WebSearch" \
  "$prompt"
```

Parse the JSON result:

- `result` — final text answer / summary
- `session_id` — save this to resume later
- `is_error` / `stop_reason` — detect failures
- `total_cost_usd` — report cost to the user

---

## Mode B — Async (long-running tasks, non-blocking)

Async mode always uses `--dangerously-skip-permissions` because the process runs
without a TTY and cannot respond to interactive permission prompts. The
`skip_permissions` parameter is only relevant for blocking mode (Mode A).

Uses the native `process` tool — no tmux required.

### Step 1: Write the prompt to a temp file

Use the `bash` tool to write the prompt safely (avoids all shell escaping/injection
issues with multi-line or quote-heavy prompts):

```bash
PROMPT_FILE="/tmp/cca-$(date +%s).prompt"
cat > "$PROMPT_FILE" << 'PROMPT_EOF'
${prompt}
PROMPT_EOF
echo "$PROMPT_FILE"
```

### Step 2: Start the agent

Use `process action:start` — returns a `session_id` immediately:

```
process action:start
  command: "cat \"$PROMPT_FILE\" | claude --print --output-format json --model ${model:-sonnet} --max-budget-usd ${budget_usd:-2.0} --dangerously-skip-permissions ${session_id:+--resume \"$session_id\"} ${worktree:+-w \"$worktree\"} --allowedTools \"Bash,Edit,Read,Write,Glob,Grep,LS,Task,TodoRead,TodoWrite,WebFetch,WebSearch\""
  workdir: "${workdir:-$HOME}"
```

→ Save the returned `session_id`. Report it to the user immediately. Do NOT wait.

### Step 3: Poll for completion

When the user asks for a status update (or after a reasonable wait), check:

```
process action:poll
  session_id: "<session_id from step 2>"
```

- `running: true` → still working, check again later
- `running: false` + `exit_code: 0` → done successfully → proceed to step 4
- `running: false` + `exit_code: <non-zero>` → failed → fetch logs and report error

### Step 4: Retrieve output

```
process action:log
  session_id: "<session_id>"
  lines: 500
```

Parse the `stdout` field as JSON (same fields as blocking mode):

- `result` — final text answer / summary
- `session_id` — save this to resume the Claude session later
- `is_error` / `stop_reason` — detect failures
- `total_cost_usd` — report cost to the user

If `is_error` is true, also check `stderr` from the log output for diagnostics.

### Step 5: Cleanup

```
process action:kill
  session_id: "<session_id>"
```

Then remove the temp prompt file:

```bash
rm -f "$PROMPT_FILE"
```

---

## Parallel worktrees (multiple agents at once)

Write each prompt to its own file, then start one `process action:start` per agent:

```bash
# Write prompt files
echo "Fix issue #42: login button broken" > /tmp/cca-issue-42.prompt
echo "Fix issue #99: avatar upload fails" > /tmp/cca-issue-99.prompt
```

```
process action:start
  command: "cat /tmp/cca-issue-42.prompt | claude -w fix-issue-42 --print --output-format json --dangerously-skip-permissions --allowedTools \"Bash,Edit,Read,Write,Glob,Grep,LS,Task,TodoRead,TodoWrite,WebFetch,WebSearch\""
  workdir: "~/code/myproject"
```

```
process action:start
  command: "cat /tmp/cca-issue-99.prompt | claude -w fix-issue-99 --print --output-format json --dangerously-skip-permissions --allowedTools \"Bash,Edit,Read,Write,Glob,Grep,LS,Task,TodoRead,TodoWrite,WebFetch,WebSearch\""
  workdir: "~/code/myproject"
```

Poll each `session_id` independently. Use `process action:list` to see all active agents at once.

---

## Guidelines

- Default to `--model sonnet` (faster, cheaper); use `opus` only if the user asks or the task is very complex.
- Keep `--max-budget-usd` at 2.0 unless the user explicitly requests more.
- **Always write the prompt to a file and pipe it via stdin** — never interpolate `$prompt` directly into the command string.
- **Async mode always skips permissions** — no TTY available for background processes.
- Always report the `process session_id` and Claude `session_id` back to the user so they can follow up.
- If `is_error` is true, show the error and suggest a fix.
- Clean up prompt temp files after results are collected.
- The process tool buffers up to 1000 lines of output — request `lines: 500` or more when fetching Claude's JSON result to avoid truncation.

---

## Example invocations

**Quick one-shot (blocking):**

```yaml
prompt: "What's the largest file in ~/code/assistant?"
workdir: "~/code/assistant"
async: false
```

**Long build (async):**

```yaml
prompt: "Run cargo build --release and fix any errors"
workdir: "~/code/assistant"
async: true
```

**Resume a Claude session:**

```yaml
prompt: "Now also add tests for the function you wrote"
session_id: "3153e086-80f2-4937-afa3-80a922ef1bdc"
async: false
```

**Poll async agent:**

```yaml
process_session_id: "<process tool session_id>"
prompt: "(check status)"
```

**Parallel worktree agents:**

```yaml
prompt: "Fix issue #42: login button broken"
workdir: "~/code/myproject"
worktree: "fix-issue-42"
async: true
```
