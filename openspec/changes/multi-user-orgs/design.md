## Context

The assistant is currently single-user. Authentication is a single shared token (`ASSISTANT_WEB_TOKEN`) validated in `crates/web-ui/src/auth.rs`. There is no user, organization, or workspace model. All data lives under `~/.assistant/` with one SQLite database (`assistant.db`). Personas are the primary scoping mechanism — conversations, tasks, and webhooks are indexed by `agent_id` — but there is no user dimension.

The codebase follows a clean "trait in core, implementation elsewhere" pattern: `LlmProvider` in core with implementations in `assistant-llm-provider`, `ChannelAdapter` in core with implementations in `assistant-interfaces`, `ToolHandler` in core with implementations in `assistant-tool-executor`. The new auth system follows this same pattern.

Interface adapters (Slack, Matrix, Mattermost, Signal, Nextcloud) are currently singleton configurations in `config.toml` with one instance per interface type. The adapter registry keys by interface name, preventing multiple instances. Persona-to-interface binding is limited to a single optional `home_channel` per persona, used only by the scheduler.

## Goals

- Multi-user access with per-user data isolation within a shared organizational context
- OAuth2-based authentication for all clients, replacing the single shared token
- Support both self-managed credentials (password) and federated identity (OIDC)
- Physical data isolation at the org and space level (separate databases and disk partitions)
- Org-level resource catalog with explicit space subscription (no implicit inheritance)
- Flexible interface ownership (org or user) with user-defined persona bindings
- Backward-compatible migration for existing single-user installations

## Non-Goals

- Custom role definitions beyond the four predefined roles
- Fine-grained field-level permissions (row-level via scoping is sufficient)
- Real-time collaboration on shared conversations
- Cross-org federation or multi-org user membership

---

## Decisions

### D1: The assistant is always the OAuth2 Authorization Server

In both password and OIDC mode, the assistant server issues its own JWTs. In OIDC mode, the external IdP handles authentication (proving who the user is), but the assistant handles authorization (what the user can do). This means all downstream code sees the same token format regardless of auth mode.

**Why:** The assistant needs claims the IdP doesn't know about — org membership, space roles, persona access grants, API key scopes. Issuing our own tokens lets us embed these claims and validate them statelessly. It also means dynamic client registration, token refresh, and revocation are always under our control.

**Alternative considered:** Pass through IdP tokens directly and look up permissions on every request. Rejected because it couples every API request to the IdP's availability, prevents offline validation, and means our tokens can't carry authorization claims.

### D2: One SQLite database per org + one per space (physical isolation)

Organization-level data (users, roles, space memberships, API keys, catalog) lives in `org.db`. Space-level data (conversations, messages, memory chunks, attachments, scheduled tasks, personas) lives in `space.db` per space.

**Why:** Physical database separation provides hard isolation between orgs and between spaces. No query can accidentally leak data across boundaries without explicitly opening the wrong database file. It also enables independent backup/restore, independent migrations, and prevents one noisy space from affecting another's performance. This is consistent with the user's stated goal of spaces as "isolated chunks."

**Alternative considered:** Single database with `org_id`/`space_id` columns on all tables. Simpler operationally but relies on every query including the right WHERE clause — a single missing filter is a data leak. Also makes independent backup and space-level operations harder.

**Trade-off:** Cross-space queries (org admin dashboards, global search) require opening multiple databases. Acceptable because these are admin-only operations and SQLite supports `ATTACH DATABASE`.

### D3: Three-tier resource model (Org > Space > User) with explicit catalog subscription

Resources live at three levels. Org-level resources (skills, templates, LLM configs, interfaces) are maintained centrally in a catalog. Spaces explicitly subscribe to catalog resources to make them available. Space-local resources exist independently. User-private resources (personas, conversations) are scoped within a space.

**Why:** Explicit subscription prevents surprise resource availability and keeps space isolation meaningful. The org team can update a skill once and all subscribed spaces get the update, but a space admin must opt in. This matches how real organizations manage shared tooling — a central team maintains it, teams adopt it.

**Alternative considered:** Automatic inheritance (spaces see all org resources). Rejected because it breaks isolation semantics and makes it impossible for a space admin to control their environment.

### D4: Predefined roles with per-role quotas and template access

Four roles: `org-admin`, `space-admin`, `member`, `viewer`. Each role has associated privilege definitions. Org admins can adjust quotas (max personas, template access list) per role at the org level.

