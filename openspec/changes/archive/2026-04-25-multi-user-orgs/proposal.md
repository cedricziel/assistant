## Why

The assistant is currently a single-user system. One shared token gates all access, there is no concept of user identity, and all clients share a single global persona context. This blocks adoption in any team or organizational setting where multiple people need isolated access to their own agents, conversations, and configurations. Organizations also need to partition their business into smaller isolated units (spaces), control which resources users can access, and integrate with existing identity providers.

## What Changes

- Introduce a three-tier resource model: **Organization > Space > User** with per-org and per-space disk and database isolation
- Add an **OAuth2 Authorization Server** to the assistant (new `assistant-auth` crate) that supports two authentication modes: local username/password or external OIDC — both issue the assistant's own JWTs
- Support **dynamic OAuth2 client registration** (RFC 7591) so CLIs, mobile apps, and third-party integrations can register as OAuth clients
- All apps (CLI, web-ui, Flutter) authenticate via **OAuth2 flows** (Authorization Code + PKCE for public clients, client credentials for server integrations), replacing the single shared token
- Add **GitHub-style scoped API keys** as a parallel auth path, resolving to the same `AuthContext` as OAuth tokens
- Define **auth abstractions in `assistant-core`** (`AuthProvider`, `AuthContext`, identity types) with implementations in `assistant-auth`, following the existing trait-in-core pattern
- Remove the global default persona. Users create their first persona at onboarding, within admin-defined quotas and template restrictions
- Personas, skills, and interface instances are **space-scoped**. Org-level resources (skills, templates, LLM configs, interfaces) live in a **catalog** that spaces explicitly subscribe to
- Interface instances can be **org-owned** (shared, e.g. company Slack) or **user-owned** (private, e.g. personal Signal). Users define **bindings** between personas and interface instances
- Predefined roles (**org-admin, space-admin, member, viewer**) with admin-configurable quotas (max personas, template access) per role
- Migrate existing single-user installations to a **"default" org** with the first user as admin

## Non-goals

- Custom/dynamic role definitions (v1 uses predefined roles only)
- Channel-based routing within a single interface instance (v1: one binding per interface instance)
- Slack/platform-specific user identity mapping (deferred, solve per-platform later)
- Multi-org membership for a single user (one user belongs to exactly one org)
- Billing, subscription, or payment integration
- SCIM provisioning or directory sync
- End-to-end encryption of conversations

## Capabilities

### New Capabilities

- `org-management`: Create and configure organizations with auth mode (password or OIDC), LLM providers, and resource catalog
- `space-management`: Create isolated spaces within an org; subscribe to org catalog resources
- `user-management`: Invite users to an org, assign space memberships and roles, set quotas
- `oauth2-server`: Full OAuth2 Authorization Server with authorize, token, revoke, and dynamic client registration endpoints
- `oidc-federation`: Delegate authentication to an external OIDC provider while retaining local authorization
- `api-key-management`: Create, list, revoke scoped API keys with resource-level restrictions
- `persona-templates`: Org-maintained persona blueprints (SOUL.md, IDENTITY.md, pre-wired skills/tools) that users instantiate within quota
- `interface-bindings`: User-defined mappings between personas and interface instances (org-owned or user-owned)
- `catalog-subscription`: Org publishes resources (skills, templates, interfaces) to a catalog; spaces subscribe to consume them
- `install-migration`: Automatic migration of single-user installations to a default org

### Modified Capabilities

- `persona-lifecycle`: Personas are now space-scoped, created by users (private) or admins (org-shared), no global default
- `conversation-scoping`: Conversations scoped to (org, space, user, persona) instead of just (persona)
- `storage-layout`: Disk layout changes from flat `~/.assistant/` to `~/.assistant/orgs/{slug}/spaces/{name}/`
- `interface-config`: Interface instances become named, typed resources with ownership instead of singleton config sections
- `auth-flow`: All apps authenticate via OAuth2 instead of a single shared token

## Impact

- **New crate: `assistant-auth`** — OAuth2 server, password/OIDC providers, API key management, JWT issuance/validation, dynamic client registration
- **`assistant-core`** — New auth abstractions (`AuthProvider`, `AuthContext`, `IdentityResolver`), identity types (`UserId`, `OrgId`, `SpaceId`, `Role`, `Scope`), permission checking
- **`assistant-storage`** — New tables (organizations, spaces, users, roles, api_keys, catalog_subscriptions, interface_instances, bindings); per-org and per-space database files; migration logic for existing installs
- **`assistant-web-ui`** — OAuth2 endpoints, auth middleware rewrite (token → AuthContext), all API handlers receive and enforce AuthContext, per-user persona/conversation scoping
- **`assistant-runtime`** — AuthContext threaded through orchestrator, channel runner uses IdentityResolver, scheduler runs with persona identity
- **`assistant-interfaces`** — Interface instances become first-class resources; ChannelAdapter receives identity context
- **`assistant-cli`** — OAuth2 device code or Authorization Code + PKCE login flow, token storage
- **Flutter app** — OAuth2 login flow, org/space navigation, persona creation from templates, API key management UI
- **Dependencies**: `jsonwebtoken`, `openidconnect`, `argon2` (password hashing), `oauth2` crate
