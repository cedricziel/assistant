# Authentication

The assistant web UI supports two authentication modes: a **legacy
single-token** mode for quick local setups, and a full **OAuth2 / API key**
system for multi-user deployments. The server auto-detects which mode to
use based on configuration.

## Quick start (single-token mode)

Set a shared token via CLI flag or environment variable:

```sh
ASSISTANT_WEB_TOKEN=my-secret-token assistant webui serve
# or
assistant webui serve --auth-token my-secret-token
```

All users share the same token. This is the simplest setup and is
appropriate for single-user local development.

### Browser flow

1. Unauthenticated requests redirect to `/login`.
2. Enter the token on the login page.
3. On success, the server sets an `assistant_session` cookie and
   redirects to the dashboard.
4. Sign out via `POST /logout`, which clears the cookie.

### API flow

```sh
curl -H "Authorization: Bearer my-secret-token" http://localhost:8080/api/personas
```

## First start: the initial admin

When the default organization uses password authentication and has no
users, `assistant webui serve` creates one on startup:

- Email: `admin@localhost`, with the `OrgAdmin` role in the default space.
- Password: 24 random alphanumeric characters, written to
  `~/.assistant/orgs/default/admin_credentials.txt` (mode `0600`).

The server logs the path of that file, never the password. Sign in, change
the password with `assistant account change-password`, then delete the file.
Later starts find a user and do nothing. Organizations in OIDC mode get no
initial admin; users are provisioned on first sign-in.

### Reset a password locally

If nobody can sign in, run this as the OS user that owns `~/.assistant`:

```sh
assistant admin reset-password admin@localhost
# another organization:
assistant admin reset-password someone@example.com --org acme
```

The command prompts for the new password twice, writes it to `org.db`
directly, and signs the user out everywhere. It refuses users managed by an
external identity provider.

## Multi-user mode (OAuth2 + API keys)

When an `org.db` database exists and users are registered, the server
runs in multi-user mode with full OAuth2 support.

### OAuth2 endpoints

| Method | Path                                      | Description                                                         |
| ------ | ----------------------------------------- | ------------------------------------------------------------------- |
| POST   | `/oauth/register`                         | Dynamic client registration (RFC 7591)                              |
| GET    | `/oauth/authorize`                        | Authorization endpoint (email form, password form, or IdP redirect) |
| POST   | `/oauth/authorize/identify`               | Find the org an email signs in to                                   |
| POST   | `/oauth/authorize`                        | Submit org + credentials, issue authorization code                  |
| GET    | `/oauth/callback`                         | OIDC IdP callback                                                   |
| POST   | `/oauth/token`                            | Token exchange (auth code, refresh, device code)                    |
| POST   | `/oauth/device`                           | Initiate device authorization (RFC 8628)                            |
| GET    | `/oauth/device/verify`                    | User-facing code entry page                                         |
| POST   | `/oauth/device/verify`                    | Submit device code + org slug + credentials                         |
| POST   | `/oauth/revoke`                           | Token revocation (RFC 7009)                                         |
| GET    | `/.well-known/oauth-authorization-server` | Server metadata (RFC 8414)                                          |

### Grant types

| Grant type                                     | Use case                               |
| ---------------------------------------------- | -------------------------------------- |
| `authorization_code`                           | Browser-based apps (Flutter web/macOS) |
| `urn:ietf:params:oauth:grant-type:device_code` | CLI and headless devices               |
| `refresh_token`                                | Silent token renewal                   |

### Sign-in flow

Emails are unique per org, not per server, so every sign-in resolves an org
first. `GET /oauth/authorize` does it one of two ways:

- **With `?org=<slug>`**, it goes straight to that org. An org in OIDC mode
  redirects to its IdP. A password org shows the password form with the org
  fixed. An unknown slug is a 400.
- **Without `org`**, it shows an email form that posts to
  `POST /oauth/authorize/identify`. If exactly one account has that email,
  the flow continues in that account's org: an OIDC org redirects to its IdP
  with the email as `login_hint`, and a password org shows the password form.
  If no account matches, or accounts in several orgs match, the page shows
  the same "enter the name of your organization" form. That form continues at
  `GET /oauth/authorize?org=<slug>&login_hint=<email>`.

