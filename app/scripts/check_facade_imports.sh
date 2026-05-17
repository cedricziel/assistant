#!/usr/bin/env bash
#
# Enforce façade-import discipline for the adaptive UI consolidation.
#
# Phase 4 of the adaptive-ui-consolidation OpenSpec change bans direct
# imports of `package:flutter/cupertino.dart` and `package:flutter/material.dart`
# in feature code. Feature code routes UI primitives through the barrel at
# `app/lib/shared/platform/widgets.dart`.
#
# Allowlists live at:
#   scripts/facade_cupertino_allowlist.txt
#   scripts/facade_material_allowlist.txt
#
# `lib/shared/platform/**` and `lib/main.dart` are always allowed.
#
# The Material allowlist is generous on day one (all files currently
# violating the rule are listed) and shrinks as each feature file migrates
# to the barrel (one ratchet per follow-up PR).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIB_DIR="$ROOT/lib"
PLATFORM_DIR="$ROOT/lib/shared/platform"
CUPERTINO_ALLOW="$ROOT/scripts/facade_cupertino_allowlist.txt"
MATERIAL_ALLOW="$ROOT/scripts/facade_material_allowlist.txt"

if [[ ! -d "$LIB_DIR" ]]; then
  echo "check_facade_imports.sh: lib/ not found at $LIB_DIR" >&2
  exit 1
fi

# Read allowlist into a set (one path per line, ignore comments + blanks).
read_allowlist() {
  local file="$1"
  if [[ ! -f "$file" ]]; then
    return 0
  fi
  grep -vE '^\s*(#|$)' "$file" || true
}

# Files always allowed by structure: barrel + main entry point + theme.
# The theme dir defines Material ThemeData and colour tokens — material.dart
# is unavoidable there.
is_structural_allowed() {
  local rel="$1"
  case "$rel" in
    lib/shared/platform/*) return 0 ;;
    lib/shared/theme/*)    return 0 ;;
    lib/main.dart)         return 0 ;;
  esac
  return 1
}

scan() {
  local pattern="$1"
  local label="$2"
  local allow_file="$3"

  local violators=()
  while IFS= read -r abs; do
    local rel="${abs#"$ROOT/"}"
    if is_structural_allowed "$rel"; then continue; fi
    if grep -qxF "$rel" <(read_allowlist "$allow_file"); then continue; fi
    violators+=("$rel")
  done < <(grep -rlE "^import '$pattern'" "$LIB_DIR" --include="*.dart" 2>/dev/null || true)

  if [[ ${#violators[@]} -gt 0 ]]; then
    echo
    echo "FAIL: direct '$label' import in non-allowlisted file(s):"
    for v in "${violators[@]}"; do
      echo "  - $v"
    done
    echo
    echo "Either:"
    echo "  1) Migrate the file to import 'package:assistant_app/shared/platform/widgets.dart'"
    echo "     (the façade barrel — see lib/shared/platform/widgets.dart for what it exports)"
    echo "  2) Add the file to $(basename "$allow_file") with a TODO line explaining why"
    return 1
  fi
}

ec=0
scan 'package:flutter/cupertino\.dart' 'flutter/cupertino.dart' "$CUPERTINO_ALLOW" || ec=$?
scan 'package:flutter/material\.dart'  'flutter/material.dart'  "$MATERIAL_ALLOW"  || ec=$?

if [[ $ec -eq 0 ]]; then
  echo "check_facade_imports.sh: no façade-import violations."
fi
exit $ec