**Why:** Predefined roles are simpler to implement, reason about, and explain to users. Custom roles add significant UX complexity (role editor, permission matrix UI) with limited value in v1 when most deployments will be small-to-medium teams.

**Role definitions:**

| Capability              | org-admin | space-admin     | member       | viewer    |
| ----------------------- | --------- | --------------- | ------------ | --------- |
| Manage org settings     | yes       | no              | no           | no        |
| Create/delete spaces    | yes       | no              | no           | no        |
| Manage all users        | yes       | no              | no           | no        |
| Manage space personas   | yes       | yes (own space) | no           | no        |
| Manage space interfaces | yes       | yes (own space) | no           | no        |
| Grant space access      | yes       | yes (own space) | no           | no        |
| Create private personas | yes       | yes             | within quota | no        |
| Use granted personas    | yes       | yes             | yes          | yes       |
| Send messages           | yes       | yes             | yes          | no        |
| View conversations      | yes       | yes             | yes          | yes (own) |
| Manage own API keys     | yes       | yes             | yes          | yes       |

### D5: User-created personas are private within their space

When a member creates a persona (within their quota), it is visible only to them, scoped to the space they created it in. An org-admin or space-admin can see all personas in their scope. Admins create org-wide personas that are granted to users by role or individually.

**Why:** Private-by-default is safer and more intuitive. Users expect their custom assistant configurations to be personal. Admin-created org personas serve the shared use case. If a user loses access to a space, their private personas in that space become inaccessible — this is correct because work-context personas shouldn't outlive the work context.

**Alternative considered:** User personas float above spaces (org-level). Rejected because it breaks the space isolation model and creates ambiguity about which space's resources (skills, interfaces) the persona can access.

### D6: OAuth2 dynamic client registration (RFC 7591)

The assistant's OAuth2 server supports dynamic client registration at `POST /oauth/register`. Any application can register as an OAuth2 client without manual admin configuration.

**Why:** Essential for ecosystem growth. Third-party integrations, custom CLIs, MCP servers, and automation tools need to authenticate. Manual client registration by an admin doesn't scale. First-party clients (web-ui, CLI, Flutter app) use pre-configured client IDs but the mechanism is the same.

**Consideration:** Dynamic registration is open by default. Org admins can restrict it (require admin approval for new clients) if needed for security-sensitive deployments.

### D7: API keys use GitHub-style scoped access

API keys are owned by a user, carry explicit scopes (`resource:action`), and can optionally be restricted to specific resource IDs (e.g., a single persona). They resolve to the same `AuthContext` as OAuth tokens.

**Why:** Scoped keys follow the principle of least privilege. A CI integration that only needs to send messages to one persona shouldn't have access to user management. The GitHub model is well-understood by developers.

**Scope hierarchy:**

```
personas:read, personas:write, personas:delete
conversations:read, conversations:write, conversations:delete
messages:send, messages:read
skills:read, skills:execute, skills:write
interfaces:read, interfaces:manage
bindings:read, bindings:write
users:read, users:manage
org:read, org:manage
api_keys:read, api_keys:write, api_keys:revoke
spaces:read, spaces:manage
```

### D8: Interface instances are named, typed, owned resources with persona bindings

Interface connections are promoted from singleton config sections to first-class named resources with ownership (org or user) and explicit persona bindings. V1 supports one binding per interface instance (1:1 persona mapping).

**Why:** The current model (one `[slack]` section in config.toml) can't support multiple Slack workspaces, shared vs. private interfaces, or user-defined routing. Named instances with ownership and bindings solve all three. The 1:1 binding keeps routing unambiguous in v1 — if you want the same Slack workspace to serve two personas, create two Slack apps.

**Alternative considered:** Channel-based routing (one interface instance, multiple bindings filtered by channel). Deferred to v2 — adds routing complexity and ambiguity (what happens when no channel matches?).

### D9: Auth abstractions in core, implementations in assistant-auth

`AuthProvider`, `AuthContext`, `IdentityResolver`, and all identity types (`UserId`, `OrgId`, `SpaceId`, `Role`, `Scope`) are defined in `assistant-core`. The `assistant-auth` crate provides concrete implementations: password auth, OIDC federation, API key validation, JWT issuance, OAuth2 server logic, session management.

