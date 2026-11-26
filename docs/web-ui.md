# Web UI

The assistant ships an optional web UI for chatting with Personas, managing
skills, Persona A2A Profiles and webhooks. It runs through the unified
`assistant` binary (`assistant webui serve`) and reads the same per-space
databases under `~/.assistant/orgs/` that the runtime writes to. Each request
runs against the space the caller selects (see
[Selecting a space](multi-user.md#selecting-a-space)). Telemetry is not shown here: it is exported
over OTLP ([docs/opentelemetry.md](opentelemetry.md)).

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

## Operator note: web-ui requires the orchestrator service

`assistant webui serve` publishes chat turns to the message bus of the
caller's space but **does not spawn the turn-processing worker pool or the
title-generator worker**. It does run the workflow runner, schedule adapter
and event adapter, once per space. Operators must run an orchestrator
service alongside it, e.g.

```sh
assistant orchestrator run --no-repl                # in another systemd unit
# (or: --interfaces slack,matrix to also handle messengers)
```

The orchestrator service supervises every space in `org.db` and runs its
workers (turn workers, scheduler, title generator, memory indexer) once per
space. It picks up new spaces and drops deleted ones every 30 seconds, so one
orchestrator handles Web, messenger and Scheduler turns for all spaces. A
messenger runs in the space its interface instance is bound to, or in the
default space if it has none. Running runtime workers in both processes simultaneously
causes per-second `claim failed` warnings as the two consumers race for
every bus claim.

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

## Theme

The Flutter app's theme is centralised in `app/lib/shared/theme/`. Use the
exported tokens instead of inline `Color(0x...)` / `Colors.X` literals or
ad-hoc `EdgeInsets`/`BorderRadius` numbers.

### Modules

- `assistant_colors.dart` — `AssistantColors.brand`, `accentSecondary`,
  `accentTertiary`, `warning`. The first three seed the Material 3 scheme's
  `primary` / `secondary` / `tertiary` slots. `warning` covers Material 3's
  missing warning role.
- `assistant_typography.dart` — `AssistantTypography.material()` builds the
  `TextTheme` with the Inter family applied; `AssistantTypography.mono` is
  a JetBrains Mono `TextStyle` for inline code, attribute keys, and log
  lines. Fonts are fetched lazily via `google_fonts` and preloaded in
  `main()` so they resolve on first paint.
- `assistant_spacing.dart` — `AssistantSpacing.xs|sm|md|lg|xl|xxl`
  (4/8/12/16/24/32 dp), `AssistantRadius.sm|md|lg` (8/12/16 dp), and
  `AssistantElevation.low|medium|high`.
- `assistant_theme.dart` — `assistantLightTheme()` and `assistantDarkTheme()`
  builders consumed by `AdaptiveApp`.

### Migrating a screen to tokens

1. Replace inline `EdgeInsets.all(12)` / `SizedBox(height: 8)` / similar
   literals with `AssistantSpacing.*`.
2. Replace `BorderRadius.circular(12)` / similar with `AssistantRadius.*`.
3. Replace `Colors.amber.shade50` etc. with `AssistantColors.*` (or
   `theme.colorScheme.X` where the value should follow the active theme).
4. Run `app/scripts/check_no_raw_colors.sh` — it fails on `Color(0x...)` /
   `Colors.X` outside `lib/shared/theme/`. `Colors.transparent` is allowed.
   Pre-existing offenders are listed in `app/scripts/theme_color_allowlist.txt`
   and should be migrated incrementally.

### Contrast

Unit tests under `app/test/unit/shared/theme/` check that the audited
foreground / background pairs meet WCAG AA on both light and dark themes
(body text >= 4.5:1, large text / pills >= 3:1). Add a case to
`contrast_test.dart` whenever a new colour pairing becomes load-bearing.

## CLI options

| Flag                             | Env var                             | Default               | Description                                                                                                              |
| -------------------------------- | ----------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `--auth-token`                   | `ASSISTANT_WEB_TOKEN`               | _(required)_          | Token used for Bearer auth                                                                                               |
| `--listen`                       |                                     | `127.0.0.1:8080`      | Address to bind                                                                                                          |
| `--cors-origin`                  | `ASSISTANT_WEB_CORS_ORIGIN`         | _(none: same-origin)_ | Allowed cross-origin `http(s)://host[:port]` origins; repeat the flag or comma-separate. An invalid value aborts startup |
| `--no-secure-cookie`             |                                     | `false`               | Disable `Secure` attribute on session cookies (see below)                                                                |
| `--auth-rate-limit-per-minute`   | `ASSISTANT_WEB_AUTH_RATE_LIMIT`     | `30`                  | Credential-endpoint requests per client IP per minute (see below)                                                        |
| `--register-rate-limit-per-hour` | `ASSISTANT_WEB_REGISTER_RATE_LIMIT` | `10`                  | Dynamic client registrations per client IP per hour                                                                      |
| `--login-max-failures`           | `ASSISTANT_WEB_LOGIN_MAX_FAILURES`  | `10`                  | Failed password attempts per account before lockout; one attempt is restored per minute                                  |
| `--trust-forwarded-for`          | `ASSISTANT_WEB_TRUST_FORWARDED_FOR` | `false`               | Take the client IP from the rightmost `X-Forwarded-For` entry (reverse proxy only)                                       |

### Rate limiting

`POST /login`, `/oauth/authorize`, `/oauth/token`, `/oauth/device`,
`/oauth/device/verify` and `/oauth/revoke` share one token bucket per client
IP. `POST /oauth/register` has its own, stricter bucket. IPv6 clients are
bucketed by /64 prefix. A rejected request gets `429 Too Many Requests` with a
`Retry-After` header.

`POST /oauth/authorize` also locks an account after `--login-max-failures`
failed attempts, whichever IP they come from. Unknown emails count the same
way, so the lockout does not reveal which accounts exist.

Behind a reverse proxy every request arrives from the proxy's address, so all
clients would share one bucket. Set `--trust-forwarded-for` only when the proxy
appends the real peer address to `X-Forwarded-For`; the server then uses the
rightmost entry. Without the flag the header is ignored, so a direct client
cannot choose its own bucket.

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
persona switching, and skills views.

### Persona starter prompts

A Persona can carry up to 4 starter prompts that the app shows as suggestions
in an empty chat. The Persona detail screen has a "Starter prompts" section
that lists them, and an editor (`/personas/{id}/starters`) to add, edit,
reorder and remove them. The editor saves through
`PATCH /api/personas/{id}` and checks the same limits as the API first: at
most 4 prompts, each non-blank and at most 200 characters after trimming
(characters are Unicode scalar values, not bytes).

Who may edit starters:

- Personal Persona: its owner.
- Org-owned Persona: a space admin.

Everyone else sees the prompts read-only; the section offers no edit action
(`can_edit_starter_prompts` is false on `GET /api/personas/{id}`).

There are no org-level fallback starters. A Persona without starters shows the
default empty-chat copy; an admin who wants starters everywhere sets them on
the org-owned Personas.

## REST API (`/api/*`)

These endpoints power the Flutter app and are also consumable directly:

| Route                                  | Method | Description                                    |
| -------------------------------------- | ------ | ---------------------------------------------- |
| `/api/conversations`                   | GET    | List conversations                             |
| `/api/conversations`                   | POST   | Create conversation                            |
| `/api/conversations/{id}`              | GET    | Get conversation with messages                 |
| `/api/conversations/{id}`              | PATCH  | Rename conversation                            |
| `/api/conversations/{id}`              | DELETE | Delete conversation                            |
| `/api/conversations/{id}/messages`     | POST   | Send message (SSE streaming response)          |
| `/api/personas`                        | GET    | List personas                                  |
| `/api/personas/active`                 | POST   | Switch active persona                          |
| `/api/personas/{id}`                   | PATCH  | Update a persona's starter prompts             |
| `/api/personas/{id}/skills`            | GET    | List skills for a persona                      |
| `/api/orgs`                            | GET    | List organizations (filtered by user access)   |
| `/api/orgs`                            | POST   | Create organization                            |
| `/api/orgs/{id}`                       | GET    | Get organization details                       |
| `/api/orgs/{id}`                       | PATCH  | Update organization settings                   |
| `/api/orgs/{org}/users`                | GET    | List users in org                              |
| `/api/orgs/{org}/users`                | POST   | Invite user to org                             |
| `/api/orgs/{org}/users/{uid}`          | GET    | Get user details                               |
| `/api/orgs/{org}/users/{uid}`          | PATCH  | Update user (name, role)                       |
| `/api/orgs/{org}/users/{uid}`          | DELETE | Remove user from org                           |
| `/api/orgs/{org}/spaces`               | GET    | List spaces (filtered by membership)           |
| `/api/orgs/{org}/spaces`               | POST   | Create space                                   |
| `/api/orgs/{org}/spaces/{sid}`         | GET    | Get space details                              |
| `/api/orgs/{org}/spaces/{sid}`         | PATCH  | Update space                                   |
| `/api/orgs/{org}/spaces/{sid}`         | DELETE | Delete space (org-admin only)                  |
| `/api/orgs/{org}/spaces/{sid}/members` | GET    | List space members                             |
| `/api/orgs/{org}/spaces/{sid}/members` | POST   | Add user to space with role                    |
| `/api/users/me/api-keys`               | GET    | List API keys (prefix + name, no secrets)      |
| `/api/users/me/api-keys`               | POST   | Create scoped API key (returns plaintext once) |
| `/api/users/me/api-keys/{kid}`         | DELETE | Revoke API key                                 |

Full OpenAPI spec is served at `/api/openapi.json`; Swagger UI at `/api/docs`.

## OAuth2 endpoints (`/oauth/*`)

These endpoints implement the assistant's OAuth2 Authorization Server.
See [authentication.md](authentication.md) for flow details.

| Route                                     | Method   | Description                                                            |
| ----------------------------------------- | -------- | ---------------------------------------------------------------------- |
| `/oauth/register`                         | POST     | Dynamic client registration (RFC 7591)                                 |
| `/oauth/authorize`                        | GET      | Render the email form, the password form, or redirect to the org's IdP |
| `/oauth/authorize/identify`               | POST     | Find the org an email signs in to                                      |
| `/oauth/authorize`                        | POST     | Validate one org's credentials, redirect with auth code                |
| `/oauth/token`                            | POST     | Exchange auth code, refresh token, or device code                      |
| `/oauth/device`                           | POST     | Initiate device code flow (RFC 8628)                                   |
| `/oauth/device/verify`                    | GET/POST | User enters device code, org slug, and credentials                     |
| `/oauth/callback`                         | GET      | Callback from the org's OIDC IdP (server as OIDC client)               |
| `/oauth/complete`                         | GET      | Auth code delivery as JSON (Flutter web)                               |
| `/oauth/revoke`                           | POST     | Token revocation (RFC 7009)                                            |
| `/.well-known/oauth-authorization-server` | GET      | Server metadata (RFC 8414)                                             |

## Server-side management pages

| Route                    | Description                                               |
| ------------------------ | --------------------------------------------------------- |
| `/agents`                | A2A Profile management — list, create, edit, delete       |
| `/agents/{id}/card.json` | Raw A2A Profile card JSON                                 |
| `/webhooks`              | Webhook management — list, create, toggle, rotate secrets |

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
`crates/web-ui/build.rs` when Cargo compiles the crate (it never
runs Flutter; `make build-web` produces `app/build/web/`, and a placeholder
page is embedded if that directory is missing).

```
assistant webui serve
├── build.rs            # Embeds app/build/web/ (or a placeholder page) at compile time
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
