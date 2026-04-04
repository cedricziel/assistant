# Web UI

The assistant ships an optional web UI for inspecting traces, logs,
metrics, managing Persona A2A Profiles, and configuring webhooks. It runs through
the unified `assistant` binary (`assistant webui serve`) and reads from
the same SQLite database the runtime writes to.

## Quick start

```sh
# Auth token is required — the server refuses to start without one.
ASSISTANT_WEB_TOKEN=changeme cargo run -p assistant-cli -- webui serve --listen 127.0.0.1:8080
```

The Flutter web app loads at <http://127.0.0.1:8080>. Enter the server URL
and token in the connection screen to sign in.

## CLI options

| Flag                 | Env var                     | Default                     | Description                                               |
| -------------------- | --------------------------- | --------------------------- | --------------------------------------------------------- |
| `--auth-token`       | `ASSISTANT_WEB_TOKEN`       | _(required)_                | Token used for Bearer auth                                |
| `--listen`           |                             | `127.0.0.1:8080`            | Address to bind                                           |
| `--db-path`          |                             | `~/.assistant/assistant.db` | Path to the SQLite database                               |
| `--trace-limit`      |                             | `200`                       | Max traces returned by `GET /api/traces`                  |
| `--log-limit`        |                             | `500`                       | Max log entries returned by `GET /api/logs`               |
| `--cors-origin`      | `ASSISTANT_WEB_CORS_ORIGIN` | _(wildcard)_                | Restrict CORS to a specific origin (e.g. macOS app URL)   |
| `--no-secure-cookie` |                             | `false`                     | Disable `Secure` attribute on session cookies (see below) |

### Plain HTTP on non-loopback addresses

When the server binds to a non-loopback address (e.g. `0.0.0.0`), it
automatically sets the `Secure` attribute on session cookies. This
means browsers will only send the cookie over HTTPS — if you access the
UI over plain HTTP, login will appear to succeed but the session cookie
is silently rejected.

If you are running behind a VPN or firewall where plain HTTP is
acceptable, pass `--no-secure-cookie` to disable this behaviour:

```sh
assistant webui serve --listen 0.0.0.0:8080 --no-secure-cookie --auth-token changeme
```

## Flutter app (primary interface)

All unmatched paths are handled by the embedded Flutter web app (SPA
fallback). The app uses the REST API below to drive the chat,
persona switching, traces, logs, and skills views.

## REST API (`/api/*`)

These endpoints power the Flutter app and are also consumable directly:

| Route                              | Method | Description                                       |
| ---------------------------------- | ------ | ------------------------------------------------- |
| `/api/conversations`               | GET    | List conversations                                |
| `/api/conversations`               | POST   | Create conversation                               |
| `/api/conversations/{id}`          | GET    | Get conversation with messages                    |
| `/api/conversations/{id}`          | PATCH  | Rename conversation                               |
| `/api/conversations/{id}`          | DELETE | Delete conversation                               |
| `/api/conversations/{id}/messages` | POST   | Send message (SSE streaming response)             |
| `/api/personas`                    | GET    | List personas                                     |
| `/api/personas/active`             | POST   | Switch active persona                             |
| `/api/personas/{id}/skills`        | GET    | List skills for a persona                         |
| `/api/traces`                      | GET    | List traces (supports limit/offset/filter params) |
| `/api/traces/{id}`                 | GET    | Get trace with span breakdown                     |
| `/api/logs`                        | GET    | List log entries (supports filter params)         |

Full OpenAPI spec is served at `/api/openapi.json`; Swagger UI at `/api/docs`.

## Server-side management pages

| Route                    | Description                                                                |
| ------------------------ | -------------------------------------------------------------------------- |
| `/analytics`             | Metrics dashboard — token usage, model comparison, tool stats, error rates |
| `/agents`                | A2A Profile management — list, create, edit, delete                        |
| `/agents/{id}/card.json` | Raw A2A Profile card JSON                                                  |
| `/webhooks`              | Webhook management — list, create, toggle, rotate secrets                  |

## A2A protocol endpoints

The web UI also serves the [Agent-to-Agent protocol](https://google.github.io/A2A/)
endpoints for machine-to-machine communication:

| Route                              | Auth      | Description                               |
| ---------------------------------- | --------- | ----------------------------------------- |
| `/.well-known/agent.json`          | Public    | A2A Profile discovery card (per A2A spec) |
| `/agent/authenticatedExtendedCard` | Protected | Extended A2A Profile card                 |
| `/message/send`                    | Protected | Send a message (request/response)         |
| `/message/stream`                  | Protected | Send a message (SSE streaming)            |
| `/tasks`                           | Protected | List tasks                                |
| `/tasks/{id}`                      | Protected | Get task                                  |
| `/tasks/{id}/cancel`               | Protected | Cancel task                               |
| `/tasks/{id}/subscribe`            | Protected | Subscribe to task updates (SSE)           |

Protected endpoints require either a valid session cookie (browser) or
`Authorization: Bearer <token>` header (API). See
[authentication.md](authentication.md) for details.

## Auto-hardening

When authentication is enabled (always), the web UI automatically
injects a `bearer_token` security scheme into the Persona A2A Profile card. This
means callers discovering the profile via `/.well-known/agent.json` will
see that Bearer authentication is required before making any API calls.

## Architecture

The server is an Axum application that embeds the Flutter web app at
compile time via `rust-embed`. All unmatched routes fall through to the
Flutter SPA so client-side routing (`go_router`) works correctly.

The Flutter app lives in `app/` and is built automatically by
`crates/web-ui/build.rs` when Cargo compiles the crate (requires the
Flutter SDK on `PATH`; falls back to a placeholder if unavailable).

```
assistant webui serve
├── build.rs            # Runs `flutter build web` before compilation
├── src/
│   ├── flutter_assets.rs   # rust-embed of app/build/web/ → SPA fallback handler
│   ├── auth.rs             # Bearer token middleware + session cookies
│   ├── main.rs             # CLI args, router assembly, CORS layer
│   ├── api/                # REST API consumed by the Flutter app
│   │   ├── mod.rs              # /api/conversations + /api/conversations/{id}/messages (SSE)
│   │   ├── personas.rs         # /api/personas, /api/personas/active
│   │   ├── skills.rs           # /api/personas/{id}/skills
│   │   ├── traces.rs           # /api/traces, /api/traces/{id}
│   │   └── logs.rs             # /api/logs
│   ├── a2a/                # A2A protocol endpoints + agent management pages
│   └── webhooks/           # Webhook management pages
app/                        # Flutter source (web + macOS targets)
```

The Flutter macOS app connects to any `assistant webui serve` instance
(local or remote) by entering the server URL and token in the connection
screen. Credentials are stored in the platform keychain via
`flutter_secure_storage`.
