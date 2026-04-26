//! Authentication and authorization abstractions.
//!
//! Defines the [`AuthContext`] that is threaded through every authenticated
//! operation, and the [`AuthProvider`] / [`IdentityResolver`] traits that are
//! implemented in the `assistant-auth` crate.

use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Interface;
use crate::identity::{Action, OrgId, ResourceKind, Role, Scope, SpaceId, UserId};

// -- AuthContext --

/// The resolved identity and authorization context for a request.
///
/// Built by the auth middleware from a JWT, API key, or session cookie.
/// Every authenticated operation receives this — downstream code never
/// sees tokens, passwords, or OIDC claims.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthContext {
    /// The authenticated user.
    pub user_id: UserId,
    /// The organization this user belongs to.
    pub org_id: OrgId,
    /// The user's email address.
    pub email: String,
    /// Role assignments per space. Key = space ID, value = role in that space.
    pub space_roles: HashMap<SpaceId, Role>,
    /// Granted scopes (full set for session tokens, restricted for API keys).
    pub scopes: Vec<Scope>,
    /// Which OAuth2 client issued this request.
    pub client_id: String,
}

impl AuthContext {
    /// Returns `true` if this context carries the `OrgAdmin` role in any space,
    /// or has the `org:manage` scope.
    pub fn is_org_admin(&self) -> bool {
        self.space_roles
            .values()
            .any(|r| matches!(r, Role::OrgAdmin))
    }

    /// Returns the role this user holds in the given space, if any.
    pub fn role_in(&self, space: &SpaceId) -> Option<&Role> {
        self.space_roles.get(space)
    }

    /// Check whether this context permits the given action on a resource
    /// within a space, optionally targeting a specific resource ID.
    ///
    /// Permission is granted if:
    /// 1. The user is an org admin (bypasses space-level checks), OR
    /// 2. The user holds an org-level management scope (e.g. `org:manage`), OR
    /// 3. The user holds a sufficient role in the space AND the scopes
    ///    include the requested resource+action.
    pub fn can(
        &self,
        space: &SpaceId,
        resource: &ResourceKind,
        action: &Action,
        target_id: Option<&str>,
    ) -> bool {
        // Org admins can do everything.
        if self.is_org_admin() {
            return true;
        }

        // Org-level management scope bypasses space-role checks.
        if self
            .scopes
            .iter()
            .any(|s| s.resource == ResourceKind::Org && s.action == Action::Manage)
        {
            return true;
        }

        // Must have a role in this space.
        let role = match self.role_in(space) {
            Some(r) => r,
            None => return false,
        };

        // Check role-based permission.
        let role_allows = match role {
            Role::OrgAdmin => true, // Already handled above, but defensive.
            Role::SpaceAdmin => true,
            Role::Member => Self::member_can(resource, action),
            Role::Viewer => Self::viewer_can(resource, action),
        };

        if !role_allows {
            return false;
        }

        // Check scope-based permission (API keys may have restricted scopes).
        self.scopes
            .iter()
            .any(|s| s.covers(resource, action, target_id))
    }

    /// What a Member role is allowed to do.
    fn member_can(resource: &ResourceKind, action: &Action) -> bool {
        matches!(
            (resource, action),
            (
                ResourceKind::Personas,
                Action::Read | Action::Write | Action::Delete
            ) | (
                ResourceKind::Conversations,
                Action::Read | Action::Write | Action::Delete
            ) | (ResourceKind::Messages, Action::Read | Action::Write)
                | (ResourceKind::Skills, Action::Read | Action::Execute)
                | (ResourceKind::Interfaces, Action::Read)
                | (
                    ResourceKind::Bindings,
                    Action::Read | Action::Write | Action::Delete
                )
                | (
                    ResourceKind::ApiKeys,
                    Action::Read | Action::Write | Action::Delete
                )
        )
    }

