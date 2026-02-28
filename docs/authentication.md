# Authentication

The web UI requires a single shared token for all access.  There is no
multi-user support yet — the token acts as a password that gates both
the browser UI and API endpoints.

## Configuration

Set the token via CLI flag or environment variable:

```sh
# Environment variable
ASSISTANT_WEB_TOKEN=my-secret-token cargo run -p assistant-web-ui

# CLI flag
cargo run -p assistant-web-ui -- --auth-token my-secret-token
```

The server **refuses to start** if no token is configured.  Whitespace
is trimmed automatically.

## Browser flow

1. Unauthenticated requests to any protected page redirect to `/login`.
2. Enter the token on the login page.
3. On success, the server sets an `assistant_session` cookie and
   redirects to the dashboard.
4. The cookie is `HttpOnly`, `SameSite=Strict`, and valid for 7 days.
5. When the server binds to a non-loopback address, the `Secure`
   attribute is added so the cookie is only sent over HTTPS.
   Pass `--no-secure-cookie` to disable this if plain HTTP is acceptable.
6. Sign out via the sidebar button (`POST /logout`), which clears the
   cookie.

### Session cookie details

The cookie value is an HMAC-SHA256 digest derived from the auth token —
it never contains the raw token.  Verification is constant-time.

```
cookie = hex(HMAC-SHA256(key=auth_token, msg="assistant-web-session-v1"))
```

Since the session value is deterministic, restarting the server with the
same token preserves existing sessions.

## API / A2A flow

Machine callers authenticate with a Bearer token in the `Authorization`
header:

```sh
curl -H "Authorization: Bearer my-secret-token" http://localhost:8080/tasks
```

Unauthenticated API requests receive `401 Unauthorized` with a
`WWW-Authenticate: Bearer` header.

## Route protection

| Route | Auth required |
|-------|---------------|
| `/login` (GET, POST) | No |
| `/logout` (POST) | No |
| `/.well-known/agent.json` | No (public per A2A spec) |
| Everything else | Yes |

The `/.well-known/agent.json` endpoint is intentionally public so that
A2A callers can discover the authentication requirements before making
authenticated requests.

## Auto-hardening

At startup, the web UI injects a `bearer_token` security scheme into the
A2A agent card.  This means the public agent card at
`/.well-known/agent.json` advertises:

```json
{
  "securitySchemes": {
    "bearer_token": {
      "httpAuthSecurityScheme": {
        "scheme": "Bearer",
        "description": "Bearer token authentication. Pass the token via Authorization: Bearer <token>."
      }
    }
  },
  "securityRequirements": [
    { "bearer_token": [] }
  ]
}
```

A2A clients can use this to know they need to present a Bearer token
before calling any protected endpoint.

## Security notes

- The token is compared using constant-time equality to prevent timing
  attacks.
- The session cookie uses HMAC-SHA256 derivation — the raw token is
  never stored in the cookie.
- `SameSite=Strict` prevents CSRF via cross-origin requests.
- `HttpOnly` prevents JavaScript access to the cookie.
- `Secure` is added automatically when binding to a non-loopback address
  (override with `--no-secure-cookie` for plain HTTP behind a VPN).
- A warning is logged when binding to a non-loopback address to flag
  unintentional network exposure.
