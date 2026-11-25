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

Every space has its own database (`space.db`) and its own runtime: an
orchestrator, tool executor, skill registry and message bus bound to that
database. A turn, schedule or workflow in one space only reads and writes
that space's data. The LLM provider, config and MCP tools are shared by all
spaces. See [ADR-0013](adr/adr-0013-per-space-runtimes.md).

### Selecting a space

Data-plane routes (conversations, messages, personas, skills, templates,
webhooks, workflows, A2A) act on one space. The client names it with the
`X-Space-Id` header. A browser `EventSource` cannot set headers, so it sends
the `space_id` query parameter instead.

The selector is optional when the caller holds a role in exactly one space.
Otherwise a request without it gets `400 space selector required`. An
unknown space, a space of another org, and a space the caller holds no role
in all return the same `403`.

The role in the selected space decides what the caller may do:

| Role                     | Access                                                                                                                                           |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Viewer`                 | Read routes                                                                                                                                      |
| `Member`                 | Read, plus routes that create or change data                                                                                                     |
| `SpaceAdmin`, `OrgAdmin` | Everything, including destructive and secret operations (for example deleting a webhook or workflow, rotating a secret, reading webhook secrets) |

Conversations are further limited to their owner: another user's
conversation returns `404`.

### Workflow webhooks

The public workflow webhook URL names its space:

```
POST /workflow-hooks/{org_slug}/{space_slug}/{id}/{token}
```

`GET /workflows/{id}/webhook-secrets` returns the full URL.

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

All data lives under the install root, which is always `~/.assistant`:

- `orgs/{slug}/org.db` holds users, spaces, memberships, API keys, OAuth
  clients and interface instances.
- `orgs/{slug}/spaces/{space}/space.db` holds one space's conversations,
  messages, personas and scheduled tasks. With the SQLite message bus, the
  space's bus tables live here too.

The `--db-path` flag and the `[storage] db_path` config key are removed.
There is no way to point the assistant at a single database file.

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

## Upgrading a single-database install

The assistant does not migrate the old single-database layout. A host with
`~/.assistant/assistant.db` starts fresh in the multi-org layout.

Workflow webhook URLs without the org and space slugs no longer work.
Re-copy them from `GET /workflows/{id}/webhook-secrets`.

## OIDC federation (optional)

Each org can delegate sign-in to its own external OIDC provider (Keycloak,
Auth0, Google Workspace). An org admin configures it through the API, and
the setting applies to that org only:

1. `PUT /api/orgs/{id}/oidc` stores the issuer, client ID, and secret.
2. `PATCH /api/orgs/{id}` with `{"auth_mode": "oidc"}` turns it on.
3. `GET /oauth/authorize` finds the user's org from the email (or from
   `?org=<slug>`) and redirects to that org's IdP.
4. `GET /oauth/callback` receives the code. The server exchanges it for an ID
   token, finds or creates the user in that org, and issues its own JWT and
   refresh token.

Other orgs on the same server keep password sign-in or use their own IdP.
See [Per-org OIDC](authentication.md#per-org-oidc) and
[Set up OIDC sign-in for an org](operations/org-oidc-sign-in.md).