    /// What a Viewer role is allowed to do.
    fn viewer_can(resource: &ResourceKind, action: &Action) -> bool {
        matches!(
            (resource, action),
            (ResourceKind::Personas, Action::Read)
                | (ResourceKind::Conversations, Action::Read)
                | (ResourceKind::Messages, Action::Read)
                | (ResourceKind::Skills, Action::Read)
                | (ResourceKind::Interfaces, Action::Read)
                | (ResourceKind::Bindings, Action::Read)
                | (
                    ResourceKind::ApiKeys,
                    Action::Read | Action::Write | Action::Delete
                )
        )
    }
}

// -- OAuth2 types --

/// An OAuth2 authorization request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorizeRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// The result of an authorization request.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthorizeResponse {
    /// Redirect the user to this URL (e.g., IdP login page or callback with code).
    Redirect { url: String },
    /// Render a login form (password mode).
    LoginRequired {
        client_id: String,
        state: Option<String>,
    },
}

/// An OAuth2 token grant request.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "grant_type")]
pub enum TokenGrant {
    #[serde(rename = "authorization_code")]
    AuthorizationCode {
        code: String,
        redirect_uri: String,
        client_id: String,
        code_verifier: Option<String>,
    },
    #[serde(rename = "refresh_token")]
    RefreshToken {
        refresh_token: String,
        client_id: String,
    },
    #[serde(rename = "urn:ietf:params:oauth:grant-type:device_code")]
    DeviceCode {
        device_code: String,
        client_id: String,
    },
    #[serde(rename = "client_credentials")]
    ClientCredentials {
        client_id: String,
        client_secret: String,
    },
}

/// A token response from the OAuth2 token endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// A request to register a new OAuth2 client (RFC 7591).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientRegistration {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub response_types: Option<Vec<String>>,
    pub token_endpoint_auth_method: Option<String>,
}

/// Information about a registered OAuth2 client.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    pub client_id: String,
    pub client_name: String,
    pub client_secret: Option<String>,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub token_endpoint_auth_method: String,
}

// -- Traits --

/// Authenticates and authorizes requests.
///
/// Implemented in the `assistant-auth` crate with password-based and
/// OIDC-based providers. All implementations issue the assistant's own
/// JWTs regardless of the upstream identity source.
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Validate a bearer token (JWT or API key) and return the resolved context.
    async fn authenticate(&self, token: &str) -> Result<AuthContext>;

    /// Start an OAuth2 authorization flow.
    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse>;

    /// Exchange a grant (auth code, refresh token, device code) for tokens.
    async fn token(&self, grant: TokenGrant) -> Result<TokenResponse>;

    /// Register a new OAuth2 client dynamically (RFC 7591).
    async fn register_client(&self, req: ClientRegistration) -> Result<ClientInfo>;
}

/// Resolves a platform-specific user identity to an assistant [`UserId`].
///
/// Used by messenger interface adapters to map e.g. a Slack user ID
/// to the corresponding assistant user.
#[async_trait]
pub trait IdentityResolver: Send + Sync {
    /// Attempt to resolve a platform user to an assistant user.
    ///
    /// Returns `None` if no mapping exists (the platform user has not
    /// linked their account).
    async fn resolve(&self, platform: &Interface, platform_user_id: &str)
    -> Result<Option<UserId>>;
}

// -- Persistence stores --
//
// Trait + record definitions for stateful auth resources. Concrete in-memory
// implementations live in `assistant-auth`; SQLite-backed implementations live
// in `assistant-storage`. Defining the contracts here keeps `assistant-storage`
// free of any dependency on `assistant-auth`.

/// Metadata about a stored API key.
///
/// Mirrors the public surface of an API key record: `id`, owner, name,
/// hashed material, prefix, granted scopes, and expiry. The plaintext key
/// is never stored; only the SHA-256 hash + 8-char prefix.
#[derive(Clone, Debug)]
pub struct ApiKeyRecord {
    /// Unique ID for this key.
    pub id: String,
    /// The user who owns this key.
    pub user_id: UserId,
    /// Human-readable name.
    pub name: String,
    /// SHA-256 hash of the full key (hex-encoded).
    pub key_hash: String,
    /// First 8 characters of the key body (for display).
    pub key_prefix: String,
    /// The organization this key belongs to.
    pub org_id: OrgId,
    /// Space → role mapping inherited from the user at creation time.
    pub space_roles: HashMap<SpaceId, Role>,
    /// Granted scopes (may be a subset of the user's full scopes).
    pub scopes: Vec<Scope>,
    /// When this key expires.
    pub expires_at: Option<DateTime<Utc>>,
    /// When this key was created.
    pub created_at: DateTime<Utc>,
}

