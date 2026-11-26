---
name: verify
description: >
  Run a throwaway assistant server with the freshly built web app and drive it
  in a browser or with MCP clients to confirm a change works end to end. Use
  before opening a PR that changes user-visible behavior (web UI, sign-in,
  OAuth, routing, the MCP server or MCP Apps), or when asked to verify,
  smoke-test or "see it working".
---

# Verify

Starts an isolated server (temporary `HOME`, free loopback ports, no user
config, a generated admin password) and checks behavior in the preview browser
or with MCP clients. Never point this at `~/.assistant` or a live server.

## 1. Build

```sh
make build-web                 # app/build/web, embedded by the server
cargo build -p assistant-cli   # target/debug/assistant
```

Rebuild the web app after every Flutter change; `cargo build` alone embeds the
previous web build.

For the MCP flows, also build the example MCP server once:

```sh
(cd examples/mcp-apps-demo && npm ci && npm run build)
```

## 2. Start an instance

```sh
.claude/skills/verify/scripts/start-instance.sh              # web UI + orchestrator
.claude/skills/verify/scripts/start-instance.sh --mcp-demo   # plus the MCP demo
```

It prints `URL`, `HOME`, `PID`, `LOG` and `ADMIN_CREDENTIALS`. On first start
the server seeds org `default` with user `admin@localhost` and writes the
generated password to `ADMIN_CREDENTIALS` (`password: …` line). The web UI
only publishes turns, so the script also starts `orchestrator run --no-repl`
on the same `HOME` (log: `HOME/orchestrator.log`).

`--mcp-demo` lists `examples/mcp-apps-demo` under `[mcp]` and points the org's
LLM at the scripted model in `crates/web-ui/e2e/tests/support/mock-ollama.mjs`:
any message containing "weather" calls `mcp__demo__get-forecast`. It also
allows that tool in `[permissions]`, since web and MCP turns cannot prompt.

## 3. Drive it

Use the preview tools in an **incognito** tab (`profileId: "incognito"`,
`reuseExistingTab: false`) so no session leaks between runs.

- Load the app from `URL/`, not `URL/login`: the server itself serves a legacy
  token form at exactly `/login`. In-app navigation to `/login` is the Flutter
  screen.
- Flutter renders to a canvas: the accessibility tree is mostly empty, so
  click by coordinates from a screenshot. Server-rendered pages
  (`/oauth/authorize`) have normal refs.
- Wait on URLs with `preview_wait_for` (`urlIncludes`) rather than sleeping.

## 4. Check the MCP server

With an instance started with `--mcp-demo`:

```sh
.claude/skills/verify/scripts/mcp-smoke.py <URL> <HOME>
```

It prints a `PASS`/`FAIL` line per check and exits non-zero on a failure:

- `assistant mcp` over stdio with the instance's `HOME` lists its tools;
- a client registered at `/oauth/register` signs in as the admin with PKCE and
  `resource=<URL>/mcp`, then lists tools on `/mcp` and calls
  `mcp__demo__get-forecast`. Pass another tool and JSON arguments as the third
  and fourth arguments.

## 5. Clean up, even on failure

```sh
.claude/skills/verify/scripts/stop-instance.sh <HOME>
```

It stops the web UI, the orchestrator and the scripted model, and deletes
`HOME`. Close the preview tab with `t3_preview_close`.

## Covered flows

| Flow                          | Steps                                                                                                                                                                            | Expect                                                                                                                                          |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Web sign-in (password org)    | `URL/` → **Sign in** → email `admin@localhost` → Continue → password → Sign In                                                                                                   | `/oauth/register` 201, server email page, 303 to `/login/callback?code=…&state=…`, `/oauth/token` 200, lands on `/chat`                         |
| Native redirect (server side) | `POST /oauth/register` with redirect `com.cedricziel.assistant:/oauth/callback`, then `POST /oauth/authorize` (`org`, `email`, `password`, PKCE `S256`, `Origin: URL`) with curl | `303` with `location: com.cedricziel.assistant:/oauth/callback?code=…&state=…`                                                                  |
| MCP over stdio and HTTP       | `start-instance.sh --mcp-demo`, then `mcp-smoke.py <URL> <HOME>`                                                                                                                 | four `PASS` lines; the call returns "Sunny with a light breeze, 21°C in Berlin"                                                                 |
| MCP App rendering             | `cd crates/web-ui/e2e && ASSISTANT_BIN=$PWD/../../../target/debug/assistant npx playwright test -g "MCP Apps"` (starts its own server with the demo under `[mcp]`)               | 2 passed: the demo app renders below the `mcp__demo__get-forecast` chip, and **Save as favorite** gets `409`, a confirmation dialog, then `200` |

Not covered yet: OIDC orgs (needs a local identity provider), the native
sign-in sheet itself (needs simulator access: Settings → Integrations →
Devices), `assistant login` device approval, `run-prompt` as an MCP task (server
Tasks are not merged yet). A turn sent with `POST .../messages` while a
separate orchestrator runs it never sends its `done` SSE frame; wait on
`GET /api/conversations/{id}` instead. Add a row when you verify a new
flow.
