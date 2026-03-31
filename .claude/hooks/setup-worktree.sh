#!/usr/bin/env bash
# WorktreeCreate hook: create the worktree and do post-creation setup.
# Claude Code passes JSON via stdin with: session_id, cwd, hook_event_name, name
# The hook must create the worktree and print its path to stdout.

set -euo pipefail

INPUT=$(cat)
NAME=$(echo "$INPUT" | jq -r '.name')
ORIGINAL_DIR=$(echo "$INPUT" | jq -r '.cwd')

WORKTREE_PATH="$ORIGINAL_DIR/.claude/worktrees/$NAME"
BRANCH_NAME="worktree-${NAME}"

# ── 1. Create the git worktree ───────────────────────────────────────────────
if [[ ! -d "$WORKTREE_PATH" ]]; then
    if git -C "$ORIGINAL_DIR" show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
        git -C "$ORIGINAL_DIR" worktree add "$WORKTREE_PATH" "$BRANCH_NAME" >&2
    else
        git -C "$ORIGINAL_DIR" worktree add -b "$BRANCH_NAME" "$WORKTREE_PATH" HEAD >&2
    fi
fi

# ── Resolve the worktree's git dir (e.g. .git/worktrees/critique) ────────────
GIT_DIR=$(git -C "$WORKTREE_PATH" rev-parse --git-dir)
mkdir -p "$GIT_DIR/info"

# ── 2. Copy vendored JS assets from original repo ───────────────────────────
VENDOR_SRC="$ORIGINAL_DIR/crates/web-ui/src/static_assets/vendor"
VENDOR_DST="$WORKTREE_PATH/crates/web-ui/src/static_assets/vendor"

if [[ -d "$VENDOR_SRC" ]]; then
    mkdir -p "$(dirname "$VENDOR_DST")"
    cp -r "$VENDOR_SRC" "$VENDOR_DST"
    echo "Copied vendor dir" >&2
fi

# ── 3. Share Cargo target directory with the original repo ───────────────────
mkdir -p "$WORKTREE_PATH/.cargo"
CARGO_CONFIG="$WORKTREE_PATH/.cargo/config.toml"

if [[ -f "$CARGO_CONFIG" ]]; then
    if ! grep -q '\[build\]' "$CARGO_CONFIG"; then
        printf '\n[build]\ntarget-dir = "%s/target"\n' "$ORIGINAL_DIR" >> "$CARGO_CONFIG"
    fi
else
    printf '[build]\ntarget-dir = "%s/target"\n' "$ORIGINAL_DIR" > "$CARGO_CONFIG"
fi

# Mark .cargo/config.toml as intentionally modified so it never shows dirty
git -C "$WORKTREE_PATH" update-index --skip-worktree .cargo/config.toml 2>/dev/null || true

echo "Worktree ready: $WORKTREE_PATH" >&2
echo "$WORKTREE_PATH"
