#!/usr/bin/env bash
# Stop an instance from start-instance.sh and delete its HOME.
set -euo pipefail

home="${1:?usage: stop-instance.sh <HOME printed by start-instance.sh>}"
case "$home" in
  */assistant-verify.*) ;;
  *) echo "refusing: $home is not a verify instance" >&2; exit 1 ;;
esac

if [[ -f "$home/server.pid" ]]; then
  kill "$(cat "$home/server.pid")" 2>/dev/null || true
fi
rm -rf "$home"
