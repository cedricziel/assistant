#!/usr/bin/env bash
# Start a throwaway assistant: its own HOME, free ports, no user state. Runs
# `webui serve` and the `orchestrator run` that executes its turns.
#
# Usage: start-instance.sh [--mcp-demo]
#   --mcp-demo  list examples/mcp-apps-demo under [mcp] and use the scripted
#               model from crates/web-ui/e2e/tests/support/mock-ollama.mjs
#               (build the demo first: npm ci && npm run build in its dir).
#
# Prints KEY=value lines (URL, HOME, PID, LOG, ADMIN_CREDENTIALS).
# Stop it with stop-instance.sh <HOME>, also after a failure.
set -euo pipefail

repo_root="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
bin="$repo_root/target/debug/assistant"
[[ -x "$bin" ]] || { echo "missing $bin: run 'make build-web && cargo build -p assistant-cli'" >&2; exit 1; }

mcp_demo=false
case "${1:-}" in
  "") ;;
  --mcp-demo) mcp_demo=true ;;
  *) echo "usage: start-instance.sh [--mcp-demo]" >&2; exit 2 ;;
esac

free_port() { python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])'; }

home="$(mktemp -d "${TMPDIR:-/tmp}/assistant-verify.XXXXXX")"
port="$(free_port)"
url="http://127.0.0.1:$port"
log="$home/server.log"
# A start that fails before printing HOME stops what it started and removes it.
started=false
trap '$started || { for f in "$home"/*.pid; do [[ -f "$f" ]] && kill "$(cat "$f")" 2>/dev/null; done; rm -rf "$home"; }' EXIT
# Empty LLM/agent env so the user's own settings never leak in.
run() { exec env -i PATH="$PATH" HOME="$home" TMPDIR="${TMPDIR:-/tmp}" RUST_LOG=info "$@"; }

if $mcp_demo; then
  demo="$repo_root/examples/mcp-apps-demo"
  [[ -f "$demo/dist/app.html" ]] || { echo "build the demo first: (cd $demo && npm ci && npm run build)" >&2; exit 1; }
  llm_port="$(free_port)"
  node "$repo_root/crates/web-ui/e2e/tests/support/mock-ollama.mjs" "$llm_port" >"$home/mock-llm.log" 2>&1 &
  echo $! >"$home/mock-llm.pid"
  mkdir -p "$home/.assistant/orgs/default"
  cat >"$home/.assistant/server.toml" <<EOF
[[mcp.servers]]
name = "demo"
command = ["$(command -v node)", "$demo/server.mjs"]
trust = "trust"
EOF
  cat >"$home/.assistant/orgs/default/org.toml" <<EOF
[llm]
provider = "ollama"
model = "mock"
base_url = "http://127.0.0.1:$llm_port"

[memory]
enabled = false

[permissions]
allow = ["mcp__demo__get-forecast"]
EOF
fi

run "$bin" webui serve --listen "127.0.0.1:$port" --public-url "$url" >"$log" 2>&1 &
pid=$!
echo "$pid" >"$home/server.pid"

for _ in $(seq 1 60); do
  if curl -fs -o /dev/null "$url/.well-known/oauth-authorization-server"; then break; fi
  kill -0 "$pid" 2>/dev/null || { echo "server exited:" >&2; tail -20 "$log" >&2; exit 1; }
  sleep 1
done

# The web UI only publishes turns; this process runs them.
run "$bin" orchestrator run --no-repl >"$home/orchestrator.log" 2>&1 &
echo $! >"$home/orchestrator.pid"

started=true
creds="$(find "$home/.assistant/orgs" -name admin_credentials.txt 2>/dev/null | head -1)"
echo "URL=$url"
echo "HOME=$home"
echo "PID=$pid"
echo "LOG=$log"
if [[ -n "$creds" ]]; then
  echo "ADMIN_CREDENTIALS=$creds"
fi
