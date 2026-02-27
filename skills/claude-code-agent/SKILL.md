---
name: claude-code-agent
description: >
  Run tasks on the local machine using Claude Code CLI as a background agent.
  Use this when the user wants to execute code, edit files, run shell commands,
  build projects, analyse repositories, or do any agentic work on the device —
  especially multi-step tasks that benefit from Claude Code's tool-use loop.
  Supports fire-and-forget async jobs (tmux-based, non-blocking) as well as
  quick blocking one-shot tasks and follow-up questions in the same session.
license: MIT
metadata:
  tier: bash
  mutating: "true"
  confirmation-required: "false"
  params: >
    {
      "prompt":        {"type": "string",  "description": "What Claude Code should do"},
      "workdir":       {"type": "string",  "description": "Working directory (default: ~)", "default": "~"},
      "session_id":    {"type": "string",  "description": "Resume an existing Claude session (UUID)", "default": ""},
      "tmux_session":  {"type": "string",  "description": "Poll/resume an async tmux session by name", "default": ""},
      "model":         {"type": "string",  "description": "Model alias: sonnet or opus (default: sonnet)", "default": "sonnet"},
      "budget_usd":    {"type": "number",  "description": "Max USD to spend (default: 2.0)", "default": 2.0},
      "skip_permissions": {"type": "boolean", "description": "Bypass all permission checks — only for trusted tasks (default: false)", "default": false},
      "async":         {"type": "boolean", "description": "Run non-blocking in tmux (default: false for short tasks, true for long ones)", "default": false},
      "worktree":      {"type": "string",  "description": "Create a git worktree with this name (requires git repo in workdir)", "default": ""}
    }
---

# Claude Code Agent Skill

Run an agentic task on the local device via the `claude` CLI (Claude Code).

## How to choose between blocking and async mode

| Use **blocking** (`async: false`) | Use **async** (`async: true`) |
|---|---|
| Quick one-shot tasks (<30s) | Long builds, refactors, multi-file work |
| Single follow-up questions | Parallel agents / multiple worktrees |
| Simple shell automation | Tasks that may take minutes |

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

### Step 1: Start the agent

```bash
SESSION="cca-$(date +%s)"
OUTFILE="/tmp/${SESSION}.json"
WORKDIR="${workdir:-$HOME}"

mkdir -p /tmp/cca-sessions
echo "$SESSION" > /tmp/cca-sessions/latest

tmux new-session -d -s "$SESSION" -x 220 -y 50

tmux send-keys -t "$SESSION" \
  "cd '$WORKDIR' && claude \
    --print \
    --output-format json \
    --model '${model:-sonnet}' \
    --max-budget-usd '${budget_usd:-2.0}' \
    ${session_id:+--resume '$session_id'} \
    --dangerously-skip-permissions \
    ${worktree:+-w '$worktree'} \
    --allowedTools 'Bash,Edit,Read,Write,Glob,Grep,LS,Task,TodoRead,TodoWrite,WebFetch,WebSearch' \
    '$prompt' > '$OUTFILE' 2>&1; echo '___CLAUDE_DONE___'" \
  Enter

echo "✅ Agent started async"
echo "tmux_session: $SESSION"
echo "output_file:  $OUTFILE"
echo "Check progress: tmux capture-pane -t $SESSION -p -S -20"
```

→ Report `tmux_session` and `output_file` to the user immediately. Do NOT wait.

### Step 2: Poll for completion

When the user asks for a status update (or after a reasonable wait), check:

```bash
SESSION="${tmux_session}"
OUTFILE="/tmp/${SESSION}.json"

# Is it done?
if tmux capture-pane -t "$SESSION" -p -S -5 2>/dev/null | grep -q "___CLAUDE_DONE___"; then
  echo "✅ Done"
  cat "$OUTFILE"
else
  echo "⏳ Still running..."
  # Show last few lines of live output
  tmux capture-pane -t "$SESSION" -p -S -15
fi
```

Parse `$OUTFILE` as JSON once done (same fields as blocking mode).

### Step 3: Cleanup (after reading results)

```bash
tmux kill-session -t "$SESSION" 2>/dev/null
rm -f "/tmp/${SESSION}.json"
```

---

## Parallel worktrees (multiple agents at once)

```bash
# Fix two issues in parallel, each in its own git worktree
REPO_DIR=~/code/myproject

SESSION_A="cca-issue-42-$(date +%s)"
SESSION_B="cca-issue-99-$(date +%s)"

tmux new-session -d -s "$SESSION_A" -x 220 -y 50
tmux new-session -d -s "$SESSION_B" -x 220 -y 50

tmux send-keys -t "$SESSION_A" \
  "cd '$REPO_DIR' && claude -w fix-issue-42 --print --output-format json --dangerously-skip-permissions \
   'Fix issue #42: <description>' > /tmp/${SESSION_A}.json 2>&1; echo '___CLAUDE_DONE___'" Enter

tmux send-keys -t "$SESSION_B" \
  "cd '$REPO_DIR' && claude -w fix-issue-99 --print --output-format json --dangerously-skip-permissions \
   'Fix issue #99: <description>' > /tmp/${SESSION_B}.json 2>&1; echo '___CLAUDE_DONE___'" Enter

echo "Both agents running:"
echo "  Session A: $SESSION_A"
echo "  Session B: $SESSION_B"
```

---

## Guidelines

- Default to `--model sonnet` (faster, cheaper); use `opus` only if the user asks or the task is very complex.
- Keep `--max-budget-usd` at 2.0 unless the user explicitly requests more.
- **Always use `--dangerously-skip-permissions` in async/tmux mode** (no TTY for interactive prompts).
- Always report `tmux_session` and `session_id` back to the user so they can follow up.
- If `is_error` is true, show the error and suggest a fix.
- Clean up tmux sessions after results are collected.

---

## Example invocations

**Quick one-shot (blocking):**
```
prompt: "What's the largest file in ~/code/assistant?"
workdir: "~/code/assistant"
async: false
```

**Long build (async):**
```
prompt: "Run cargo build --release and fix any errors"
workdir: "~/code/assistant"
async: true
skip_permissions: true
```

**Resume a Claude session:**
```
prompt: "Now also add tests for the function you wrote"
session_id: "3153e086-80f2-4937-afa3-80a922ef1bdc"
async: false
```

**Poll async session:**
```
tmux_session: "cca-1772179451"
prompt: "(check status)"
```

**Parallel worktree agents:**
```
prompt: "Fix issue #42: login button broken"
workdir: "~/code/myproject"
worktree: "fix-issue-42"
async: true
```
