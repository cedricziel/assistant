## Definition of Done

- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes
- [ ] OAuth2 flows work for web-ui, CLI, and Flutter app
- [ ] Existing single-user install migrates to default org without data loss
- [ ] OIDC mode tested against at least one provider (Keycloak or Auth0)
- [ ] API keys with scoped access work end-to-end
- [ ] Multi-user conversations are isolated (user A cannot see user B's conversations)
- [ ] Space isolation verified (space A data not accessible from space B)
- [ ] OpenAPI spec updated with all new endpoints

---

## PR Strategy

This change is broken into **10 PRs**, each independently mergeable into `main`.
Every PR leaves the workspace in a green, releasable state. Later PRs build on
earlier ones but each is a coherent, reviewable unit. Within each PR, commits are
atomic — one logical change per commit, compiles and passes tests at every point.

```
PR 1  ─── Core identity types & auth abstractions (pure additive, no behavior change)
PR 2  ─── assistant-auth crate: JWT + password (new crate, nothing depends on it yet)
PR 3  ──�� assistant-auth crate: OAuth2 server + PKCE + device code + dynamic client reg
PR 4  ──��� assistant-auth crate: OIDC federation + API keys
PR 5  ─── Storage: org/space database layer (new stores, no migration yet)
PR 6  ─── Storage: conversation/persona user-scoping + remove default persona
PR 7  ─── Web-UI: OAuth2 endpoints + auth middleware rewrite (apps can authenticate)
PR 8  ─── Web-UI: org/space/user management APIs + interface bindings + catalog
PR 9  ─── Storage migration: legacy → default org + runtime AuthContext threading
PR 10 ─── CLI OAuth2 login + Flutter OAuth2 + docs
```

---

## PR 1: Core identity types and auth abstractions

> Pure type definitions and traits. No behavior change, no new dependencies.
> Everything compiles, existing tests pass unchanged.

**Commits:**

- [x] `feat(core): add identity newtypes (UserId, OrgId, SpaceId)`
  - Create `crates/core/src/identity.rs`
  - `UserId`, `OrgId`, `SpaceId` as newtype wrappers over `String`
  - `Role` enum: `OrgAdmin`, `SpaceAdmin`, `Member`, `Viewer`
  - Derive: `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`

- [x] `feat(core): add scope and permission types`
  - Add to `crates/core/src/identity.rs`: `Scope` struct, `ResourceKind` enum (`Personas`, `Conversations`, `Messages`, `Skills`, `Interfaces`, `Bindings`, `Users`, `Org`, `ApiKeys`, `Spaces`), `Action` enum (`Read`, `Write`, `Delete`, `Execute`, `Manage`)

- [x] `feat(core): add AuthContext with permission checking`
  - Create `crates/core/src/auth.rs`
  - `AuthContext` struct: `user_id`, `org_id`, `email`, `space_roles: HashMap<SpaceId, Role>`, `scopes: Vec<Scope>`, `client_id`
  - Implement `can(&self, space, resource, action) -> bool`
  - Implement `is_org_admin(&self) -> bool`, `role_in(&self, space) -> Option<&Role>`

- [x] `test(core): permission logic for AuthContext`
  - Tests in `crates/core/src/auth.rs` `#[cfg(test)] mod tests`
  - Cover: org-admin bypasses space checks, space-admin within own space, member scoped by grants, viewer read-only, scope filtering for API keys

- [x] `feat(core): add AuthProvider and IdentityResolver traits`
  - `AuthProvider` trait in `crates/core/src/auth.rs`: `authenticate()`, `authorize()`, `token()`, `register_client()`
  - `IdentityResolver` trait: `resolve(platform, platform_user_id) -> Option<UserId>`
  - Supporting request/response types: `AuthorizeRequest`, `AuthorizeResponse`, `TokenGrant`, `TokenResponse`, `ClientRegistration`, `ClientInfo`

- [x] `feat(core): add CatalogResolver trait`
  - Create `crates/core/src/catalog.rs`
  - `CatalogResolver` trait: `resolve_skill(space, name)`, `resolve_template(space, name)`, `list_available(space, resource_type)`

- [x] `chore(core): export auth, identity, and catalog modules`
  - Update `crates/core/src/lib.rs` with re-exports

---

## PR 2: assistant-auth crate — JWT and password fundamentals

> New crate with zero consumers. Ships JWT signing/validation and password hashing.
> Workspace compiles, new crate has its own test suite.

**Commits:**

- [x] `feat(auth): scaffold assistant-auth crate`
  - Create `crates/auth/Cargo.toml` with deps: `jsonwebtoken`, `argon2`, `rand`, `uuid`, `chrono`, `async-trait`, `anyhow`, `tracing`, `serde`, `serde_json`
  - Add to workspace `Cargo.toml` members and `[workspace.dependencies]`
  - Create `crates/auth/src/lib.rs` with module declarations

- [x] `feat(auth): JWT signing and validation`
  - Create `crates/auth/src/jwt.rs`
  - Key generation (EdDSA or RS256), key persistence (load from file or generate)
  - `sign(claims: &AuthContext, expiry: Duration) -> Result<String>`
  - `validate(token: &str) -> Result<AuthContext>`
  - Claim mapping: AuthContext fields ↔ JWT registered + custom claims

- [x] `test(auth): JWT round-trip sign/validate/extract`
  - Generate key, sign token, validate, assert claims match
  - Test expired token rejection
  - Test tampered token rejection

- [x] `feat(auth): argon2 password hashing`
  - Create `crates/auth/src/password.rs`
  - `hash_password(plain: &str) -> Result<String>`
  - `verify_password(plain: &str, hash: &str) -> Result<bool>`
  - Use argon2id with sensible defaults

- [x] `test(auth): password hash and verify`
  - Hash → verify matches
  - Wrong password → verify fails
  - Different hashes for same input (salt)

---

## PR 3: assistant-auth crate — OAuth2 server, PKCE, device code, dynamic client registration

> Adds the OAuth2 authorization server logic. Still no consumers — pure library code
> with its own tests. Can be reviewed independently of any web-ui wiring.

**Commits:**

- [ ] `feat(auth): PKCE challenge and verification`
  - Create `crates/auth/src/oauth2/pkce.rs`
  - `generate_verifier() -> String`, `challenge_from_verifier(verifier) -> String`
  - `verify(verifier, challenge) -> bool`
  - S256 method per RFC 7636

- [ ] `test(auth): PKCE challenge round-trip`

- [ ] `feat(auth): authorization code grant logic`
  - Create `crates/auth/src/oauth2/server.rs`
  - `AuthCodeStore` (in-memory or DB-backed): generate, store, exchange, expire
  - `generate_auth_code(user_id, client_id, redirect_uri, pkce_challenge, scopes) -> String`
  - `exchange_auth_code(code, client_id, redirect_uri, pkce_verifier) -> Result<(AccessToken, RefreshToken)>`

- [ ] `feat(auth): refresh token management`
  - Add to `crates/auth/src/oauth2/server.rs`
  - `RefreshTokenStore`: generate, store, rotate, revoke
  - `refresh(token, client_id) -> Result<(AccessToken, RefreshToken)>` with rotation

- [ ] `test(auth): authorization code flow end-to-end (in-memory)`
  - Full flow: generate code → exchange with PKCE → get tokens → refresh → revoke

- [ ] `feat(auth): device code flow`
  - Create `crates/auth/src/oauth2/device.rs`
  - `initiate(client_id) -> DeviceCodeResponse { device_code, user_code, verification_uri, interval }`
  - `poll(device_code) -> PollResult { Pending | Authorized(tokens) | Expired }`
  - `complete(user_code, user_id)` — called when user approves in browser

- [ ] `test(auth): device code flow lifecycle`

- [ ] `feat(auth): dynamic client registration (RFC 7591)`
  - Create `crates/auth/src/oauth2/clients.rs`
  - `ClientStore`: register, lookup, list
  - `register(req: ClientRegistration) -> Result<ClientInfo>`
  - Validate redirect_uris, grant_types, response_types
  - Generate `client_id`, optional `client_secret` for confidential clients

- [ ] `test(auth): dynamic client registration and lookup`

---

## PR 4: assistant-auth crate — OIDC federation and API keys

> Completes the auth crate. After this PR, all auth mechanisms are implemented
> as library code with tests. Still no web-ui wiring.

**Commits:**

- [ ] `feat(auth): OIDC discovery and id_token validation`
  - Create `crates/auth/src/oidc.rs`
  - Add `openidconnect` to crate deps
  - `OidcProvider::discover(issuer_url) -> Result<Self>` — fetch .well-known/openid-configuration
  - `validate_id_token(token: &str) -> Result<IdTokenClaims>` — verify signature, issuer, audience, expiry
  - `extract_user_info(claims: &IdTokenClaims) -> (sub, email, name)`

- [ ] `feat(auth): OIDC user provisioning logic`
  - Add to `crates/auth/src/oidc.rs`
  - `provision_or_lookup(idp_issuer, idp_subject, email, name) -> Result<UserId>` — find existing user by (issuer, subject) or create new one
  - Auto-provision setting (org-level): auto-create or require admin pre-approval
  - Email domain matching for org assignment

- [ ] `test(auth): OIDC token validation with mocked provider (wiremock)`

- [ ] `feat(auth): scoped API key generation and resolution`
  - Create `crates/auth/src/api_keys.rs`
  - `generate_key(user_id, name, scopes, resource_restrictions, expires_at) -> (key_plaintext, key_hash, key_prefix)`
  - Key format: `ask_live_{32 random chars}` (prefix `ask_live_` for identification)
  - `resolve_key(key_plaintext) -> Result<AuthContext>` — hash, lookup, check expiry, build AuthContext from stored scopes
  - Constant-time comparison for key hash

- [ ] `test(auth): API key generation, scoping, and resolution`
  - Create key with limited scopes → resolve → AuthContext has those scopes
  - Expired key → rejection
  - Resource restrictions honored

- [ ] `feat(auth): Axum auth middleware extractors`
  - Create `crates/auth/src/middleware.rs`
  - `AuthExtractor` implements `FromRequestParts`: checks `Authorization: Bearer <jwt>`, falls back to session cookie, falls back to API key prefix detection
  - Returns `AuthContext` or 401
  - `RequireScope(resource, action)` — Axum layer that checks `ctx.can()` after extraction

- [ ] `test(auth): middleware extraction from various token types`

- [ ] `feat(auth): AuthProvider implementations (password + OIDC)`
  - Implement `AuthProvider` trait in `crates/auth/src/lib.rs`
  - `PasswordAuthProvider`: delegates to password.rs + jwt.rs + oauth2/server.rs
  - `OidcAuthProvider`: delegates to oidc.rs + jwt.rs + oauth2/server.rs
  - Factory: `create_auth_provider(config) -> Arc<dyn AuthProvider>`

---

## PR 5: Storage — org and space database layer

> New storage modules for org-level and space-level data. Additive only — existing
> storage code unchanged. New stores have their own in-memory test suites.

**Commits:**

- [ ] `feat(storage): org.db migration — organizations and users tables`
  - New migration files for org database
  - Tables: `organizations` (id, name, slug, auth_mode, created_at, updated_at), `users` (id, org_id, email, name, password_hash, idp_issuer, idp_subject, created_at, updated_at)

- [ ] `feat(storage): org.db migration — memberships, API keys, OAuth clients`
  - Tables: `space_memberships` (user_id, space_id, role, max_personas, allowed_templates_json, created_at), `api_keys` (id, user_id, name, key_hash, key_prefix, scopes_json, resource_restrictions_json, expires_at, created_at), `oauth_clients` (client_id, client_name, redirect_uris_json, grant_types_json, token_endpoint_auth_method, client_secret_hash, created_at)

- [ ] `feat(storage): org.db migration — auth state tables`
  - Tables: `auth_codes` (code_hash, user_id, client_id, redirect_uri, pkce_challenge, scopes_json, expires_at), `refresh_tokens` (token_hash, user_id, client_id, scopes_json, expires_at, revoked_at), `device_codes` (device_code_hash, user_code, client_id, user_id, scopes_json, status, expires_at), `catalog_subscriptions` (space_id, resource_type, resource_id, subscribed_at)

- [ ] `feat(storage): OrgStore — organization and user CRUD`
  - Create `crates/storage/src/org.rs`
  - `OrgStore::new(pool)` — takes org.db pool
  - CRUD: `create_org`, `get_org`, `update_org`, `create_user`, `get_user_by_email`, `get_user_by_idp`, `list_users`, `update_user`, `delete_user`

- [ ] `test(storage): OrgStore organization and user CRUD`
  - In-memory org.db, create org, create users, query, update, delete

- [ ] `feat(storage): OrgStore — space memberships and API keys`
  - Add to `crates/storage/src/org.rs`
  - `add_space_membership`, `remove_space_membership`, `get_memberships_for_user`, `get_members_of_space`
  - `create_api_key`, `list_api_keys`, `get_api_key_by_hash`, `revoke_api_key`

- [ ] `test(storage): space memberships and API key CRUD`

- [ ] `feat(storage): OrgStore — OAuth client and auth state storage`
  - `register_client`, `get_client`, `list_clients`
  - `store_auth_code`, `consume_auth_code`, `store_refresh_token`, `consume_refresh_token`, `revoke_refresh_token`
  - `store_device_code`, `get_device_code`, `complete_device_code`

- [ ] `test(storage): OAuth client registration and auth code lifecycle`

- [ ] `feat(storage): SpaceStore — space CRUD and catalog subscriptions`
  - Create `crates/storage/src/spaces.rs`
  - Operates on org.db (spaces table) + creates/opens space.db files
  - `create_space`, `list_spaces`, `get_space`, `delete_space`
  - `subscribe_catalog_resource`, `unsubscribe`, `list_subscriptions`

- [ ] `test(storage): SpaceStore CRUD and catalog subscriptions`

- [ ] `feat(storage): database pool factory for org/space resolution`
  - Modify `crates/storage/src/lib.rs`
  - `OrgPoolFactory::org_pool(org_slug) -> Result<SqlitePool>` — resolves `~/.assistant/orgs/{slug}/org.db`
  - `OrgPoolFactory::space_pool(org_slug, space_slug) -> Result<SqlitePool>` — resolves `~/.assistant/orgs/{slug}/spaces/{space}/space.db`
  - Directory creation on first access

---

## PR 6: Storage — conversation/persona user-scoping, remove default persona

> Modifies existing storage code. This is the riskiest storage PR — existing tests
> must be updated. The default persona model is removed here.

**Commits:**

- [ ] `feat(storage): add user_id scoping to ConversationStore`
  - Modify `crates/storage/src/conversations.rs`
  - Add `user_id: Option<String>` field to `ConversationStore` (Option for backward compat during migration)
  - When `user_id` is set, all queries add `AND user_id = ?` filter
  - `create_conversation` writes `user_id` into the row
  - New space.db migration: `ALTER TABLE conversations ADD COLUMN user_id TEXT`

- [ ] `test(storage): conversation user isolation`
  - Create conversations as user A and user B on same persona
  - Assert user A's store only returns user A's conversations
  - Assert user B cannot access user A's conversation by ID

- [ ] `feat(storage): add owner_user_id to personas, remove is_default`
  - Modify `crates/storage/src/personas.rs`
  - New migration: `ALTER TABLE personas ADD COLUMN owner_user_id TEXT` (null = org-owned)
  - Remove `is_default` column (migration: `ALTER TABLE personas DROP COLUMN is_default` or mark deprecated)
  - Remove `get_default_persona`, `set_default_persona` methods
  - Add `list_accessible_personas(user_id, role)` — returns org personas the user has access to + user's private personas

- [ ] `test(storage): persona ownership and access filtering`
  - Org-owned persona visible to granted users
  - User-private persona visible only to owner
  - Admin sees all personas in space

- [ ] `refactor(storage): update callers of removed default persona API`
  - Grep for `is_default`, `get_default_persona`, `set_default_persona`
  - Update runtime bootstrap, web-ui persona endpoints, CLI persona selection
  - Where a default was assumed, require explicit persona selection

- [ ] `feat(storage): add sender_user_id to messages`
  - New migration: `ALTER TABLE messages ADD COLUMN sender_user_id TEXT`
  - Modify `crates/storage/src/conversations.rs` message insertion to accept optional `sender_user_id`
  - For `role = 'user'` messages, populate sender_user_id from AuthContext

- [ ] `test(storage): messages carry sender identity`

---

## PR 7: Web-UI — OAuth2 endpoints and auth middleware rewrite

> This is the "big flip" — the web-ui starts accepting OAuth2 tokens instead of the
> single shared token. Legacy token support is retained as fallback. After this PR,
> apps can authenticate via OAuth2.

**Commits:**

- [ ] `feat(web-ui): OAuth2 route module scaffold`
  - Create `crates/web-ui/src/oauth/mod.rs` with sub-modules and route registration
  - Wire into main router at `/oauth/*`

- [ ] `feat(web-ui): GET /oauth/authorize endpoint`
  - Password mode: render login form (HTML or redirect to SPA login page)
  - OIDC mode: redirect to external IdP with state + PKCE
  - Validate `client_id`, `redirect_uri`, `response_type`, `code_challenge`

- [ ] `feat(web-ui): POST /oauth/authorize — password credential validation`
  - Accept email + password form submission
  - Validate via `AuthProvider`, generate auth code, redirect to `redirect_uri?code=`

- [ ] `feat(web-ui): GET /oauth/callback — OIDC IdP callback`
  - Exchange IdP auth code for id_token
  - Validate id_token, provision/lookup user
  - Generate our auth code, redirect to original client's redirect_uri

- [ ] `feat(web-ui): POST /oauth/token endpoint`
  - Grant types: `authorization_code` (with PKCE), `refresh_token`, `urn:ietf:params:oauth:grant-type:device_code`
  - Returns `{ access_token, token_type, expires_in, refresh_token }`

- [ ] `feat(web-ui): POST /oauth/register — dynamic client registration`
  - Accept `ClientRegistration`, validate, store, return `ClientInfo`
  - Optionally gate behind org admin approval (org setting)

- [ ] `feat(web-ui): POST /oauth/device — device code initiation`
  - Generate device_code + user_code, return verification_uri
  - `GET /oauth/device/verify` — browser page where user enters code and approves

- [ ] `feat(web-ui): POST /oauth/revoke + server metadata`
  - Token revocation (access or refresh)
  - `GET /.well-known/oauth-authorization-server` — RFC 8414 metadata document

- [ ] `test(web-ui): OAuth2 endpoint integration tests`
  - Full authorization code + PKCE flow via HTTP
  - Device code flow via HTTP
  - Token refresh and revocation
  - Invalid client_id, bad PKCE, expired code

- [ ] `feat(web-ui): rewrite auth middleware — JWT + API key + legacy fallback`
  - Replace `crates/web-ui/src/auth.rs` internals
  - Extract `AuthContext` from: Bearer JWT → session cookie → API key → legacy `ASSISTANT_WEB_TOKEN`
  - Legacy token maps to default org admin AuthContext (migration bridge)
  - `AuthContext` available as Axum extractor in all handlers

- [ ] `feat(web-ui): permission guard middleware`
  - `RequireScope` layer: checks `ctx.can(space, resource, action)` before handler
  - Returns 403 with scope information on denial

- [ ] `feat(web-ui): session cookie management`
  - On OAuth2 login completion: set `HttpOnly`, `Secure`, `SameSite=Lax` cookie
  - Cookie contains encrypted reference to refresh token
  - Silent refresh on access token expiry

- [ ] `feat(web-ui): update CSRF protection for new session model`

- [ ] `test(web-ui): auth middleware with JWT, API key, legacy token, expired JWT, invalid token`

- [ ] `chore(web-ui): update OpenAPI spec with OAuth2 endpoints`

---

## PR 8: Web-UI — org/space/user management APIs, interface bindings, catalog

> Management API surface. After this PR, orgs, spaces, users, interfaces, and
> catalog resources can be managed via the API.

**Commits:**

- [ ] `feat(web-ui): organization management endpoints`
  - `POST /api/orgs` — create org (bootstrapping / super-admin)
  - `GET /api/orgs` — list orgs (filtered by user access)
  - `GET /api/orgs/{org_id}` — read org details
  - `PATCH /api/orgs/{org_id}` — update org settings (auth mode, LLM config)
  - Requires `org:manage` scope or org-admin role

- [ ] `feat(web-ui): space management endpoints`
  - `POST /api/orgs/{org_id}/spaces` — create space
  - `GET /api/orgs/{org_id}/spaces` — list spaces (filtered by membership)
  - `GET /api/orgs/{org_id}/spaces/{space_id}` — read space
  - `PATCH /api/orgs/{org_id}/spaces/{space_id}` — update space
  - `DELETE /api/orgs/{org_id}/spaces/{space_id}` — delete space (org-admin only)

- [ ] `feat(web-ui): user management endpoints`
  - `POST /api/orgs/{org_id}/users` — invite user (creates account, sends invite or returns credentials)
  - `GET /api/orgs/{org_id}/users` — list users
  - `GET /api/orgs/{org_id}/users/{user_id}` — read user
  - `PATCH /api/orgs/{org_id}/users/{user_id}` — update user (name, role)
  - `DELETE /api/orgs/{org_id}/users/{user_id}` — remove user

- [ ] `feat(web-ui): space membership endpoints`
  - `POST /api/orgs/{org_id}/spaces/{space_id}/members` — add user to space with role
  - `GET /api/orgs/{org_id}/spaces/{space_id}/members` — list space members
  - `PATCH /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` — change role
  - `DELETE /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` — remove from space

- [ ] `feat(web-ui): API key management endpoints`
  - `POST /api/users/me/api-keys` — create scoped API key (returns plaintext once)
  - `GET /api/users/me/api-keys` — list keys (prefix + name + scopes, no secrets)
  - `DELETE /api/users/me/api-keys/{key_id}` — revoke key

- [ ] `test(web-ui): org/space/user/membership/api-key endpoint tests`
  - CRUD happy paths
  - Permission enforcement: member can't create spaces, viewer can't invite users

- [ ] `feat(web-ui): catalog management endpoints`
  - `POST /api/orgs/{org_id}/catalog/skills` — publish skill to catalog
  - `POST /api/orgs/{org_id}/catalog/templates` — publish template to catalog
  - `GET /api/orgs/{org_id}/catalog/{type}` — list catalog resources
  - `DELETE /api/orgs/{org_id}/catalog/{type}/{id}` — remove from catalog

- [ ] `feat(web-ui): catalog subscription endpoints`
  - `POST /api/orgs/{org_id}/spaces/{space_id}/subscriptions` — subscribe space to catalog resource
  - `GET /api/orgs/{org_id}/spaces/{space_id}/subscriptions` — list subscriptions
  - `DELETE /api/orgs/{org_id}/spaces/{space_id}/subscriptions/{id}` — unsubscribe

- [ ] `feat(web-ui): interface instance endpoints`
  - `POST /api/orgs/{org_id}/spaces/{space_id}/interfaces` — create interface instance (type, config, owner: org|user)
  - `GET /api/orgs/{org_id}/spaces/{space_id}/interfaces` — list instances
  - `DELETE /api/orgs/{org_id}/spaces/{space_id}/interfaces/{id}` — remove instance

- [ ] `feat(web-ui): persona ↔ interface binding endpoints`
  - `POST /api/orgs/{org_id}/spaces/{space_id}/bindings` — bind persona to interface instance
  - `GET /api/orgs/{org_id}/spaces/{space_id}/bindings` — list bindings
  - `DELETE /api/orgs/{org_id}/spaces/{space_id}/bindings/{id}` — unbind

- [ ] `feat(web-ui): persona template instantiation and onboarding`
  - `GET /api/orgs/{org_id}/catalog/templates` — list templates (filtered by user's allowed_templates quota)
  - `POST /api/orgs/{org_id}/spaces/{space_id}/personas/from-template` — create persona from template, enforce max_personas quota
  - `GET /api/users/me/onboarding-status` — has user created at least one persona?

- [ ] `test(web-ui): catalog, subscriptions, interfaces, bindings, and template instantiation`

- [ ] `refactor(web-ui): update existing API handlers to enforce AuthContext`
  - Conversations, messages, personas, skills endpoints
  - Scope queries by user_id + space from AuthContext
  - Return 403 when user lacks access

- [ ] `chore(web-ui): update OpenAPI spec with management endpoints`

---

## PR 9: Storage migration + runtime AuthContext threading

> The migration path for existing installs, and the runtime plumbing that threads
> identity through the orchestrator and channel runner.

**Commits:**

- [ ] `feat(storage): detect legacy layout`
  - Create `crates/storage/src/migration.rs`
  - `is_legacy_layout(base_path) -> bool` — checks for `assistant.db` without `orgs/` directory

- [ ] `feat(storage): create backup before migration`
  - `backup_legacy(base_path) -> Result<PathBuf>` — tar.gz of entire `~/.assistant/`
  - Reuse existing backup logic from `crates/backup/`

- [ ] `feat(storage): migrate filesystem layout to default org`
  - Create `orgs/default/spaces/default/` structure
  - Copy `agents/` → `orgs/default/spaces/default/agents/`
  - Copy `skills/` → `orgs/default/spaces/default/skills/`
  - Split `config.toml` → `server.toml` + `orgs/default/org.toml`
  - Convert interface config sections to instance files in `orgs/default/spaces/default/interfaces/`

- [ ] `feat(storage): migrate database to org/space split`
  - Copy `assistant.db` → `orgs/default/spaces/default/space.db`
  - Create `orgs/default/org.db` with schema, populate with default org + initial admin user
  - Assign all existing conversations to admin user

- [ ] `feat(storage): initial admin user creation during migration`
  - If `ASSISTANT_WEB_TOKEN` is set: create admin user with that as temporary password
  - Otherwise: generate random password, print to stdout
  - Log clear instructions for the operator

- [ ] `test(storage): full migration round-trip on fixture data`
  - Create a fixture legacy layout (db + files)
  - Run migration
  - Verify: all conversations accessible, personas intact, skills present, org.db has admin user

- [ ] `feat(runtime): extend ExecutionContext with identity fields`
  - Add `user_id: Option<UserId>`, `org_id: Option<OrgId>`, `space_id: Option<SpaceId>` to `ExecutionContext`

- [ ] `feat(runtime): thread AuthContext through Orchestrator.run_turn_with_tools()`
  - Accept `AuthContext` parameter (or extract from ExecutionContext)
  - Pass through to tool handlers via execution context

- [ ] `feat(runtime): ChannelRunner resolves platform identity before dispatch`
  - On incoming message: call `IdentityResolver::resolve(platform, sender_id)`
  - If resolved: build AuthContext for that user, dispatch with identity
  - If unresolved: reject or prompt for identity linking (configurable)

- [ ] `feat(runtime): adapter registry keyed by instance ID`
  - Change `AdapterRegistry` key from interface type name to instance ID string
  - `ChannelRunner` looks up persona via binding table instead of global `agent_id`

- [ ] `feat(runtime): scheduler runs with persona identity`
  - Scheduled tasks look up the persona's owning user (or system identity for org tasks)
  - Build AuthContext for the task's execution

- [ ] `test(runtime): turn execution carries user identity to tool handlers`

---

## PR 10: CLI OAuth2 login, Flutter OAuth2, documentation

> Final PR: all client apps can authenticate, documentation is updated.
> After this merges, the full multi-user system is operational.

**Commits:**

- [ ] `feat(cli): assistant login command — device code flow`
  - `assistant login --server <url>` — initiates device code flow
  - Displays user_code and verification URL
  - Polls for completion, stores tokens on success

- [ ] `feat(cli): token storage and automatic refresh`
  - Store access + refresh tokens in `~/.assistant/credentials.json` (mode 0600)
  - On 401: attempt silent refresh, retry request
  - Clear tokens on refresh failure (re-login required)

- [ ] `feat(cli): assistant logout and api-keys commands`
  - `assistant logout` — revoke tokens, delete credentials.json
  - `assistant api-keys create --name <n> --scopes <s>` — create and display key (once)
  - `assistant api-keys list` — table of keys (prefix, name, scopes, expiry)
  - `assistant api-keys revoke <id>` — revoke key

- [ ] `feat(cli): --api-key flag for non-interactive use`
  - Accept `--api-key` or `ASSISTANT_API_KEY` env var
  - Skip device code flow, authenticate directly with key

- [ ] `test(cli): login flow mocked end-to-end`

- [ ] `feat(app): OAuth2 Authorization Code + PKCE login in Flutter`
  - Replace single-token entry with OAuth2 flow
  - Use `url_launcher` to open browser for auth, listen on localhost callback
  - Store tokens securely (macOS keychain via `flutter_secure_storage`)

- [ ] `feat(app): org/space selector after login`
  - Fetch user's orgs and spaces from API
  - Display space picker, persist selection

- [ ] `feat(app): persona creation from template during onboarding`
  - Check onboarding status on first load
  - Show template picker, create persona, navigate to chat

- [ ] `feat(app): space switcher in navigation`
  - Dropdown/drawer for switching active space
  - Refreshes persona list, conversation list on switch

- [ ] `feat(app): update conversation and persona lists for multi-user scoping`
  - Conversation list filtered by current user + space + persona
  - Persona picker shows granted org personas + own private personas

- [ ] `feat(app): API key management screen`
  - Create, list, revoke API keys
  - Copy-to-clipboard on creation

- [ ] `feat(app): admin screens (user, space, catalog management)`
  - Behind org-admin / space-admin role check
  - User management: invite, list, assign roles
  - Space management: create, list, manage memberships
  - Catalog: publish skills/templates, manage subscriptions

- [ ] `test(app): widget tests for login, space switching, persona scoping`

- [ ] `docs: update authentication.md with OAuth2 flows and API keys`

- [ ] `docs: add multi-user.md — org/space/user model, roles, setup guide`

- [ ] `docs: add ADR for multi-user-orgs architectural decision`

- [ ] `docs: update CLAUDE.md workspace structure with assistant-auth crate`

- [ ] `chore: final OpenAPI spec update with all new endpoints`
