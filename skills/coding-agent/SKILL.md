---
name: coding-agent
description: >
  Run coding agents (Claude Code, Codex, OpenCode, or others) as background
  processes for programmatic control. Use when you need non-blocking execution,
  parallel agents, PR reviews, or long-running coding tasks. Prefer this over
  direct bash for any task that takes more than ~20 seconds.
license: MIT
metadata:
  tier: bash
  mutating: "true"
  confirmation-required: "false"

---

# Coding Agent (background-first)

Use the **`process` tool** for non-interactive coding work. This gives you
non-blocking execution, progress monitoring, stdin/stdout access, and clean
lifecycle management.

## The Pattern: workdir + process:start

```bash
# Start agent in a focused directory ("little box" — only sees relevant files)
process action:start command:"claude --dangerously-skip-permissions --print 'Your task'" workdir:~/project/folder
# Returns session_id immediately

# Monitor output
process action:log session_id:XXX lines:50

# Check if done
process action:poll session_id:XXX

# Send input (if agent asks a question)
process action:write session_id:XXX data:"y\n"

# Kill if needed
process action:kill session_id:XXX

# See all running agents
process action:list
```

**Why workdir matters:** Agent wakes up in a focused directory — doesn't wander
off reading unrelated files. Use `mktemp -d` for scratch/chat work.

---

## Claude Code

```bash
# Non-interactive (recommended for background tasks)
process action:start \
  command:"claude --print --output-format json --dangerously-skip-permissions 'Your task'" \
  workdir:~/project

# With worktree isolation
process action:start \
  command:"claude -w fix-issue-42 --print --output-format json --dangerously-skip-permissions 'Fix issue #42'" \
  workdir:~/project

# Parse JSON output from action:log once done
# { "result": "...", "session_id": "...", "total_cost_usd": ... }
```

---

## Codex CLI

```bash
# --full-auto: sandboxed but auto-approves in workspace
process action:start command:"codex exec --full-auto \"Build a snake game with dark theme\"" workdir:~/project

# --yolo: NO sandbox, NO approvals (fastest, most dangerous)
process action:start command:"codex --yolo \"Build a snake game with dark theme\"" workdir:~/project
```

---

## OpenCode

```bash
process action:start command:"opencode run 'Your task'" workdir:~/project
```

---

## PR Reviews

**⚠️ Never review PRs in the live assistant repo — clone to /tmp or use git worktree!**

```bash
# Option 1: Review in the actual project repo
process action:start command:"claude --print --dangerously-skip-permissions 'Review PR #42: git diff origin/main...origin/pr/42'" workdir:~/project

# Option 2: Clone to temp folder (safe for any repo)
REVIEW_DIR=$(mktemp -d)
git clone https://github.com/user/repo.git $REVIEW_DIR
cd $REVIEW_DIR && gh pr checkout 42
process action:start command:"claude --print --dangerously-skip-permissions 'Review this PR against main'" workdir:$REVIEW_DIR

# Option 3: Git worktree (keeps main intact)
git worktree add /tmp/pr-42-review pr-42-branch
process action:start command:"claude --print --dangerously-skip-permissions 'Review this PR'" workdir:/tmp/pr-42-review
```

### Batch PR Reviews (parallel army!)

```bash
# Fetch all PR refs
git fetch origin '+refs/pull/*/head:refs/remotes/origin/pr/*'

# Launch one agent per PR
process action:start command:"claude --print --dangerously-skip-permissions 'Review PR #86: git diff origin/main...origin/pr/86'" workdir:~/project
process action:start command:"claude --print --dangerously-skip-permissions 'Review PR #87: git diff origin/main...origin/pr/87'" workdir:~/project
process action:start command:"claude --print --dangerously-skip-permissions 'Review PR #95: git diff origin/main...origin/pr/95'" workdir:~/project

# Monitor all
process action:list

# Get result and post to GitHub
process action:log session_id:XXX
gh pr comment 86 --body "<review content>"
```

---

## Parallel Issue Fixing with git worktrees

```bash
# Create isolated worktrees per issue
git worktree add -b fix/issue-78 /tmp/issue-78 main
git worktree add -b fix/issue-99 /tmp/issue-99 main

# Launch agents in parallel
process action:start command:"claude --print --dangerously-skip-permissions 'Fix issue #78: <description>. Commit when done.'" workdir:/tmp/issue-78
process action:start command:"claude --print --dangerously-skip-permissions 'Fix issue #99: <description>. Commit when done.'" workdir:/tmp/issue-99

# Poll both
process action:poll session_id:SESSION_A
process action:poll session_id:SESSION_B

# Create PRs after completion
cd /tmp/issue-78 && git push -u origin fix/issue-78
gh pr create --head fix/issue-78 --title "fix: ..." --body "..."

# Cleanup
git worktree remove /tmp/issue-78
git worktree remove /tmp/issue-99
```

**Why worktrees?** Each agent works in its own isolated branch — no conflicts,
5+ parallel fixes possible.

---

## PR Template (The Razor Standard)

When submitting PRs, use this format:

````markdown
## Original Prompt
[Exact request/problem statement]

## What this does
[High-level description]

**Features:**
- [Key feature 1]
- [Key feature 2]

## Feature intent (maintainer-friendly)
[Why useful, how it fits, workflows it enables]

## Prompt history (timestamped)
- YYYY-MM-DD HH:MM UTC: [Step 1]
- YYYY-MM-DD HH:MM UTC: [Step 2]

## How I tested
1. [Test step] - Output: `[result]`
2. [Test step] - Result: [result]

## Implementation details
**New files:** `path/file.rs` — [description]
**Modified:** `path/file.rs` — [change]
````

---

## ⚠️ Rules

1. **Respect tool choice** — if user asks for Codex, use Codex. Never substitute.
2. **Be patient** — don't kill sessions just because they're slow.
3. **Monitor with process:log** — check progress without interfering.
4. **workdir isolation** — always set workdir to the relevant project directory.
5. **Parallel is fine** — run many agents at once for batch work.
6. **Never work directly in the live assistant directory** — clone to /tmp or use git worktree.
