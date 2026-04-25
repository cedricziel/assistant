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

## Multi-user mode (OAuth2 + API keys)

When an `org.db` database exists and users are registered, the server
runs in multi-user mode with full OAuth2 support.

### OAuth2 endpoints

| Method | Path                                      | Description                                                |
| ------ | ----------------------------------------- | ---------------------------------------------------------- |
| POST   | `/oauth/register`                         | Dynamic client registration (RFC 7591)                     |
| GET    | `/oauth/authorize`                        | Authorization endpoint (renders login or redirects to IdP) |
| POST   | `/oauth/authorize`                        | Submit credentials, issue authorization code               |
| GET    | `/oauth/callback`                         | OIDC IdP callback                                          |
| POST   | `/oauth/token`                            | Token exchange (auth code, refresh, device code)           |
| POST   | `/oauth/device`                           | Initiate device authorization (RFC 8628)                   |
| GET    | `/oauth/device/verify`                    | User-facing code entry page                                |
| POST   | `/oauth/device/verify`                    | Submit device code + credentials                           |
| POST   | `/oauth/revoke`                           | Token revocation (RFC 7009)                                |
| GET    | `/.well-known/oauth-authorization-server` | Server metadata (RFC 8414)                                 |

### Grant types

| Grant type                                     | Use case                               |
| ---------------------------------------------- | -------------------------------------- |
| `authorization_code`                           | Browser-based apps (Flutter web/macOS) |
| `urn:ietf:params:oauth:grant-type:device_code` | CLI and headless devices               |
| `refresh_token`                                | Silent token renewal                   |

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
- `SameSite=Lax` prevents CSRF via cross-origin POST.
- `Secure` is added when binding to a non-loopback address (override
  with `--no-secure-cookie` for plain HTTP behind a VPN).

## Security notes

- JWT signing key is auto-generated on first run and persisted in
  `~/.assistant/jwt_key.json`. Back it up for session continuity
  across restarts.
- API key plaintext is shown once at creation — store it securely.
- Password hashing uses Argon2id.
- Token comparison uses constant-time equality.
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