**Why:** Follows the established crate pattern (LlmProvider in core, implementations in llm-provider). Keeps core free of heavy dependencies (OIDC, JWT, argon2) while letting all other crates depend on core for the auth types. Any crate that needs to check permissions imports `AuthContext` from core — no dependency on the auth implementation.

### D10: Existing installations migrate to a "default" org

On first startup after upgrade, the assistant detects the legacy `~/.assistant/` layout and migrates it: creates a "default" org, moves agents/skills/database into `~/.assistant/orgs/default/spaces/default/`, and prompts for the first admin user's credentials.

**Why:** Zero-friction upgrade path. Users don't lose data or need to re-configure. The single-user experience continues to work — it's just now the "default" org with one admin user. The legacy `ASSISTANT_WEB_TOKEN` env var is accepted during migration as a temporary credential until the admin sets up proper auth.

---

## Entity Model

```
Organization
├── id: Uuid
├── name: String
├── slug: String              (filesystem-safe, used in paths)
├── auth_mode: Password | Oidc { issuer_url, client_id, client_secret }
├── created_at, updated_at
│
├── has many → Users
│   ├── id: Uuid
│   ├── org_id: Uuid
│   ├── email: String
│   ├── name: String
│   ├── password_hash: Option<String>     (password mode only)
│   ├── idp_issuer: Option<String>        (OIDC mode only)
│   ├── idp_subject: Option<String>       (OIDC mode only)
│   ├── created_at, updated_at
│   │
│   ├── has many → SpaceMemberships
│   │   ├── user_id, space_id, role
│   │   └── quotas: { max_personas, allowed_templates }
│   │
│   └── has many → ApiKeys
│       ├── id, user_id, name
│       ├── key_hash: String
│       ├── key_prefix: String            (for display: "ask_live_abc...")
│       ├── scopes: Vec<Scope>
│       ├── resource_restrictions: Option<Vec<ResourceRestriction>>
│       └── expires_at: Option<DateTime>
│
├── has many → Spaces
│   ├── id: Uuid
│   ├── org_id: Uuid
│   ├── name: String
│   ├── slug: String
│   ├── created_at, updated_at
│   │
│   ├── has many → Personas (org-created or user-private)
│   ├── has many → InterfaceInstances (org-owned or user-owned)
│   ├── has many → Bindings (persona_id, interface_instance_id)
│   ├── has many → Skills (space-local)
│   └── subscribes to → CatalogResources
│
├── has → Catalog
│   ├── Skills (org-maintained)
│   ├── PersonaTemplates (org-maintained)
│   └── InterfaceInstances (org-owned)
│
└── has → OAuthClients (dynamically registered)
    ├── client_id, client_name
    ├── redirect_uris, grant_types
    ├── token_endpoint_auth_method
    └── created_at
```

## Disk Layout

```
~/.assistant/
├── server.toml                              # global: listen addr, log level
└── orgs/
    └── {org-slug}/
        ├── org.toml                         # auth mode, LLM providers, catalog config
        ├── org.db                           # users, roles, memberships, API keys,
        │                                    # OAuth clients, catalog subscriptions
        ├── catalog/
        │   ├── skills/
        │   │   └── {skill-name}/SKILL.md
        │   ├── templates/
        │   │   └── {template-name}/
        │   │       ├── SOUL.md
        │   │       ├── IDENTITY.md
        │   │       └── template.toml        # pre-wired skills, tools, config
        │   └── interfaces/
        │       └── {instance-name}.toml
        │
        └── spaces/
            └── {space-slug}/
                ├── space.db                 # conversations, messages, memory,
                │                            # attachments metadata, personas,
                │                            # scheduled tasks, webhooks, metrics
                ├── agents/
                │   └── {persona-id}/
                │       ├── SOUL.md
                │       ├── IDENTITY.md
                │       ├── MEMORY.md
                │       ├── memory/
                │       ├── attachments/{conv_id}/{att_id}.ext
                │       └── workspace/
                ├── skills/                  # space-local skills
                └── interfaces/              # space-local interface configs
```

## Auth Flows

### Password Mode — Web UI Login

