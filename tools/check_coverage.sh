#!/usr/bin/env bash
# tools/check_coverage.sh
#
# Reads coverage.json (produced by `cargo llvm-cov --workspace --json`)
# and coverage.toml; computes per-crate line coverage; prints a sorted
# table; exits non-zero when any crate not in the report-only allowlist
# falls below the floor.
#
# Dependencies: bash 3.2+, jq, awk (POSIX), grep, sed.
#
# Specified by openspec/changes/workspace-test-coverage-floor/.

set -euo pipefail

# --- Locate inputs ----------------------------------------------------------

ROOT="${ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
COVERAGE_JSON="${COVERAGE_JSON:-$ROOT/coverage.json}"
COVERAGE_TOML="${COVERAGE_TOML:-$ROOT/coverage.toml}"

if [[ ! -f "$COVERAGE_JSON" ]]; then
    echo "error: $COVERAGE_JSON not found." >&2
    echo "       run 'make coverage' (or 'cargo llvm-cov --workspace --json --output-path coverage.json') first." >&2
    exit 2
fi

if [[ ! -f "$COVERAGE_TOML" ]]; then
    echo "error: $COVERAGE_TOML not found." >&2
    exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq is required but not installed." >&2
    exit 2
fi

# --- Parse coverage.toml ----------------------------------------------------
#
# POSIX awk only — no gawk `match(...,arr)` extensions, no bash mapfile.
# Tracks the active [section] header and extracts double-quoted strings
# inside a `key = [ ... ]` array spanning one or more lines.

extract_array() {
    local section="$1"
    local key="$2"
    awk -v sect="[$section]" -v target_key="$key" '
        BEGIN { in_section = 0; in_array = 0 }
        /^\[.+\]$/ {
            in_section = ($0 == sect) ? 1 : 0
            in_array = 0
            next
        }
        {
            line = $0
        }
        in_section && !in_array && line ~ /^[[:space:]]*[a-zA-Z_]+[[:space:]]*=[[:space:]]*\[/ {
            k = line
            sub(/^[[:space:]]+/, "", k)
            sub(/[[:space:]]*=.*$/, "", k)
            if (k == target_key) {
                in_array = 1
                sub(/^[^\[]*\[/, "", line)
            }
        }
        in_array {
            # Pull every "..." occurrence on this line.
            while (match(line, /"[^"]*"/)) {
                s = substr(line, RSTART + 1, RLENGTH - 2)
                print s
                line = substr(line, RSTART + RLENGTH)
            }
            if (index(line, "]") > 0) { in_array = 0 }
        }
    ' "$COVERAGE_TOML"
}

FLOOR="$(awk '
    /^[[:space:]]*floor[[:space:]]*=/ {
        gsub(/[^0-9.]/, "")
        print
        exit
    }
' "$COVERAGE_TOML")"

if [[ -z "$FLOOR" ]]; then
    echo "error: 'floor' not set in $COVERAGE_TOML" >&2
    exit 2
fi

# Portable replacement for `mapfile -t`: read into bash 3.2-compatible array.
EXCLUDED=()
while IFS= read -r line; do
    [[ -n "$line" ]] && EXCLUDED+=("$line")
done < <(extract_array excluded_crates crates)

REPORT_ONLY=()
while IFS= read -r line; do
    [[ -n "$line" ]] && REPORT_ONLY+=("$line")
done < <(extract_array report_only crates)

is_in_list() {
    local needle="$1"
    shift
    local item
    for item in "$@"; do
        [[ "$item" == "$needle" ]] && return 0
    done
    return 1
}

# --- Compute per-crate coverage from coverage.json --------------------------
#
# llvm-cov's `--json` output places file-level coverage under
# `.data[0].files[].summary.lines.{count,covered}`. We group by the
# `crates/<name>/` segment of the filename, sum count and covered per
# crate, and compute percent.
#
# Files outside `crates/<name>/` are ignored.

CRATE_DATA="$(jq -r '
    .data[0].files
    | map(select(.filename | test("/crates/[^/]+/")))
    | group_by(.filename | capture("/crates/(?<c>[^/]+)/") | .c)
    | map({
        crate: (.[0].filename | capture("/crates/(?<c>[^/]+)/") | .c),
        covered: (map(.summary.lines.covered) | add // 0),
        count:   (map(.summary.lines.count)   | add // 0)
      })
    | map(. + { percent: (if .count > 0 then (.covered * 100 / .count) else 100 end) })
    | sort_by(.percent)
    | .[] | "\(.crate)\t\(.covered)\t\(.count)\t\(.percent)"
' "$COVERAGE_JSON")"

# --- Print the table --------------------------------------------------------

printf "\n%-36s %10s %10s %8s %s\n" "CRATE" "COVERED" "TOTAL" "PERCENT" "STATUS"
printf -- "----------------------------------------------------------------------------------\n"

FAILED=0
ANY_REPORTED=0

# Portable array-empty checks (set -u trips on ${arr[@]} with empty arrays in bash 3.2).
excluded_args=()
if [[ "${#EXCLUDED[@]}" -gt 0 ]]; then
    excluded_args=("${EXCLUDED[@]}")
fi
report_only_args=()
if [[ "${#REPORT_ONLY[@]}" -gt 0 ]]; then
    report_only_args=("${REPORT_ONLY[@]}")
fi

while IFS=$'\t' read -r crate covered count percent; do
    [[ -z "$crate" ]] && continue
    ANY_REPORTED=1
    delta="$(awk -v p="$percent" -v f="$FLOOR" 'BEGIN { printf "%+.1f", (p - f) }')"

    if is_in_list "$crate" "${excluded_args[@]+"${excluded_args[@]}"}"; then
        status="excluded"
    elif [[ "$(awk -v p="$percent" -v f="$FLOOR" 'BEGIN { print (p + 0.0001 >= f) }')" == "1" ]]; then
        status="ok"
    elif is_in_list "$crate" "${report_only_args[@]+"${report_only_args[@]}"}"; then
        status="report-only (delta ${delta}%)"
    else
        status="FAIL (delta ${delta}%)"
        FAILED=1
    fi

    printf "%-36s %10d %10d %7.1f%% %s\n" "$crate" "$covered" "$count" "$percent" "$status"
done <<EOF
$CRATE_DATA
EOF

if [[ "$ANY_REPORTED" -eq 0 ]]; then
    echo
    echo "warning: no crates matched the 'crates/<name>/' pattern in coverage.json." >&2
    echo "         coverage.json may be empty or malformed." >&2
fi

echo
echo "floor: ${FLOOR}%   excluded: ${#EXCLUDED[@]}   report-only: ${#REPORT_ONLY[@]}"

if [[ "$FAILED" -eq 1 ]]; then
    echo
    echo "Coverage gate FAILED: one or more enforcing crates fell below the floor." >&2
    exit 1
fi

echo "Coverage gate passed."
