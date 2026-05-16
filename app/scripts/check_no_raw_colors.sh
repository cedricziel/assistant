#!/usr/bin/env bash
#
# Enforce theme token discipline (#266).
#
# Fail when any file under `app/lib/` outside `app/lib/shared/theme/` references
# a raw `Color(0x...)` literal or `Colors.X` named colour. Tokens live in
# `app/lib/shared/theme/assistant_colors.dart`; everything else SHALL go through
# them.
#
# The allow-list file at `app/scripts/theme_color_allowlist.txt` documents
# legitimate exceptions (one path per line, `#` comments allowed). Add entries
# only after agreeing the exception is necessary.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIB_DIR="$ROOT/lib"
THEME_DIR="$ROOT/lib/shared/theme"
ALLOWLIST="$ROOT/scripts/theme_color_allowlist.txt"

if [[ ! -d "$LIB_DIR" ]]; then
  echo "check_no_raw_colors.sh: lib/ not found at $LIB_DIR" >&2
  exit 1
fi

# Load allow-list into an associative-style lookup.
allow_paths=()
if [[ -f "$ALLOWLIST" ]]; then
  while IFS= read -r line; do
    # Strip comments and trim whitespace.
    line="${line%%#*}"
    line="${line#"${line%%[![:space:]]*}"}"
    line="${line%"${line##*[![:space:]]}"}"
    [[ -z "$line" ]] && continue
    allow_paths+=("$line")
  done < "$ALLOWLIST"
fi

is_allowed() {
  local rel="$1"
  for p in "${allow_paths[@]}"; do
    if [[ "$rel" == "$p" ]]; then return 0; fi
  done
  return 1
}

fail=0
while IFS= read -r -d '' file; do
  rel="${file#$ROOT/}"

  # Theme module is exempt by design.
  if [[ "$file" == "$THEME_DIR"/* ]]; then continue; fi
  # Explicit allow-list.
  if is_allowed "$rel"; then continue; fi

  # `Colors.transparent` is semantically "no fill" and stays allowed
  # everywhere; filter it out before matching so the lint doesn't flag it.
  if grep -nE 'Color\(0x|\bColors\.' "$file" \
      | grep -vE '\bColors\.transparent\b' \
      >/dev/null; then
    matches=$(grep -nE 'Color\(0x|\bColors\.' "$file" \
                | grep -vE '\bColors\.transparent\b')
    while IFS= read -r match; do
      echo "$rel:$match — raw colour reference. Use a token from lib/shared/theme/assistant_colors.dart or document an exception in scripts/theme_color_allowlist.txt." >&2
      fail=1
    done <<< "$matches"
  fi
done < <(find "$LIB_DIR" -type f -name "*.dart" -print0)

if [[ "$fail" -ne 0 ]]; then
  echo "" >&2
  echo "check_no_raw_colors.sh: raw colour references found above. See docs/web-ui.md#theme for the token system." >&2
  exit 1
fi

echo "check_no_raw_colors.sh: no raw colour references outside lib/shared/theme/."
