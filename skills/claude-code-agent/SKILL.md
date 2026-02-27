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

Async mode always uses `--dangerously-skip-permissions` because there is no TTY
for interactive prompts inside a detached tmux session. The `skip_permissions`
parameter is only relevant for blocking mode (Mode A).

### Step 1: Start the agent

Write the prompt to a file and pass it via stdin — this avoids all shell
escaping/injection issues with multi-line or quote-heavy prompts:

```bash
SESSION="cca-$(date +%s)"
OUTFILE="/tmp/${SESSION}.json"
PROMPT_FILE="/tmp/${SESSION}.prompt"
WORKDIR="${workdir:-$HOME}"

mkdir -p /tmp/cca-sessions
echo "$SESSION" > /tmp/cca-sessions/latest

# Write prompt to file — no escaping needed, handles any content safely
cat > "$PROMPT_FILE" << 'PROMPT_EOF'
${prompt}
PROMPT_EOF

# Build script — prompt fed via stdin to avoid argument length limits
cat > "/tmp/${SESSION}.sh" << SCRIPT_EOF
#!/usr/bin/env bash
cd $(printf '%q' "$WORKDIR")
cat $(printf '%q' "$PROMPT_FILE") | claude \\
  --print \\
  --output-format json \\
  --model "${model:-sonnet}" \\
  --max-budget-usd "${budget_usd:-2.0}" \\
  --dangerously-skip-permissions \\
  ${session_id:+--resume "${session_id}"} \\
  ${worktree:+-w "${worktree}"} \\
  --allowedTools "Bash,Edit,Read,Write,Glob,Grep,LS,Task,TodoRead,TodoWrite,WebFetch,WebSearch" \\
  > $(printf '%q' "$OUTFILE") 2>&1
echo '___CLAUDE_DONE___'
SCRIPT_EOF
chmod +x "/tmp/${SESSION}.sh"

tmux new-session -d -s "$SESSION" -x 220 -y 50
tmux send-keys -t "$SESSION" "bash /tmp/${SESSION}.sh" Enter

echo "✅ Agent started async"
echo "tmux_session: $SESSION"
echo "output_file:  $OUTFILE"
```

→ Report `tmux_session` and `output_file` to the user immediately. Do NOT wait.

### Step 2: Poll for completion

When the user asks for a status update (or after a reasonable wait), check:

```bash
SESSION="${tmux_session}"
OUTFILE="/tmp/${SESSION}.json"

if tmux capture-pane -t "$SESSION" -p -S -5 2>/dev/null | grep -q "___CLAUDE_DONE___"; then
  echo "✅ Done"
  cat "$OUTFILE"
else
  echo "⏳ Still running..."
  tmux capture-pane -t "$SESSION" -p -S -15
fi
```

Parse `$OUTFILE` as JSON once done (same fields as blocking mode).

### Step 3: Cleanup (after reading results)

```bash
tmux kill-session -t "$SESSION" 2>/dev/null
rm -f "/tmp/${SESSION}.json" "/tmp/${SESSION}.sh" "/tmp/${SESSION}.prompt"
```

---

## Parallel worktrees (multiple agents at once)

```bash
REPO_DIR=~/code/myproject

for ISSUE in 42 99; do
  SESSION="cca-issue-${ISSUE}-$(date +%s)"
  OUTFILE="/tmp/${SESSION}.json"
  PROMPT_FILE="/tmp/${SESSION}.prompt"

  echo "Fix issue #${ISSUE}: <description>" > "$PROMPT_FILE"

  cat > "/tmp/${SESSION}.sh" << SCRIPT_EOF
#!/usr/bin/env bash
cd $(printf '%q' "$REPO_DIR")
cat $(printf '%q' "$PROMPT_FILE") | claude -w fix-issue-${ISSUE} \\
  --print --output-format json --dangerously-skip-permissions \\
  > $(printf '%q' "$OUTFILE") 2>&1
echo '___CLAUDE_DONE___'
SCRIPT_EOF
  chmod +x "/tmp/${SESSION}.sh"
  tmux new-session -d -s "$SESSION" -x 220 -y 50
  tmux send-keys -t "$SESSION" "bash /tmp/${SESSION}.sh" Enter
  echo "Started: $SESSION"
done
```

---

## Guidelines

- Default to `--model sonnet` (faster, cheaper); use `opus` only if the user asks or the task is very complex.
- Keep `--max-budget-usd` at 2.0 unless the user explicitly requests more.
- **Always write the prompt to a file and pipe it via stdin** — never interpolate `$prompt` directly into shell strings.
- **Async mode always skips permissions** — no TTY available in detached tmux sessions.
- Always report `tmux_session` and `session_id` back to the user so they can follow up.
- If `is_error` is true, show the error and suggest a fix.
- Clean up all temp files (`.json`, `.sh`, `.prompt`) after results are collected.

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

**Poll async session:**
```yaml
tmux_session: "cca-1772179451"
prompt: "(check status)"
```

**Parallel worktree agents:**
```yaml
prompt: "Fix issue #42: login button broken"
workdir: "~/code/myproject"
worktree: "fix-issue-42"
async: true
```
