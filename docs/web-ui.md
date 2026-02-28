# Web UI

The assistant ships an optional web UI for inspecting traces, logs,
metrics, managing A2A agents, and configuring webhooks.  It is a
standalone binary (`assistant-web-ui`) that reads from the same SQLite
database the runtime writes to.

## Quick start

```sh
# Auth token is required — the server refuses to start without one.
ASSISTANT_WEB_TOKEN=changeme cargo run -p assistant-web-ui -- --listen 127.0.0.1:8080
```

Open <http://127.0.0.1:8080/login> and enter the token to sign in.

## CLI options

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--auth-token` | `ASSISTANT_WEB_TOKEN` | *(required)* | Token used for login and Bearer auth |
| `--listen` | | `127.0.0.1:8080` | Address to bind |
| `--db-path` | | `~/.assistant/assistant.db` | Path to the SQLite database |
| `--trace-limit` | | `200` | Max traces shown on the dashboard |
| `--log-limit` | | `500` | Max logs shown on the logs page |
| `--no-secure-cookie` | | `false` | Disable `Secure` attribute on session cookies (see below) |

### Plain HTTP on non-loopback addresses

When the server binds to a non-loopback address (e.g. `0.0.0.0`), it
automatically sets the `Secure` attribute on session cookies.  This
means browsers will only send the cookie over HTTPS — if you access the
UI over plain HTTP, login will appear to succeed but the session cookie
is silently rejected.

If you are running behind a VPN or firewall where plain HTTP is
acceptable, pass `--no-secure-cookie` to disable this behaviour:

```sh
assistant-web-ui --listen 0.0.0.0:8080 --no-secure-cookie
```

## Pages

| Route | Description |
|-------|-------------|
| `/` | Dashboard — recent traces with span counts, errors, tool names |
| `/traces` | Same as `/` |
| `/trace/{id}` | Trace detail — full span tree for a single trace |
| `/logs` | Log viewer — filterable by severity, target, search, trace ID |
| `/log/{id}` | Single log record detail |
| `/analytics` | Metrics dashboard — token usage, model comparison, tool stats, error rates |
| `/agents` | A2A agent management — list, create, edit, delete |
| `/agents/new` | Create a new agent card |
| `/agents/{id}` | Agent detail view |
| `/agents/{id}/card.json` | Raw agent card JSON |
| `/webhooks` | Webhook management — list, create, toggle, rotate secrets |

## A2A protocol endpoints

The web UI also serves the [Agent-to-Agent protocol](https://google.github.io/A2A/)
endpoints for machine-to-machine communication:

| Route | Auth | Description |
|-------|------|-------------|
| `/.well-known/agent.json` | Public | Agent card discovery (per A2A spec) |
| `/agent/authenticatedExtendedCard` | Protected | Extended agent card |
| `/message/send` | Protected | Send a message (request/response) |
| `/message/stream` | Protected | Send a message (SSE streaming) |
| `/tasks` | Protected | List tasks |
| `/tasks/{id}` | Protected | Get task |
| `/tasks/{id}/cancel` | Protected | Cancel task |
| `/tasks/{id}/subscribe` | Protected | Subscribe to task updates (SSE) |

Protected endpoints require either a valid session cookie (browser) or
`Authorization: Bearer <token>` header (API).  See
[authentication.md](authentication.md) for details.

## Auto-hardening

When authentication is enabled (always), the web UI automatically
injects a `bearer_token` security scheme into the A2A agent card.  This
means callers discovering the agent via `/.well-known/agent.json` will
see that Bearer authentication is required before making any API calls.

## Architecture

The web UI is a server-side rendered (SSR) Axum application.  There is
no JavaScript framework — pages are plain HTML generated in Rust with
inline CSS.  This keeps the dependency footprint minimal and the UI
fast.

```
assistant-web-ui
├── auth.rs         # Token auth middleware, login page, session cookies
├── common.rs       # Shared HTML helpers (sidebar, page shell, escaping)
├── main.rs         # CLI args, router setup, auto-hardening, page handlers
├── a2a/
│   ├── mod.rs          # Public + protected router split
│   ├── handlers.rs     # A2A protocol JSON handlers
│   ├── pages.rs        # Agent management HTML pages
│   ├── agent_store.rs  # Filesystem-backed agent card store
│   └── task_store.rs   # In-memory A2A task store
└── webhooks/
    ├── mod.rs          # Webhook router
    └── pages.rs        # Webhook management HTML pages
```