/// Persistence trait for API keys.
///
/// Implemented by `assistant_auth::api_keys::InMemoryApiKeyStore` (test/dev)
/// and `assistant_storage::SqliteApiKeyStore` (production).
#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    /// Store a new API key record.
    async fn store_key(&self, record: ApiKeyRecord) -> Result<()>;
    /// Look up a key by its hash.
    async fn get_by_hash(&self, key_hash: &str) -> Result<Option<ApiKeyRecord>>;
    /// List all keys for a user (without secrets).
    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ApiKeyRecord>>;
    /// Revoke (delete) a key by ID.
    async fn revoke(&self, key_id: &str, user_id: &UserId) -> Result<bool>;
}

/// Storage backend for registered OAuth2 clients.
#[async_trait]
pub trait ClientStore: Send + Sync {
    /// Register a new client.
    async fn register(&self, info: ClientInfo) -> Result<()>;
    /// Look up a client by ID.
    async fn get(&self, client_id: &str) -> Result<Option<ClientInfo>>;
    /// List all registered clients.
    async fn list(&self) -> Result<Vec<ClientInfo>>;
}

/// A pending authorization code waiting to be exchanged for tokens.
///
/// `user_id` is intentionally `String` here rather than the typed `UserId`
/// newtype used by `ApiKeyRecord` and `AuthContext`. Migrating it is planned
/// follow-up work — see openspec change `auth-typed-ids` (TODO) — and was
/// deferred from this slice to keep the OAuth2 token/refresh/device flow
/// touch-points minimal. Same reasoning applies to `StoredRefreshToken` and
/// `DeviceState::authorized_user` below.
#[derive(Clone, Debug)]
pub struct AuthCode {
    pub code: String,
    pub user_id: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub pkce_challenge: Option<String>,
    pub expires_at: DateTime<Utc>,
}

/// Storage backend for OAuth2 authorization codes.
#[async_trait]
pub trait AuthCodeStore: Send + Sync {
    /// Store a new authorization code.
    async fn store_code(&self, code: AuthCode) -> Result<()>;
    /// Look up and consume an authorization code (single-use).
    async fn take_code(&self, code: &str) -> Result<Option<AuthCode>>;
}

/// A stored refresh token.
#[derive(Clone, Debug)]
pub struct StoredRefreshToken {
    pub token: String,
    pub user_id: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    /// Whether this token has been consumed (for rotation detection).
    pub consumed: bool,
}

/// Storage backend for refresh tokens.
#[async_trait]
pub trait RefreshTokenStore: Send + Sync {
    /// Store a new refresh token.
    async fn store_token(&self, token: StoredRefreshToken) -> Result<()>;
    /// Look up a refresh token without consuming it.
    async fn get_token(&self, token: &str) -> Result<Option<StoredRefreshToken>>;
    /// Mark a refresh token as consumed (for rotation).
    async fn consume_token(&self, token: &str) -> Result<()>;
    /// Revoke all refresh tokens for a user+client pair (security measure when
    /// replay is detected).
    async fn revoke_all(&self, user_id: &str, client_id: &str) -> Result<()>;
    /// Revoke all refresh tokens belonging to a user except the specified
    /// token. Used after a self-service password change to log the user out
    /// of every other session while keeping the current one alive. Returns
    /// the number of tokens revoked.
    async fn revoke_for_user_except(&self, user_id: &str, except_token: &str) -> Result<u64>;
}

/// Externalized device flow state for storage backends.
#[derive(Clone, Debug)]
pub struct DeviceState {
    pub device_code: String,
    pub user_code: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub authorized_user: Option<String>,
}

