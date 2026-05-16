# Web UI

The assistant ships an optional web UI for inspecting traces, logs,
metrics, managing Persona A2A Profiles, and configuring webhooks. It runs through
the unified `assistant` binary (`assistant webui serve`) and reads from
the same SQLite database the runtime writes to.

## Quick start

```sh
# Legacy single-token mode (for quick local dev):
ASSISTANT_WEB_TOKEN=changeme cargo run -p assistant-cli -- webui serve --listen 127.0.0.1:8080

# Multi-user mode is enabled automatically when org.db exists.
# See docs/authentication.md for OAuth2 setup and docs/multi-user.md for the full model.
```

The Flutter web app loads at <http://127.0.0.1:8080>. In single-token mode,
enter the server URL and token. In multi-user mode, the app uses OAuth2
Authorization Code + PKCE to authenticate.

## Navigation sidebar

On viewports >= 768 dp wide, the app renders a navigation sidebar with all
destinations. Two affordances toggle the collapsed state:

- A top-leading **Hide navigation / Show navigation** icon button rendered as
  an overlay on the main content area. Discoverable on iPad landscape and
  desktop browser windows.
- A smaller toggle inside the sidebar itself (right-aligned at the top of the
  rail).

On touch input devices (iPad PWA, iOS / Android), a horizontal swipe starting
from the left 20 logical pixels of the viewport toggles the sidebar — drag
right to expand, left to collapse. Mouse drags are ignored.

Collapse state persists across reloads under the `SharedPreferences` key
`assistant.sidebarCollapsed` (localStorage on web).

## Trace detail — tool call cards

Tool invocations performed by the agent are recorded as OpenTelemetry spans
named `execute_tool <tool_name>` with the following attributes:

| Attribute                                           | Description                                                    |
| --------------------------------------------------- | -------------------------------------------------------------- |
| `tool_name`                                         | Tool the agent invoked (e.g. `file-read`, `bash`, `web-fetch`) |
| `tool_params`                                       | JSON-encoded parameter object                                  |
| `tool_status`                                       | `ok`, `error`, or `denied`                                     |
| `tool_observation`                                  | Output returned to the model (success path)                    |
| `tool_error`                                        | Error / denial message (failure / denied path)                 |
| `duration_ms`                                       | Execution duration                                             |
| `iteration` / `turn` / `interface` / `active_skill` | ReAct loop context                                             |

The trace detail screen (`/traces/{id}`) renders these spans as dedicated
**tool-call cards** with:

- Status icon + colour pill: ✓ (`tertiary`) for `ok`, ✕ (`error`) for `error`,
  ⊘ (amber) for `denied`, ? (neutral) when unknown.
- Side-by-side **Params** / **Output** panes when expanded (vertical stack
  on viewports narrower than 600 dp).
- A `Show all attributes` toggle to reveal the full attribute map.

Spans that don't match this shape (LLM chat calls, orchestrator spans, etc.)
keep the generic span card.

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

| Route                                  | Method | Description                                       |
| -------------------------------------- | ------ | ------------------------------------------------- |
| `/api/conversations`                   | GET    | List conversations                                |
| `/api/conversations`                   | POST   | Create conversation                               |
| `/api/conversations/{id}`              | GET    | Get conversation with messages                    |
| `/api/conversations/{id}`              | PATCH  | Rename conversation                               |
| `/api/conversations/{id}`              | DELETE | Delete conversation                               |
| `/api/conversations/{id}/messages`     | POST   | Send message (SSE streaming response)             |
| `/api/personas`                        | GET    | List personas                                     |
| `/api/personas/active`                 | POST   | Switch active persona                             |
| `/api/personas/{id}/skills`            | GET    | List skills for a persona                         |
| `/api/traces`                          | GET    | List traces (supports limit/offset/filter params) |
| `/api/traces/{id}`                     | GET    | Get trace with span breakdown                     |
| `/api/logs`                            | GET    | List log entries (supports filter params)         |
| `/api/orgs`                            | GET    | List organizations (filtered by user access)      |
| `/api/orgs`                            | POST   | Create organization                               |
| `/api/orgs/{id}`                       | GET    | Get organization details                          |
| `/api/orgs/{id}`                       | PATCH  | Update organization settings                      |
| `/api/orgs/{org}/users`                | GET    | List users in org                                 |
| `/api/orgs/{org}/users`                | POST   | Invite user to org                                |
| `/api/orgs/{org}/users/{uid}`          | GET    | Get user details                                  |
| `/api/orgs/{org}/users/{uid}`          | PATCH  | Update user (name, role)                          |
| `/api/orgs/{org}/users/{uid}`          | DELETE | Remove user from org                              |
| `/api/orgs/{org}/spaces`               | GET    | List spaces (filtered by membership)              |
| `/api/orgs/{org}/spaces`               | POST   | Create space                                      |
| `/api/orgs/{org}/spaces/{sid}`         | GET    | Get space details                                 |
| `/api/orgs/{org}/spaces/{sid}`         | PATCH  | Update space                                      |
| `/api/orgs/{org}/spaces/{sid}`         | DELETE | Delete space (org-admin only)                     |
| `/api/orgs/{org}/spaces/{sid}/members` | GET    | List space members                                |
| `/api/orgs/{org}/spaces/{sid}/members` | POST   | Add user to space with role                       |
| `/api/users/me/api-keys`               | GET    | List API keys (prefix + name, no secrets)         |
| `/api/users/me/api-keys`               | POST   | Create scoped API key (returns plaintext once)    |
| `/api/users/me/api-keys/{kid}`         | DELETE | Revoke API key                                    |

