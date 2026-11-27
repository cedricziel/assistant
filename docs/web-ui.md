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

**Streaming across the process boundary.** The chat stream (`POST
/api/conversations/{id}/messages`) works with the orchestrator in a separate
process. The turn runs in the orchestrator, which publishes each event of the
run on the bus topic `turn.event` (keyed by the run id); the web process reads
them and forwards them to the browser, so tokens, tool results and the final
`done` frame arrive as they do when both run in one process. Streaming text
(`token` and `thinking`) is batched to at most one bus message per 200 ms per
run, so the browser sees text in short bursts. Both processes must open the
same `~/.assistant/` data directory and message bus. Run them from the same
release: the event format is not versioned.

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

## Conversation list

The sidebar lists the caller's conversations for the active Persona, newest
first. Tapping a row opens the conversation. A new chat starts untitled and the
title worker names it after the first turns unless you rename it first. The list
updates live through `GET /api/conversations/stream`: renames, pins, archives
and new messages reach other tabs and devices without a reload.

### Rename, pin and archive

Row actions are available from the row's context menu or action sheet:

- **Rename** opens a dialog prefilled with the current title. The title is
  trimmed and must be 1 to 200 characters. Renaming locks the title, so the
  title worker will not replace it later.
- **Pin** / **Unpin** — pinned conversations sit above the rest under a
  "Pinned" header, newest pin first. Re-pinning keeps the original pin time.
- **Archive** / **Unarchive** — archived conversations leave the main list and
  the stream snapshot. They appear in a collapsed "Archived" section below the
  list, most recently archived first, and search still finds them with an
  "Archived" badge.
- **Delete** permanently deletes the conversation.

Pin and archive are exclusive. Pinning an archived conversation unarchives it
and pins it; archiving a pinned conversation unpins it. Sending a new user
message unarchives the conversation, while an assistant reply leaves it
archived.

Renaming, pinning, archiving and auto-titling do not change a conversation's
recency. Only a new message bumps `updated_at`, so metadata edits do not
reorder the list.

### Row gestures

| Input                                     | Presentation |
| ----------------------------------------- | ------------ |
| Right-click a row, or the overflow button | Context menu |
| Long-press a row on touch                 | Action sheet |

The overflow button (labelled "Conversation actions") appears while a row is
hovered, focused or selected. Keyboard activation follows the platform: a menu
on desktop and web, an action sheet on iOS and Android. The only swipe is
end-to-start, which deletes after confirmation; pin and archive are offered in
the menu and sheet only. Rows in the "Archived" section offer the same actions,
where Pin means unarchive and pin.

### Search

The search field above the list replaces the list with results while the query
is non-empty. Each result shows the title with matches highlighted and up to
three matching message excerpts. Selecting a result opens the conversation and
clears the query; Esc clears the field.

Search rules:

- The query is split on whitespace into at most 8 terms.
- Every term must appear somewhere in the conversation, but not necessarily in
  the same message. One term can match the title and another a later message.
- A term matches as a prefix of a word in message text (`lisb` matches
  "Lisbon") and as a substring of the title.
- Titles and messages fold case and diacritics together, so `arzte` matches
  "Ärztetermin" and `lisboa` matches "Lisbôa". FTS5 operators such as `OR` and
  `*` are ordinary text.
- One-character terms match titles only. When the query also has a term of two
  or more characters, one-character terms do not narrow the result.
- Only `user` and `assistant` messages are searched. Thinking steps and tool
  output are never matched.
- A conversation with a title match ranks above one that matched only in a
  message. A conversation appears at most once, with at most 3 message matches.
  The API returns 20 results by default and caps `limit` at 50.

Search covers archived conversations too. Results are scoped to the caller and
the active Persona, like the list.

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

### Suggestions and starter prompts

#### Follow-up chips

After a turn finishes, the server can propose up to three follow-up prompts.
The app shows them as chips above the composer, below the reconnect banner and
the turn progress card. A tap sends the prompt as the next message. The chips
disappear as soon as a message is sent or queued, while a turn is running, and
when the conversation changes. A conversation that is reopened later shows no
chips: suggestions belong to the live turn only.

