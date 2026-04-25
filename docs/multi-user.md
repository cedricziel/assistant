# Multi-User Setup

This guide covers the organization, space, and user model that enables
multi-user deployments of the assistant.

## Data model

```
Organization (org)
├── Users
│   ├── alice (OrgAdmin)
│   └── bob (Member)
├── Spaces
│   ├── engineering
│   │   ├── Members: alice (SpaceAdmin), bob (Member)
│   │   ├── Personas: code-reviewer, qa-bot
│   │   └── Conversations (scoped to space + persona + user)
│   └── marketing
│       ├── Members: alice (Viewer)
│       ├── Personas: content-writer
│       └── Conversations
└── Catalog
    ├── Published skills
    ├── Published templates
    └── Interface bindings
```

### Organizations

An organization is the top-level tenant. Each deployment typically has
one organization. All users, spaces, and resources belong to an org.

### Users

Users authenticate via OAuth2 (browser or device code flow) or API keys.
Each user has:

- **User ID** — unique identifier (e.g. `user_abc123`)
- **Email** — used for login and display
- **Password hash** — Argon2id (for password-based auth)
- **Org membership** — which org they belong to

### Spaces

Spaces are isolated workgroups within an organization. They scope:

- **Personas** — each persona lives in a space
- **Conversations** — scoped to space + persona + user
- **Skills** — access controlled per space via the catalog

Spaces enable teams to have separate contexts without cross-contamination.

### Roles

Roles form a hierarchy (highest to lowest):

| Role         | Level | Capabilities                                                                         |
| ------------ | ----- | ------------------------------------------------------------------------------------ |
| `OrgAdmin`   | 3     | Full access to all spaces, user management, org settings. Bypasses all scope checks. |
| `SpaceAdmin` | 2     | Manage space memberships, personas, and settings within their space.                 |
| `Member`     | 1     | Use personas, create conversations, execute skills within their space.               |
| `Viewer`     | 0     | Read-only access to conversations and personas in their space.                       |

A user can have different roles in different spaces. For example, Alice
might be `SpaceAdmin` in engineering but `Viewer` in marketing.

### Scopes and permissions

Every authenticated request carries an `AuthContext` with:

- The user's identity (`user_id`, `org_id`, `email`)
- A map of `space_id → role` for all spaces the user belongs to
- A list of `scopes` (for API keys: restricted; for sessions: full)

Permission checks use `AuthContext::can(space, resource, action)`:

1. Org admins → always allowed
2. Check the user's role in the target space
3. Check whether the token's scopes include the required `resource:action`
4. If the scope has `resource_ids` restrictions, verify the target ID is in the list

## Storage

Multi-user data lives in a **separate database** (`org.db`) from the
main assistant data (`assistant.db`). This separation allows:

- Clean upgrades — org schema evolves independently
- Backup isolation — org data can be backed up separately
- Single-user mode — works without `org.db` at all

### Database tables (org.db)

| Table                   | Purpose                                       |
| ----------------------- | --------------------------------------------- |
| `organizations`         | Org metadata                                  |
| `users`                 | User accounts (id, email, password hash, org) |
| `spaces`                | Workgroup definitions                         |
| `space_memberships`     | User ↔ space ↔ role mappings                  |
| `api_keys`              | Hashed API keys with scopes and restrictions  |
| `oauth_clients`         | Registered OAuth2 clients                     |
| `auth_codes`            | Short-lived authorization codes               |
| `refresh_tokens`        | Long-lived refresh tokens                     |
| `device_codes`          | Pending device authorization requests         |
| `catalog_items`         | Published skills, templates, interfaces       |
| `catalog_subscriptions` | Space subscriptions to catalog items          |
| `interface_configs`     | Per-space interface configurations            |
| `interface_bindings`    | Interface ↔ persona bindings                  |

## Setup guide

### 1. Start the web UI

```sh
assistant webui serve
```

On first run with no `org.db`, the server creates the database and runs
migrations automatically.

### 2. Create the first user

The first user to register becomes the org admin. Use the web UI
registration flow or create a user via the API:

```sh
curl -X POST http://localhost:8080/api/orgs/default/users \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "secure-password",
    "display_name": "Alice"
  }'
```

### 3. Create spaces

```sh
curl -X POST http://localhost:8080/api/orgs/default/spaces \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Engineering", "description": "Dev team workspace"}'
```

### 4. Add members to spaces

```sh
curl -X POST http://localhost:8080/api/orgs/default/spaces/eng/members \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"user_id": "user_bob", "role": "member"}'
```

### 5. CLI login

Users can authenticate the CLI with the device code flow:

```sh
assistant login http://localhost:8080
```

This opens a browser for authorization. After approval, the CLI stores
credentials in `~/.assistant/credentials.json` and can manage personas,
conversations, and API keys.

### 6. Create API keys for automation

```sh
assistant api-keys create --name "CI pipeline" --scopes "skills:execute,conversations:write"
```

Use the key in CI/CD:

```sh
export ASSISTANT_API_KEY=ask_live_...
export ASSISTANT_SERVER=http://your-server:8080
```

## Backward compatibility

The multi-user system is fully backward compatible:

- **No org.db** → single-user mode with legacy token auth
- **Legacy `--auth-token`** → still works, returns a default AuthContext
- **Existing `assistant.db`** → untouched, personas and conversations
  continue to work
- The `is_default` column on personas is preserved in the database for
  backward compatibility but is no longer exposed in the API

## OIDC federation (optional)

The server can delegate authentication to an external OIDC provider
(e.g., Keycloak, Auth0, Google Workspace):

1. Configure the OIDC provider in the server settings
2. `GET /oauth/authorize` redirects to the IdP login page
3. `GET /oauth/callback` receives the authorization code from the IdP
4. The server exchanges it for an ID token, creates or updates the local
   user, and issues its own JWT + refresh token

This enables SSO without managing passwords locally.
