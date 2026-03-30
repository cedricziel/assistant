#!/usr/bin/env bash
# WorktreeCreate hook: vendor web-ui assets and point Cargo at the shared target dir.
#
# - Runs make vendor so the worktree has all vendored JS assets without re-downloading.
# - Writes .cargo/config.toml in the worktree pointing build.target-dir at the
#   original repo's target/ so incremental compilation artifacts are shared.

set -euo pipefail

INPUT=$(cat)
WORKTREE_PATH=$(echo "$INPUT" | jq -r '.worktree_path')
ORIGINAL_DIR=$(echo "$INPUT" | jq -r '.cwd')

# ── 1. Vendor web-ui assets ──────────────────────────────────────────────────
echo "Vendoring web-ui assets..."
cd "$WORKTREE_PATH"
make vendor

# ── 2. Share Cargo target directory with the original repo ───────────────────
mkdir -p "$WORKTREE_PATH/.cargo"
CARGO_CONFIG="$WORKTREE_PATH/.cargo/config.toml"

# Preserve any existing content (e.g. cross-compilation linker config).
if [[ -f "$CARGO_CONFIG" ]]; then
    # Only add the target-dir block if it isn't already set.
    if ! grep -q '\[build\]' "$CARGO_CONFIG"; then
        printf '\n[build]\ntarget-dir = "%s/target"\n' "$ORIGINAL_DIR" >> "$CARGO_CONFIG"
        echo "Appended [build] target-dir to existing .cargo/config.toml"
    else
        echo ".cargo/config.toml already has a [build] section — skipping target-dir"
    fi
else
    printf '[build]\ntarget-dir = "%s/target"\n' "$ORIGINAL_DIR" > "$CARGO_CONFIG"
    echo "Created .cargo/config.toml with shared target-dir"
fi

echo "Worktree setup complete."
exit 0
