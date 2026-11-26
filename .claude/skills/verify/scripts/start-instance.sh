#!/usr/bin/env bash
# Start a throwaway assistant server: its own HOME, free port, no user state.
# Prints KEY=value lines (URL, HOME, PID, LOG, ADMIN_EMAIL, ADMIN_PASSWORD).
# Stop it with stop-instance.sh <HOME>.
set -euo pipefail

repo_root="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
bin="$repo_root/target/debug/assistant"
[[ -x "$bin" ]] || { echo "missing $bin: run 'make build-web && cargo build -p assistant-cli'" >&2; exit 1; }

home="$(mktemp -d "${TMPDIR:-/tmp}/assistant-verify.XXXXXX")"
port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
url="http://127.0.0.1:$port"
log="$home/server.log"

# Empty LLM/agent env so the user's own settings never leak in.
env -i PATH="$PATH" HOME="$home" TMPDIR="${TMPDIR:-/tmp}" RUST_LOG=info \
  "$bin" webui serve --listen "127.0.0.1:$port" --public-url "$url" \
  >"$log" 2>&1 &
pid=$!
echo "$pid" >"$home/server.pid"

for _ in $(seq 1 60); do
  if curl -fs -o /dev/null "$url/.well-known/oauth-authorization-server"; then break; fi
  kill -0 "$pid" 2>/dev/null || { echo "server exited; see $log" >&2; tail -20 "$log" >&2; exit 1; }
  sleep 1
done

creds="$(find "$home/.assistant/orgs" -name admin_credentials.txt 2>/dev/null | head -1)"
echo "URL=$url"
echo "HOME=$home"
echo "PID=$pid"
echo "LOG=$log"
if [[ -n "$creds" ]]; then
  echo "ADMIN_CREDENTIALS=$creds"
fi