`POST /oauth/authorize` requires the `org` field and looks the user up in
that org only. An org in OIDC mode refuses it with a 400. The failed-login
lockout counts per org and email, so the same address in two orgs has two
counters.

`POST /oauth/device/verify` takes an org slug the same way, so the device
flow signs in to one org's password accounts. Accounts in an OIDC org have no
local password and cannot use it.

The identify step tells a caller whether an address has an account and, for
one match, which sign-in method its org uses. It shares the per-IP auth rate
limit (`--auth-rate-limit-per-minute`, env `ASSISTANT_WEB_AUTH_RATE_LIMIT`,
default 30) with the other credential endpoints. See
[ADR-0016](adr/adr-0016-per-org-oidc.md) for why this is accepted.

### Per-org OIDC

OIDC is configured per org, not per server. Each org can point at its own IdP
(Keycloak, Auth0, Google Workspace, and so on) or stay on passwords. The
server still issues its own JWTs after the IdP login.

Org admins manage the settings with these endpoints. The caller must be an
admin of the org in the path.

| Method | Path                  | Description                                               | Status codes            |
| ------ | --------------------- | --------------------------------------------------------- | ----------------------- |
| GET    | `/api/orgs/{id}/oidc` | Read the settings                                         | 200, 401, 403, 404      |
| PUT    | `/api/orgs/{id}/oidc` | Create or replace the settings                            | 200, 400, 401, 403, 404 |
| DELETE | `/api/orgs/{id}/oidc` | Remove the settings. Refused while `auth_mode` is `oidc`. | 204, 401, 403, 404, 409 |

`PUT` body fields:

| Field                   | Notes                                                                                                                                                                                                                        |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `issuer_url`            | `https`, or `http` for `localhost` and loopback addresses. Stored as sent, minus surrounding whitespace. Discovery compares it byte for byte with the `issuer` in the IdP's discovery document, so a trailing slash matters. |
| `client_id`             | Required.                                                                                                                                                                                                                    |
| `client_secret`         | Write-only. Omit it to keep the stored secret, send `null` to clear it. Public clients leave it unset.                                                                                                                       |
| `auto_provision`        | Defaults to `true`. When `false`, only users who already exist can sign in.                                                                                                                                                  |
| `allowed_email_domains` | Bare domains such as `example.com`, lowercased on save. Empty allows any domain.                                                                                                                                             |

Responses carry `has_client_secret` and never the secret. The server stores
the secret sealed with its `SecretBox` key (`secrets.key` or
`ASSISTANT_SECRETS_KEY`), the same key that protects integration credentials.
Restoring `org.db` without that key leaves the secret unreadable. Set it again
with `PUT`.

`PATCH /api/orgs/{id}` accepts `auth_mode` values `password` and `oidc`.
Switching to `oidc` returns 409 until the org has OIDC settings. `POST
/api/orgs` creates `password` orgs only. For the steps, see
[Set up OIDC sign-in for an org](operations/org-oidc-sign-in.md).

Register this redirect URI at the IdP: `{server base URL}/oauth/callback`.
The server asks for the scopes `openid email profile`.

How the server handles the IdP:

- It discovers each org's IdP on first use and caches the result until the
  org's settings change. A failed discovery is not cached, so the next sign-in
  retries. The IdP's signing keys (JWKS) are fetched once per discovery. After
  key rotation at the IdP, update the org's settings or restart the server.
- The callback signs the user in to the org that started the login, never
  another. It looks the user up in that org by (issuer, subject). If none
  matches, it looks up the email. Linking an existing account by email
  requires `email_verified: true` in the ID token, otherwise the login fails.
  If there is still no match and `auto_provision` is on, it creates the user.
- The email-domain allow-list is checked first and ignores case. A token
  without an email fails when an allow-list is set.
- If the org's discovery fails at sign-in, the user sees a 502. An org in
  `oidc` mode with no settings returns a 500.

### Token format

Access tokens are **HS256 JWTs** signed by the server. Claims include:

```json
{
  "sub": "user_abc123",
  "org_id": "org_1",
  "email": "alice@example.com",
  "spaces": { "eng": "member", "ops": "admin" },
  "scope": "personas:read conversations:write",
  "exp": 1719878400
}
```

Default TTL is **1 hour**. Refresh tokens are opaque base64url strings
stored server-side with a configurable TTL (default 30 days).

### CLI authentication

The CLI uses the **device code flow** (RFC 8628):

```sh
# Log in — opens browser for authorization
assistant login http://localhost:8080

# Check status
assistant status

# Log out — revokes tokens and removes credentials
assistant logout
```

Credentials are stored in `~/.assistant/credentials.json` (mode `0600`).
The CLI automatically refreshes expired access tokens using the stored
refresh token.

#### Non-interactive authentication

For CI/CD and scripts, use an API key instead of the device code flow:

```sh
# Via flag
assistant --api-key ask_live_... --server http://localhost:8080 api-keys list

# Via environment variables
export ASSISTANT_API_KEY=ask_live_...
export ASSISTANT_SERVER=http://localhost:8080
assistant api-keys list
```

## API keys

API keys provide scoped, long-lived authentication for machine callers.

### Key format

Keys use the prefix `ask_live_` followed by 43 base64url characters:

```
ask_live_Ab3xY9kLm2nPq4rStUvWx5yZ...
```

Only the SHA-256 hash is stored server-side. The plaintext is shown
**once** at creation time and cannot be recovered.

### Managing keys

Via CLI (requires login):

```sh
# Create a key
assistant api-keys create --name "CI deploy" --scopes "conversations:read,skills:execute"

# List keys (shows prefix, name, scopes — not the secret)
assistant api-keys list

# Revoke a key
assistant api-keys revoke key_abc123
```

Via API:

```sh
# Create
curl -X POST http://localhost:8080/api/users/me/api-keys \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "CI", "scopes": ["conversations:read"]}'

# List
curl http://localhost:8080/api/users/me/api-keys \
  -H "Authorization: Bearer $TOKEN"

# Revoke
curl -X DELETE http://localhost:8080/api/users/me/api-keys/key_abc123 \
  -H "Authorization: Bearer $TOKEN"
```

### Scopes

Scopes follow the `resource:action` format:

| Resource        | Actions                              |
| --------------- | ------------------------------------ |
| `personas`      | `read`, `write`, `delete`            |
| `conversations` | `read`, `write`, `delete`            |
| `messages`      | `read`, `write`                      |
| `skills`        | `read`, `write`, `delete`, `execute` |
| `interfaces`    | `read`, `write`, `manage`            |
| `bindings`      | `read`, `write`                      |
| `users`         | `read`, `write`, `manage`            |
| `org`           | `read`, `manage`                     |
| `api_keys`      | `read`, `write`                      |
| `spaces`        | `read`, `write`, `manage`            |

API keys can optionally restrict scopes to specific resource IDs
(e.g., only a specific persona or space).

## Changing your account

Authenticated users can update their own name, email, and password without
involving an admin. Three endpoints implement this; all are mounted under
`/api` and require a valid bearer token (JWT or API key).

| Method | Path                     | Description             | Status codes                                          |
| ------ | ------------------------ | ----------------------- | ----------------------------------------------------- |
| GET    | `/api/users/me`          | Get own user record     | 200, 401                                              |
| PATCH  | `/api/users/me`          | Update own name / email | 200, 400 (invalid email), 401, 409 (collision / OIDC) |
| POST   | `/api/users/me/password` | Change own password     | 204, 400 (empty new), 401 (wrong current), 409 (OIDC) |

### Update name or email

```sh
curl -X PATCH http://localhost:8080/api/users/me \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"email": "new@example.com"}'
```

The response is a `UpdateCurrentUserResponse`:

```json
{
  "user": {
    "id": "...",
    "email": "new@example.com",
    "name": "...",
    "...": "..."
  },
  "previous_email": "old@example.com"
}
```

`previous_email` is only present when the email actually changed. Email
collisions (another user in the same org already has that address) return
`409 Conflict`. Empty body, or body with values matching current state, is
a no-op that returns `200 OK` without writing to storage.

### Change password