```
Browser                    Assistant Server               org.db
   │                            │                           │
   │  GET /oauth/authorize      │                           │
   │  ?client_id=webui          │                           │
   │  &response_type=code       │                           │
   │  &code_challenge=xxx       │                           │
   │  &redirect_uri=/callback   │                           │
   ├───────────────────────────▶│                           │
   │                            │                           │
   │  ◀── 200 Login Form ──────│                           │
   │                            │                           │
   │  POST /oauth/authorize     │                           │
   │  { email, password }       │                           │
   ├───────────────────────────▶│                           │
   │                            │  verify password_hash     │
   │                            ├──────────────────────────▶│
   │                            │  ◀── user record ─────────│
   │                            │                           │
   │                            │  generate auth code       │
   │  ◀── 302 /callback?code=  │                           │
   │                            │                           │
   │  POST /oauth/token         │                           │
   │  { code, code_verifier }   │                           │
   ├───────────────────────────▶│                           │
   │                            │  validate code + PKCE     │
   │                            │  build AuthContext         │
   │                            │  sign JWT                 │
   │  ◀── { access_token,      │                           │
   │        refresh_token }     │                           │
   │                            │                           │
```

### OIDC Mode — Web UI Login

```
Browser                  Assistant Server             External IdP
   │                          │                           │
   │  GET /oauth/authorize    │                           │
   │  ?client_id=webui        │                           │
   ├─────────────────────────▶│                           │
   │                          │                           │
   │  ◀── 302 to IdP ────────│                           │
   │     /authorize?client_id=assistant&...               │
   │                          │                           │
   │  ── authenticate at IdP ────────────────────────────▶│
   │  ◀── 302 /callback?code=idp_code ──────────────────│
   │                          │                           │
   │  GET /oauth/callback     │                           │
   │  ?code=idp_code          │                           │
   ├─────────────────────────▶│                           │
   │                          │  POST /token              │
   │                          │  { code=idp_code }        │
   │                          ├──────────────────────────▶│
   │                          │  ◀── { id_token } ────────│
   │                          │                           │
   │                          │  validate id_token         │
   │                          │  extract sub, email        │
   │                          │  lookup/create local user  │
   │                          │  build AuthContext          │
   │                          │  sign OUR JWT              │
   │                          │                           │
   │  ◀── 302 /callback      │                           │
   │     ?code=our_auth_code  │                           │
   │                          │                           │
   │  POST /oauth/token       │                           │
   │  { code=our_auth_code }  │                           │
   ├─────────────────────────▶│                           │
   │  ◀── { access_token,    │                           │
   │        refresh_token }   │                           │
```

### CLI — Device Code Flow

```
CLI                      Assistant Server
 │                            │
 │  POST /oauth/device        │
 │  { client_id=cli }         │
 ├───────────────────────────▶│
 │                            │
 │  ◀── { device_code,       │
 │        user_code: "ABCD", │
 │        verification_uri }  │
 │                            │
 │  Display: "Visit https://assistant.local/device        │
 │            and enter code: ABCD"                        │
 │                            │
 │  POST /oauth/token         │    (poll)
 │  { grant_type=             │
 │    device_code,            │
 │    device_code=xxx }       │
 ├───────────────────────────▶│
 │  ◀── { "pending" }        │
 │         ...                │    (user completes login in browser)
 │  POST /oauth/token         │
 ├───────────────────────────▶│
 │  ◀── { access_token,      │
 │        refresh_token }     │
```

## Token Structure

```json
{
  "iss": "https://assistant.example.com",
  "sub": "usr_abc123",
  "aud": "https://assistant.example.com",
  "exp": 1750000000,
  "iat": 1749996400,
  "jti": "tok_unique_id",

  "org_id": "org_acme",
  "org_slug": "acme",
  "email": "alice@acme.com",
  "name": "Alice",

  "spaces": {
    "spc_eng": "admin",
    "spc_mktg": "member"
  },

  "client_id": "webui",
  "scope": "personas:read conversations:read conversations:write messages:send skills:execute"
}
```

## Resource Resolution

When a persona in a space needs a resource (e.g., a skill):

1. Check space-local resources first
2. Check catalog subscriptions (org resources explicitly subscribed by this space)
3. Not found = not available (no implicit inheritance)

The catalog subscription table in `org.db`:

```sql
CREATE TABLE catalog_subscriptions (
    space_id      TEXT NOT NULL,
    resource_type TEXT NOT NULL,     -- 'skill', 'template', 'interface'
    resource_id   TEXT NOT NULL,     -- name/id of the catalog resource
    subscribed_at DATETIME NOT NULL,
    PRIMARY KEY (space_id, resource_type, resource_id)
);
```