Full OpenAPI spec is served at `/api/openapi.json`; Swagger UI at `/api/docs`.

## OAuth2 endpoints (`/oauth/*`)

These endpoints implement the assistant's OAuth2 Authorization Server.
See [authentication.md](authentication.md) for flow details.

| Route                                     | Method   | Description                                       |
| ----------------------------------------- | -------- | ------------------------------------------------- |
| `/oauth/register`                         | POST     | Dynamic client registration (RFC 7591)            |
| `/oauth/authorize`                        | GET      | Render login form or redirect to IdP (OIDC)       |
| `/oauth/authorize`                        | POST     | Validate credentials, redirect with auth code     |
| `/oauth/token`                            | POST     | Exchange auth code, refresh token, or device code |
| `/oauth/device`                           | POST     | Initiate device code flow (RFC 8628)              |
| `/oauth/device/verify`                    | GET/POST | User enters device code and authenticates         |
| `/oauth/callback`                         | GET      | OIDC IdP callback (server as OIDC client)         |
| `/oauth/complete`                         | GET      | Auth code delivery as JSON (Flutter web)          |
| `/oauth/revoke`                           | POST     | Token revocation (RFC 7009)                       |
| `/.well-known/oauth-authorization-server` | GET      | Server metadata (RFC 8414)                        |

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
│   ├── auth.rs             # Auth middleware: JWT, API keys, legacy tokens, session cookies
│   ├── main.rs             # CLI args, router assembly, CORS layer
│   ├── oauth/              # OAuth2 Authorization Server endpoints
│   │   ├── authorize.rs        # GET/POST /oauth/authorize
│   │   ├── token.rs            # POST /oauth/token
│   │   ├── register.rs         # POST /oauth/register (RFC 7591)
│   │   ├── device.rs           # POST /oauth/device + /oauth/device/verify (RFC 8628)
│   │   ├── callback.rs         # GET /oauth/callback (OIDC IdP callback)
│   │   ├── complete.rs         # GET /oauth/complete (auth code as JSON)
│   │   └── revoke.rs           # POST /oauth/revoke + metadata endpoint
│   ├── api/                # REST API consumed by the Flutter app
│   │   ├── mod.rs              # /api/conversations + /api/conversations/{id}/messages (SSE)
│   │   ├── personas.rs         # /api/personas, /api/personas/active
│   │   ├── skills.rs           # /api/personas/{id}/skills
│   │   ├── traces.rs           # /api/traces, /api/traces/{id}
│   │   ├── logs.rs             # /api/logs
│   │   ├── orgs.rs             # /api/orgs CRUD
│   │   ├── users.rs            # /api/orgs/{org}/users CRUD
│   │   ├── spaces.rs           # /api/orgs/{org}/spaces CRUD + members
│   │   └── api_keys.rs         # /api/users/me/api-keys CRUD
│   ├── a2a/                # A2A protocol endpoints + agent management pages
│   └── webhooks/           # Webhook management pages
app/                        # Flutter source (web + macOS targets)
```

The Flutter macOS app connects to any `assistant webui serve` instance
(local or remote). In multi-user mode it authenticates via OAuth2
Authorization Code + PKCE; in legacy mode it accepts a server URL and
token. Credentials (OAuth tokens or legacy token) are stored in the
platform keychain via `flutter_secure_storage`.
