# ADR-0007: Multi-User Organizations, Spaces, and Identity

**Status**: Accepted
**Date**: 2026-04-25

## Context

The assistant was originally a single-user system: one database, one
persona namespace, one shared auth token. As adoption grew, several
requirements emerged:

1. **Team use** — multiple people need separate conversations, personas,
   and audit trails on a shared server.
2. **Access control** — not all users should have the same permissions.
   Some need admin access; others are read-only viewers.
3. **Workgroup isolation** — teams within an organization need separate
   contexts (personas, conversations, skills) without cross-contamination.
4. **Machine access** — CI/CD pipelines and integrations need scoped,
   long-lived credentials that don't require interactive login.
5. **Federation** — enterprises want to use their existing IdP (Keycloak,
   Auth0, etc.) rather than managing a separate password database.

## Decision

### Identity model: Org → Space → User

Adopt a three-level hierarchy:

- **Organization** — top-level tenant. One per deployment (for now).
- **Space** — isolated workgroup within an org. Scopes personas,
  conversations, and skill access.
- **User** — authenticated individual. Has a role per space.

This was chosen over flat RBAC because spaces provide a natural
isolation boundary that maps to real-world team structure. The model
is similar to Slack (workspace → channels → users) and GitHub
(org → repo → collaborators).

### Separate database for org data

Org/user/space data lives in `org.db`, separate from `assistant.db`.

Rationale:

- **Backward compatibility** — existing single-user deployments keep
  working with no migration. The server auto-detects whether `org.db`
  exists.
- **Independent evolution** — org schema (users, OAuth clients, API keys)
  changes at a different cadence than conversation/persona schema.
- **Operational isolation** — org data can be backed up, replicated, or
  migrated independently from runtime data.

Trade-off: two SQLite databases means two connection pools and no
cross-database joins. In practice this hasn't been a problem because
org queries (user lookup, role check) are fast and cacheable.

### AuthContext threading

Every authenticated request produces an `AuthContext` that flows through
the entire call stack:

```
HTTP request
  → Auth middleware (JWT/API key/legacy token)
    → AuthContext { user_id, org_id, space_roles, scopes }
      → Handler (permission check via can())
        → Storage (filters by user/space)
```

This was chosen over per-handler auth checks because:

- It's impossible to forget an auth check — the context is always present.
- Storage layers can enforce row-level filtering without trusting callers.
- The same AuthContext works for both JWT sessions and API keys.

### OAuth2 for authentication

Full OAuth2 stack with:

- **Authorization Code + PKCE** — for browser apps (Flutter)
- **Device Code** (RFC 8628) — for CLI and headless devices
- **Dynamic Client Registration** (RFC 7591) — clients self-register
- **Refresh Tokens** — silent renewal without re-authentication
- **Token Revocation** (RFC 7009) — clean logout

Rationale: OAuth2 is the industry standard for delegated auth. Using
standard grants means the CLI and Flutter app use well-understood flows.
The device code flow is particularly important for CLI UX — the user
authorizes in a browser, not by pasting tokens into a terminal.

Alternative considered: simple API key-only auth. Rejected because it
doesn't support browser-based apps cleanly and doesn't allow token
expiry/rotation.

### HS256 JWTs for access tokens

Access tokens are HS256 JWTs signed with a server-generated secret.

Rationale:

- **Self-contained** — the middleware can validate tokens without a
  database lookup on every request.
- **HS256 over RS256** — simpler key management for single-server
  deployments. RS256 would be needed for multi-server token validation
  but adds complexity we don't need yet.
- **1-hour TTL** — short enough to limit damage from token theft,
  long enough that refresh is infrequent.

### API keys with prefix identification

API keys use the format `ask_live_<43 base64url chars>`. Only the
SHA-256 hash is stored.

Design decisions:

- **Prefix `ask_live_`** — allows the middleware to identify API keys
  vs JWTs without attempting validation. Also makes keys recognizable
  in logs and config files.
- **Hash-only storage** — a compromised database doesn't leak usable
  keys.
- **Per-key scopes and resource restrictions** — keys can be limited to
  specific resources and actions, following the principle of least
  privilege.

### Role hierarchy

Four roles: `OrgAdmin > SpaceAdmin > Member > Viewer`.

Kept simple intentionally. Custom roles and fine-grained permission
matrices add complexity that isn't warranted at this stage. The
`Scope` system (resource + action + optional resource IDs) provides
fine-grained control for API keys without complicating the role model.

## Consequences

### Positive

- Multi-user deployments are possible with proper isolation.
- Backward compatible — single-user mode works unchanged.
- Standard OAuth2 flows work with any OAuth2-capable client.
- API keys enable secure machine access with least-privilege scoping.
- The identity model is extensible (add roles, resources, actions)
  without schema changes.

### Negative

- Two databases add operational complexity (backup, monitoring).
- HS256 JWTs require the signing key to be available on every server
  in a future multi-server deployment (would need migration to RS256).
- The org model is currently single-org. Multi-org support would require
  schema changes and routing logic.

### Risks

- **Key management** — the JWT signing key (`jwt_key.json`) is critical.
  Loss means all active sessions are invalidated. It should be backed up.
- **Token in URL** — the workflow webhook endpoint carries an HMAC token
  in the URL path. Access logs must be configured to redact this.

## Operations

The migration from a legacy single-user install to the multi-org layout
is automatic on first startup. Operational details — what changes on
disk, how to verify with `assistant doctor`, when to run
`assistant migrate finalize`, the meaning of `assistant.db.legacy`, and
the rollback procedure — are documented in
[`docs/operations/multi-org-cutover.md`](../operations/multi-org-cutover.md).