## Crate Architecture

```
assistant-core (extended)
├── src/auth.rs          NEW  AuthProvider trait, AuthContext, TokenGrant, etc.
├── src/identity.rs      NEW  UserId, OrgId, SpaceId, Role, Scope, Action, ResourceKind
├── src/catalog.rs       NEW  CatalogResolver trait
└── src/channel.rs       MOD  ChannelAdapter receives identity context

assistant-auth (NEW CRATE)
├── src/lib.rs                 Public API: create_auth_provider(config) → Arc<dyn AuthProvider>
├── src/password.rs            Password hashing (argon2), credential verification
├── src/oidc.rs                OIDC discovery, id_token validation, claim mapping
├── src/oauth2/
│   ├── server.rs              Authorization + token endpoints
│   ├── clients.rs             Dynamic client registration (RFC 7591)
│   ├── device.rs              Device code flow
│   └── pkce.rs                PKCE challenge/verifier
├── src/jwt.rs                 JWT signing, validation, claim building
├── src/api_keys.rs            API key generation, hashing, scope resolution
└── src/middleware.rs          Axum extractors: AuthContext from request

assistant-storage (extended)
├── src/org.rs           NEW  OrgStore: organizations, users, memberships, API keys
├── src/spaces.rs        NEW  SpaceStore: space CRUD, catalog subscriptions
├── src/migration.rs     NEW  Legacy layout detection + migration to org structure
└── src/lib.rs           MOD  Database pool factory: org_pool(slug), space_pool(slug, space)

assistant-web-ui (extended)
├── src/auth.rs          MOD  Replace single-token with AuthContext middleware
├── src/oauth/           NEW  OAuth2 route handlers
│   ├── authorize.rs
│   ├── token.rs
│   ├── register.rs
│   ├── device.rs
│   └── revoke.rs
├── src/api/             MOD  All handlers receive AuthContext, enforce permissions
└── src/main.rs          MOD  Multi-org startup, database pool management

assistant-runtime (extended)
├── src/channel_runner.rs  MOD  Thread AuthContext through turns
└── src/scheduler.rs       MOD  Run scheduled tasks with persona identity

assistant-interfaces (extended)
└── src/*/adapter.rs       MOD  Receive identity context, support instance naming
```

## Risks / Trade-offs

**[Risk] JWT signing key management.** The assistant server needs a signing key for JWTs. If the key is lost, all tokens are invalidated. Mitigation: generate and persist the key in `org.toml` on first startup; support key rotation with a grace period for old keys.

**[Risk] OIDC provider outage blocks login.** In OIDC mode, if the external IdP is down, users can't authenticate. Mitigation: existing sessions (refresh tokens) continue to work; API keys are validated locally. Document this as an operational consideration.

**[Trade-off] Per-space databases increase operational complexity.** More database files means more migrations to run, more files to back up. Acceptable because the isolation guarantee is worth it, and the backup system already archives the entire `~/.assistant/` tree.

**[Trade-off] No cross-space resource sharing without the catalog.** A skill in Space A can't be used in Space B unless it's promoted to the org catalog first. This is intentional — it prevents ad-hoc coupling between spaces — but may feel restrictive for small orgs. Small orgs can use a single space.

**[Risk] Migration from single-user could fail mid-way.** File moves + database restructuring is non-atomic. Mitigation: create the new structure alongside the old one (copy, don't move), verify integrity, then remove the old layout. Keep a backup archive as a safety net.

## Open Questions

**OIDC user auto-provisioning policy.** When a user authenticates via OIDC for the first time, should they be auto-provisioned into the org (by email domain match), or require admin pre-approval? Likely an org-level setting with both options.

**Token lifetime tuning.** Access token TTL (1 hour default), refresh token TTL (30 days default), and session cookie TTL (7 days default) should be configurable per org. Need to decide sensible defaults.

**Org-level LLM provider restrictions.** The proposal mentions orgs can restrict which models users access. The mechanism (allowlist on the LLM config, quota-based, or per-role) is deferred to implementation.

**Interface instance migration.** Existing `[slack]`/`[matrix]` config sections need to become named interface instances. The migration tool needs to generate instance names from the config (e.g., `[slack]` becomes instance `slack-default`).