/// Storage backend for OAuth2 device authorization grant state (RFC 8628).
#[async_trait]
pub trait DeviceCodeStore: Send + Sync {
    /// Store a new pending device authorization.
    async fn store(&self, device_code: &str, state: DeviceState) -> Result<()>;
    /// Look up a pending device authorization by device code.
    async fn get_by_device_code(&self, device_code: &str) -> Result<Option<DeviceState>>;
    /// Look up a pending device authorization by user code.
    async fn get_by_user_code(&self, user_code: &str) -> Result<Option<DeviceState>>;
    /// Update the state (e.g., mark as authorized).
    async fn update(&self, device_code: &str, state: DeviceState) -> Result<()>;
    /// Remove a device code entry.
    async fn remove(&self, device_code: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_org_admin() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::OrgAdmin);
        AuthContext {
            user_id: UserId::from("alice"),
            org_id: OrgId::from("acme"),
            email: "alice@acme.com".into(),
            space_roles,
            scopes: vec![Scope::new(ResourceKind::Org, Action::Manage)],
            client_id: "webui".into(),
        }
    }

    fn make_member(space: &str) -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from(space), Role::Member);
        AuthContext {
            user_id: UserId::from("bob"),
            org_id: OrgId::from("acme"),
            email: "bob@acme.com".into(),
            space_roles,
            scopes: vec![
                Scope::new(ResourceKind::Personas, Action::Read),
                Scope::new(ResourceKind::Personas, Action::Write),
                Scope::new(ResourceKind::Conversations, Action::Read),
                Scope::new(ResourceKind::Conversations, Action::Write),
                Scope::new(ResourceKind::Messages, Action::Read),
                Scope::new(ResourceKind::Messages, Action::Write),
                Scope::new(ResourceKind::Skills, Action::Read),
                Scope::new(ResourceKind::Skills, Action::Execute),
                Scope::new(ResourceKind::Interfaces, Action::Read),
                Scope::new(ResourceKind::Bindings, Action::Read),
                Scope::new(ResourceKind::Bindings, Action::Write),
                Scope::new(ResourceKind::ApiKeys, Action::Read),
                Scope::new(ResourceKind::ApiKeys, Action::Write),
            ],
            client_id: "webui".into(),
        }
    }

    fn make_viewer(space: &str) -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from(space), Role::Viewer);
        AuthContext {
            user_id: UserId::from("charlie"),
            org_id: OrgId::from("acme"),
            email: "charlie@acme.com".into(),
            space_roles,
            scopes: vec![
                Scope::new(ResourceKind::Personas, Action::Read),
                Scope::new(ResourceKind::Conversations, Action::Read),
                Scope::new(ResourceKind::Messages, Action::Read),
                Scope::new(ResourceKind::Skills, Action::Read),
                Scope::new(ResourceKind::Interfaces, Action::Read),
                Scope::new(ResourceKind::Bindings, Action::Read),
                Scope::new(ResourceKind::ApiKeys, Action::Read),
                Scope::new(ResourceKind::ApiKeys, Action::Write),
            ],
            client_id: "webui".into(),
        }
    }

    fn make_api_key_restricted() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        AuthContext {
            user_id: UserId::from("bob"),
            org_id: OrgId::from("acme"),
            email: "bob@acme.com".into(),
            space_roles,
            scopes: vec![Scope::restricted(
                ResourceKind::Personas,
                Action::Read,
                vec!["deploy-bot".into()],
            )],
            client_id: "api_key".into(),
        }
    }

    // -- is_org_admin --

    #[test]
    fn org_admin_is_detected() {
        let ctx = make_org_admin();
        assert!(ctx.is_org_admin());
    }

    #[test]
    fn member_is_not_org_admin() {
        let ctx = make_member("eng");
        assert!(!ctx.is_org_admin());
    }

    // -- role_in --

    #[test]
    fn role_in_returns_correct_role() {
        let ctx = make_member("eng");
        assert_eq!(
            ctx.role_in(&SpaceId::from("eng")),
            Some(&Role::Member),
            "should return Member role for assigned space"
        );
        assert_eq!(
            ctx.role_in(&SpaceId::from("mktg")),
            None,
            "should return None for unassigned space"
        );
    }

    // -- org admin bypasses --

    #[test]
    fn org_admin_can_do_anything_in_any_space() {
        let ctx = make_org_admin();
        let space = SpaceId::from("random-space");
        assert!(ctx.can(&space, &ResourceKind::Users, &Action::Manage, None));
        assert!(ctx.can(&space, &ResourceKind::Org, &Action::Delete, None));
    }

    // -- member permissions --

    #[test]
    fn member_can_read_and_write_conversations() {
        let ctx = make_member("eng");
        let space = SpaceId::from("eng");
        assert!(ctx.can(&space, &ResourceKind::Conversations, &Action::Read, None));
        assert!(ctx.can(&space, &ResourceKind::Conversations, &Action::Write, None));
    }

    #[test]
    fn member_cannot_manage_users() {
        let ctx = make_member("eng");
        let space = SpaceId::from("eng");
        assert!(!ctx.can(&space, &ResourceKind::Users, &Action::Manage, None));
    }

    #[test]
    fn member_cannot_manage_org() {
        let ctx = make_member("eng");
        let space = SpaceId::from("eng");
        assert!(!ctx.can(&space, &ResourceKind::Org, &Action::Manage, None));
    }

    #[test]
    fn member_has_no_access_to_other_space() {
        let ctx = make_member("eng");
        let space = SpaceId::from("mktg");
        assert!(!ctx.can(&space, &ResourceKind::Conversations, &Action::Read, None));
    }

    // -- viewer permissions --

    #[test]
    fn viewer_can_read_conversations() {
        let ctx = make_viewer("eng");
        let space = SpaceId::from("eng");
        assert!(ctx.can(&space, &ResourceKind::Conversations, &Action::Read, None));
    }

    #[test]
    fn viewer_cannot_write_conversations() {
        let ctx = make_viewer("eng");
        let space = SpaceId::from("eng");
        assert!(!ctx.can(&space, &ResourceKind::Conversations, &Action::Write, None));
    }

    #[test]
    fn viewer_cannot_write_messages() {
        let ctx = make_viewer("eng");
        let space = SpaceId::from("eng");
        assert!(!ctx.can(&space, &ResourceKind::Messages, &Action::Write, None));
    }

    // -- API key scope restriction --

    #[test]
    fn api_key_restricted_to_specific_persona() {
        let ctx = make_api_key_restricted();
        let space = SpaceId::from("eng");
        // Can read the specific persona.
        assert!(ctx.can(
            &space,
            &ResourceKind::Personas,
            &Action::Read,
            Some("deploy-bot"),
        ));
        // Cannot read a different persona.
        assert!(!ctx.can(
            &space,
            &ResourceKind::Personas,
            &Action::Read,
            Some("other-bot"),
        ));
    }

    #[test]
    fn api_key_without_scope_denies_action() {
        let ctx = make_api_key_restricted();
        let space = SpaceId::from("eng");
        // The key only has personas:read, not conversations:read.
        assert!(!ctx.can(&space, &ResourceKind::Conversations, &Action::Read, None,));
    }

    // -- SpaceAdmin --

    #[test]
    fn space_admin_can_manage_within_space() {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::SpaceAdmin);
        let ctx = AuthContext {
            user_id: UserId::from("dana"),
            org_id: OrgId::from("acme"),
            email: "dana@acme.com".into(),
            space_roles,
            scopes: vec![
                Scope::new(ResourceKind::Users, Action::Manage),
                Scope::new(ResourceKind::Interfaces, Action::Manage),
            ],
            client_id: "webui".into(),
        };
        let space = SpaceId::from("eng");
        assert!(ctx.can(&space, &ResourceKind::Users, &Action::Manage, None));
        assert!(ctx.can(&space, &ResourceKind::Interfaces, &Action::Manage, None));
    }

    #[test]
    fn space_admin_has_no_access_to_other_space() {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::SpaceAdmin);
        let ctx = AuthContext {
            user_id: UserId::from("dana"),
            org_id: OrgId::from("acme"),
            email: "dana@acme.com".into(),
            space_roles,
            scopes: vec![Scope::new(ResourceKind::Users, Action::Manage)],
            client_id: "webui".into(),
        };
        let space = SpaceId::from("mktg");
        assert!(!ctx.can(&space, &ResourceKind::Users, &Action::Manage, None));
    }
}
