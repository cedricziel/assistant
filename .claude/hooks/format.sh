#!/bin/bash
# Auto-format files written or edited by Claude.
# Runs cargo fmt for Rust files, prettier for web/config files.
# Exit 0 always — formatting failures are non-fatal.

FILE=$(jq -r '.tool_input.file_path // empty')
[ -z "$FILE" ] && exit 0

case "$FILE" in
  *.rs)
    MANIFEST=$(git -C "$(dirname "$FILE")" rev-parse --show-toplevel 2>/dev/null)/Cargo.toml
    if [ -f "$MANIFEST" ]; then
      cargo fmt --manifest-path "$MANIFEST" -- "$FILE" 2>/dev/null
    fi
    ;;
  *.js|*.ts|*.jsx|*.tsx|*.json|*.css|*.html|*.md|*.yaml|*.yml)
    npx --yes prettier --write "$FILE" 2>/dev/null
    ;;
esac

exit 0