```sh
curl -X POST http://localhost:8080/api/users/me/password \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"current_password": "old-pw", "new_password": "new-pw"}'
```

On success the response is `204 No Content`.

**Refresh-token revocation.** A successful password change revokes **every
refresh token** belonging to the user. Sessions on other browsers and
devices stop being able to refresh and must re-log-in. The calling access
token (the JWT in the `Authorization` header) keeps working until its
natural expiry — typically 1 hour — so the user is not abruptly logged
out of the session that performed the change.

**API keys are not affected.** Long-lived `ask_live_*` keys continue to
authenticate after a password change. To revoke an API key, use the
explicit revoke endpoint (`DELETE /api/users/me/api-keys/{id}`).

### CLI usage

The `assistant account` subcommand wraps the same three endpoints:

```sh
# Show your current profile (email, name, org, auth mode).
assistant account show
assistant account show --json   # raw UserDetail JSON

# Update name or email.
assistant account set-name "New Name"
assistant account set-email new@example.com

# Change password — prompts for current + new + confirm via TTY.
assistant account change-password
```

`change-password` reads passwords from the TTY via `rpassword` so they
never appear in shell history. It rejects locally if the new password and
confirmation don't match.

### OIDC-managed accounts

When a user's organization runs in OIDC mode (`auth_mode = "oidc"`),
identity is owned by the identity provider that org configured (see
[Per-org OIDC](#per-org-oidc)). The PATCH and
password-change endpoints return:

```http
HTTP/1.1 409 Conflict
Content-Type: application/json

{"error": "account managed by identity provider <issuer>"}
```

`<issuer>` is taken from the user's `idp_issuer` claim when available,
otherwise a generic phrase. `GET /api/users/me` works in both modes — the
read path is never blocked, so OIDC users can still discover their own
profile.

## Auth middleware resolution order

On every request, the auth middleware resolves identity in this order:

1. Extract `Authorization: Bearer <token>` header.
2. Try **JWT validation** — if valid, extract `AuthContext` from claims.
3. If the token starts with `ask_live_`, try **API key resolution** —
   look up the hash, build `AuthContext` from the key's stored metadata.
4. If a legacy `--auth-token` is configured and the token matches,
   return the legacy context (backward compatibility).
5. Return `401 Unauthorized` if all checks fail.

Org admins bypass all scope checks. Other users are gated by their
space roles and the scopes attached to their token or API key.

## Session cookie details

On successful token exchange (auth code or refresh), the server sets:

```
assistant_session=<JWT>; HttpOnly; SameSite=Lax; Path=/; Max-Age=3600
```

- `HttpOnly` prevents JavaScript access.
- `SameSite=Lax` prevents CSRF via cross-origin POST. As a second layer,
  state-changing requests (including `POST /login` and `POST /logout`)
  must carry an `Origin` or `Referer` that matches `Host`. Only requests
  authenticated by a validated `Authorization: Bearer` token skip this check.
- `Secure` is added when binding to a non-loopback address (override
  with `--no-secure-cookie` for plain HTTP behind a VPN).

## Security notes

- JWT signing key is auto-generated on first run and persisted in
  `~/.assistant/jwt_key.json`. Back it up for session continuity
  across restarts.
- API key plaintext is shown once at creation — store it securely.
- Password hashing uses Argon2id.
- Token comparison uses constant-time equality (legacy token, PKCE verifier).
- PKCE is mandatory and `S256`-only: `/oauth/authorize` rejects `plain` and
  a missing `code_challenge_method`.
- The device code flow uses short-lived codes (15 min) with
  rate-limited polling (5s interval).

## Route protection

| Route                                     | Auth required              |
| ----------------------------------------- | -------------------------- |
| `/login` (GET, POST)                      | No                         |
| `/logout` (POST)                          | No                         |
| `/oauth/*`                                | No (OAuth2 flow endpoints) |
| `/workflow-hooks/{id}/{token}`            | No (HMAC token in URL)     |
| `/.well-known/agent.json`                 | No (A2A discovery)         |
| `/.well-known/oauth-authorization-server` | No (RFC 8414)              |
| Everything else                           | Yes                        |