The chips arrive on the finished run's replay stream (`GET
/api/conversations/{id}/runs/{run}/events/stream?since=N+1`) as a `suggestions`
frame that follows `done`. The app listens for at most 20 seconds, which is
the server's 15 second ceiling plus latency, and shows nothing when no frame
arrives. An org turns the feature off with `enabled = false` in the
`[suggestions]` section of `org.toml`; `max_suggestions` (1 to 5, default 3)
and `timeout_secs` (1 to 15, default 10) tune it.

#### Persona starter prompts

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

### Rating replies

Every finished assistant reply carries thumbs up and thumbs down in its action
row. The buttons appear once the reply has content and a server-assigned id,
so a streaming reply or one still on the client-only `assistant-<timestamp>`
fallback has none. A reply the user already rated shows the matching thumb
selected when the conversation loads.

Choosing either thumb opens an optional comment field below the actions.
Submitting sends the rating with the trimmed comment; dismissing keeps the
rating without a comment. Choosing the other thumb sends the new rating
without the old comment and reopens the field empty. Tapping the selected
thumb clears the vote and closes the field.

The app updates the selection immediately and reconciles with the server
afterwards. A failed request rolls the reply back to the last confirmed vote
and shows a snackbar. While requests for a reply are queued, its local vote
wins over a history reload.

Comment text stays in the space database. It is never exported to telemetry;
the `gen_ai.evaluation.result` event only records whether a comment exists
(`assistant.feedback.has_comment`). See
[opentelemetry.md](opentelemetry.md) for the event and metric reference.

### Response timing

The app can show how long a reply took. It is **off by default**; turn it on
per device under **Settings → Chat → Show response timing**. The switch is a
local preference (`assistant.showResponseTiming` in local storage); it never
reaches the server, so every browser and app install keeps its own setting.

With the switch on, a finished reply that has timing shows a small speed icon in
its action row. Hover, focus or activate it to see the numbers. They are
measured by the orchestrator, not the browser:

- **First token** — time from the start of the turn to the first streamed token.
  Absent when the turn produced no streaming tokens.
- **Total** — time from the start of the turn until the final answer was saved.
- **tok/s** — output tokens divided by decode time. Decode time is the time
  spent streaming tokens and excludes time spent in tool calls. tok/s is shown
  only when the provider reported a token count and decoding took at least
  250 ms.

Some OpenAI-compatible providers omit usage for streamed responses unless
`stream_options.include_usage` is set. Those replies still show first-token and
total time but no tok/s. Timing is stored per message, so replies that predate
the feature, older conversations and turns from Slack or Mattermost show none.
Cancelled and failed turns have no timing either.

### Accessibility

The chat surface targets WCAG AA. A guideline suite
(`app/test/widget/features/chat/chat_accessibility_test.dart`) pumps the chat
screen with an attachment, a code block, a tool call, a thinking entry and an
error banner in both themes, and asserts labelled tap targets, text contrast and
a label on every tappable node.

**Screen-reader announcements.** Turn lifecycle events are spoken through the
platform accessibility channel; token frames never are:

- a reply starts: "{Persona} is responding";
- it finishes: "{Persona} replied: {excerpt}", where the excerpt is the reply
  as plain text, cut to 200 characters at a word boundary and followed by "…"
  when cut;
- the user stops it: "Response stopped";
- it fails: "Response failed: {message}", spoken immediately;
- the turn goes quiet for a while: "Still working", at most once per turn, and
  not while the turn waits for the user (for example a tool approval);
- a code block is copied: "Code copied".

A turn that is resumed after a dropped connection is announced the same way when
its reply arrives. Opening a conversation's history is silent. Polite
announcements are held for up to 1.5 seconds and a burst collapses to the latest
one, so "Ada is responding" is skipped when the answer arrives immediately.
The streaming bubble is not a live region, so TalkBack and VoiceOver do not
re-read it on every token. The error banner is a live region.

**Keyboard.** The composer follows visual order: attach, voice, input, then send
or stop. Enter sends; Escape closes the slash-command menu. Timeline entry
headers are focusable and toggle on Enter or Space, and a code block's copy
button is reachable with Tab and activates with Enter or Space. The response
timing indicator is a button too; focusing it opens its tooltip.

**Reduced motion.** With the platform's reduce-motion setting on, the thread
jumps instead of animating when it scrolls to the bottom, timeline entries
resize without animating, the typing indicator is static, and the code block's
"Copied" feedback appears in the next frame. Indeterminate progress indicators
may keep animating because they convey status.

**Layout.** Navigation switches at 768 px of window width. The chat pane splits
into conversation list and messages only when the pane itself is at least
768 px wide; otherwise the list lives in the drawer. With the navigation
sidebar expanded, the list is in the drawer for windows up to 1008 px wide;
with it collapsed, up to 840 px.

## REST API (`/api/*`)

These endpoints power the Flutter app and are also consumable directly:

| Route                                  | Method | Description                                               |
| -------------------------------------- | ------ | --------------------------------------------------------- |
| `/api/conversations`                   | GET    | List conversations (`?archived=true` lists archived ones) |
| `/api/conversations`                   | POST   | Create conversation                                       |
| `/api/conversations/stream`            | GET    | Stream conversation list changes (SSE)                    |
| `/api/conversations/search`            | GET    | Search titles and message text (`?q=…&limit=…`)           |
| `/api/conversations/{id}`              | GET    | Get conversation with messages                            |
| `/api/conversations/{id}`              | PATCH  | Update title, pin or archive state                        |
| `/api/conversations/{id}`              | DELETE | Delete conversation                                       |
| `/api/conversations/{id}/messages`     | POST   | Send message (SSE streaming response)                     |
| `/api/personas`                        | GET    | List personas                                             |
| `/api/personas/active`                 | POST   | Switch active persona                                     |
| `/api/personas/{id}`                   | PATCH  | Update a persona's starter prompts                        |
| `/api/personas/{id}/skills`            | GET    | List skills for a persona                                 |
| `/api/orgs`                            | GET    | List organizations (filtered by user access)              |
| `/api/orgs`                            | POST   | Create organization                                       |
| `/api/orgs/{id}`                       | GET    | Get organization details                                  |
| `/api/orgs/{id}`                       | PATCH  | Update organization settings                              |
| `/api/orgs/{org}/users`                | GET    | List users in org                                         |
| `/api/orgs/{org}/users`                | POST   | Invite user to org                                        |
| `/api/orgs/{org}/users/{uid}`          | GET    | Get user details                                          |
| `/api/orgs/{org}/users/{uid}`          | PATCH  | Update user (name, role)                                  |
| `/api/orgs/{org}/users/{uid}`          | DELETE | Remove user from org                                      |
| `/api/orgs/{org}/spaces`               | GET    | List spaces (filtered by membership)                      |
| `/api/orgs/{org}/spaces`               | POST   | Create space                                              |
| `/api/orgs/{org}/spaces/{sid}`         | GET    | Get space details                                         |
| `/api/orgs/{org}/spaces/{sid}`         | PATCH  | Update space                                              |
| `/api/orgs/{org}/spaces/{sid}`         | DELETE | Delete space (org-admin only)                             |
| `/api/orgs/{org}/spaces/{sid}/members` | GET    | List space members                                        |
| `/api/orgs/{org}/spaces/{sid}/members` | POST   | Add user to space with role                               |
| `/api/users/me/api-keys`               | GET    | List API keys (prefix + name, no secrets)                 |
| `/api/users/me/api-keys`               | POST   | Create scoped API key (returns plaintext once)            |
| `/api/users/me/api-keys/{kid}`         | DELETE | Revoke API key                                            |

`PATCH /api/conversations/{id}` takes optional `title`, `pinned` and
`archived` fields; at least one is required. A title is trimmed and limited to
200 characters. Every 4xx response from this route uses the `{"error": "…"}`
envelope. List and search results omit `pinned_at` and `archived_at` when
unset.

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
