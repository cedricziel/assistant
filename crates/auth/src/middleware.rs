//! Shared authentication state and scope checks.
//!
//! [`AuthState`] holds everything the web layer needs to resolve tokens and
//! keys into an [`AuthContext`]. [`RequireScope`] checks permissions on an
//! already-resolved context.

use std::sync::Arc;

use anyhow::Result;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use assistant_core::auth::{AuthContext, RefreshTokenStore};
use assistant_core::identity::{Action, ResourceKind, SpaceId};
use assistant_core::store::{MembershipStore, UserStore};

use crate::api_keys::ApiKeyStore;
use crate::jwt::{JwtManager, claims_to_context};

// -- Auth state shared across handlers --

/// Shared authentication state injected into the Axum router.
///
/// Contains everything needed to resolve tokens, keys, and sessions
/// into an [`AuthContext`].
#[derive(Clone)]
pub struct AuthState {
    /// JWT manager for validating bearer tokens.
    pub jwt_manager: Arc<JwtManager>,
    /// API key store for resolving `ask_live_` keys.
    pub api_key_store: Arc<dyn ApiKeyStore>,
    /// User store: confirms a token's user still exists and is on the token's
    /// session generation.
    pub users: Arc<dyn UserStore>,
    /// Membership store: the live space roles that replace a token's claims.
    pub memberships: Arc<dyn MembershipStore>,
    /// Refresh token store, revoked together with the access tokens on logout.
    pub refresh_tokens: Arc<dyn RefreshTokenStore>,
    /// Optional legacy token for backward compatibility.
    /// When set, requests with this token get a default org-admin context.
    pub legacy_token: Option<String>,
    /// Default AuthContext for legacy token authentication.
    pub legacy_context: Option<AuthContext>,
}

/// Why a credential could not be turned into a live [`AuthContext`].
#[derive(Debug)]
pub enum SessionError {
    /// The credential is invalid, expired or revoked, or its user is gone.
    Rejected,
    /// `org.db` could not be read, so the credential cannot be vouched for.
    Unavailable(anyhow::Error),
}

impl AuthState {
    /// Resolve a bearer JWT to the caller's live [`AuthContext`].
    ///
    /// Besides the signature and expiry, the token must still match `org.db`:
    /// its user must exist and be on the session generation the token was
    /// signed under. The org and space roles come from `org.db` as well, never
    /// from the token, so removing or demoting a member takes effect on their
    /// next request.
    pub async fn authenticate_jwt(&self, token: &str) -> Result<AuthContext, SessionError> {
        self.authenticate_claims(self.jwt_manager.validate(token))
            .await
    }

    /// Like [`Self::authenticate_jwt`], for a token that must carry `audience`
    /// (an RFC 8707 resource such as the MCP endpoint) instead of the API's.
    pub async fn authenticate_jwt_for_audience(
        &self,
        token: &str,
        audience: &str,
    ) -> Result<AuthContext, SessionError> {
        self.authenticate_claims(self.jwt_manager.validate_for_audience(token, audience))
            .await
    }

    async fn authenticate_claims(
        &self,
        claims: anyhow::Result<crate::jwt::Claims>,
    ) -> Result<AuthContext, SessionError> {
        let claims = claims.map_err(|_| SessionError::Rejected)?;
        let mut ctx = claims_to_context(&claims).map_err(|_| SessionError::Rejected)?;

        let session = self
            .users
            .session_state(&ctx.user_id)
            .await
            .map_err(SessionError::Unavailable)?
            .ok_or(SessionError::Rejected)?;
        if session.token_version != claims.tv {
            return Err(SessionError::Rejected);
        }

        ctx.org_id = session.org_id;
        ctx.space_roles = self
            .memberships
            .get_space_roles(&ctx.user_id)
            .await
            .map_err(SessionError::Unavailable)?;
        Ok(ctx)
    }

    /// Limit an API key's [`AuthContext`] to what its owner still holds.
    ///
    /// A key keeps only the spaces its user is still a member of, at the lower
    /// of the key's role and the live role.
    pub async fn bind_api_key(&self, mut ctx: AuthContext) -> Result<AuthContext, SessionError> {
        self.users
            .session_state(&ctx.user_id)
            .await
            .map_err(SessionError::Unavailable)?
            .ok_or(SessionError::Rejected)?;

        let live = self
            .memberships
            .get_space_roles(&ctx.user_id)
            .await
            .map_err(SessionError::Unavailable)?;
        ctx.space_roles = ctx
            .space_roles
            .into_iter()
            .filter_map(|(space, key_role)| {
                let live_role = live.get(&space)?;
                let role = if live_role.has_at_least(&key_role) {
                    key_role
                } else {
                    live_role.clone()
                };
                Some((space, role))
            })
            .collect();
        Ok(ctx)
    }
}

// -- RequireScope --

/// A permission check that can be used in handler signatures.
///
/// Verifies that the extracted [`AuthContext`] has the required scope
/// for the given space, resource, and action.
#[derive(Clone)]
pub struct RequireScope {
    pub space_id: SpaceId,
    pub resource: ResourceKind,
    pub action: Action,
    pub target_id: Option<String>,
}

impl RequireScope {
    /// Check if the given context satisfies this scope requirement.
    pub fn check(&self, ctx: &AuthContext) -> Result<(), ScopeError> {
        if ctx.can(
            &self.space_id,
            &self.resource,
            &self.action,
            self.target_id.as_deref(),
        ) {
            Ok(())
        } else {
            Err(ScopeError {
                resource: self.resource.clone(),
                action: self.action.clone(),
                space_id: self.space_id.clone(),
            })
        }
    }
}

/// Error returned when a scope check fails.
#[derive(Debug)]
pub struct ScopeError {
    pub resource: ResourceKind,
    pub action: Action,
    pub space_id: SpaceId,
}

impl IntoResponse for ScopeError {
    fn into_response(self) -> Response {
        (
            StatusCode::FORBIDDEN,
            format!(
                "insufficient permissions: requires {}:{} in space {}",
                self.resource, self.action, self.space_id
            ),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use assistant_core::identity::{OrgId, Role, Scope, UserId};
    use assistant_core::store::{
        InMemoryMembershipStore, InMemoryUserStore, SpaceMembership, User,
    };

    use crate::api_keys::InMemoryApiKeyStore;
    use crate::jwt::JwtKeyPair;
    use crate::oauth2::server::InMemoryRefreshTokenStore;

    use super::*;

    const SECRET: &[u8] = b"test-secret-that-is-long-enough!!";

    fn jwt_manager() -> JwtManager {
        JwtManager::new(
            JwtKeyPair::from_secret(SECRET).unwrap(),
            "https://test.local".into(),
            "https://test.local".into(),
        )
    }

    /// An [`AuthState`] over in-memory stores holding `bob`, a member of `eng`
    /// in org `acme`.
    async fn state_with_bob() -> (AuthState, Arc<InMemoryMembershipStore>) {
        let users = Arc::new(InMemoryUserStore::default());
        let memberships = Arc::new(InMemoryMembershipStore::default());
        let now = chrono::Utc::now();
        users
            .create_user(&User {
                id: UserId::from("bob"),
                org_id: OrgId::from("acme"),
                email: "bob@acme.com".into(),
                name: "Bob".into(),
                password_hash: String::new(),
                idp_issuer: None,
                idp_subject: None,
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        set_role(&memberships, "eng", Some(Role::Member)).await;
        let state = AuthState {
            jwt_manager: Arc::new(jwt_manager()),
            api_key_store: Arc::new(InMemoryApiKeyStore::new()),
            users,
            memberships: memberships.clone(),
            refresh_tokens: Arc::new(InMemoryRefreshTokenStore::default()),
            legacy_token: None,
            legacy_context: None,
        };
        (state, memberships)
    }

    /// Make bob hold `role` in `space`, or no role at all.
    async fn set_role(memberships: &InMemoryMembershipStore, space: &str, role: Option<Role>) {
        let (user, space) = (UserId::from("bob"), SpaceId::from(space));
        memberships.remove_membership(&user, &space).await.unwrap();
        if let Some(role) = role {
            memberships
                .add_membership(&SpaceMembership {
                    user_id: user,
                    space_id: space,
                    role,
                    created_at: chrono::Utc::now(),
                })
                .await
                .unwrap();
        }
    }

    fn sign_for_bob(token_version: u64) -> String {
        jwt_manager()
            .sign(&make_member_context(), "acme", "Bob", token_version)
            .unwrap()
    }

    fn make_admin_context() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("default"), Role::OrgAdmin);
        AuthContext {
            user_id: UserId::from("admin"),
            org_id: OrgId::from("default"),
            email: "admin@local".into(),
            space_roles,
            scopes: vec![Scope::new(ResourceKind::Org, Action::Manage)],
            client_id: "legacy".into(),
        }
    }

    fn make_member_context() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        AuthContext {
            user_id: UserId::from("bob"),
            org_id: OrgId::from("acme"),
            email: "bob@acme.com".into(),
            space_roles,
            scopes: vec![
                Scope::new(ResourceKind::Personas, Action::Read),
                Scope::new(ResourceKind::Conversations, Action::Read),
                Scope::new(ResourceKind::Conversations, Action::Write),
            ],
            client_id: "webui".into(),
        }
    }

    // -- RequireScope tests --

    #[test]
    fn require_scope_passes_for_allowed_action() {
        let ctx = make_member_context();
        let scope = RequireScope {
            space_id: SpaceId::from("eng"),
            resource: ResourceKind::Conversations,
            action: Action::Read,
            target_id: None,
        };
        assert!(scope.check(&ctx).is_ok());
    }

    #[test]
    fn require_scope_fails_for_disallowed_action() {
        let ctx = make_member_context();
        let scope = RequireScope {
            space_id: SpaceId::from("eng"),
            resource: ResourceKind::Users,
            action: Action::Manage,
            target_id: None,
        };
        let err = scope.check(&ctx);
        assert!(err.is_err());
        let scope_err = err.unwrap_err();
        assert_eq!(scope_err.resource, ResourceKind::Users);
        assert_eq!(scope_err.action, Action::Manage);
    }

    #[test]
    fn require_scope_fails_for_wrong_space() {
        let ctx = make_member_context();
        let scope = RequireScope {
            space_id: SpaceId::from("marketing"),
            resource: ResourceKind::Conversations,
            action: Action::Read,
            target_id: None,
        };
        assert!(scope.check(&ctx).is_err());
    }

    #[test]
    fn require_scope_admin_bypasses() {
        let ctx = make_admin_context();
        let scope = RequireScope {
            space_id: SpaceId::from("any-space"),
            resource: ResourceKind::Users,
            action: Action::Manage,
            target_id: None,
        };
        assert!(scope.check(&ctx).is_ok());
    }

    // -- AuthState::authenticate_jwt --

    #[tokio::test]
    async fn jwt_resolves_to_live_org_and_roles() {
        let (state, _) = state_with_bob().await;

        let ctx = state.authenticate_jwt(&sign_for_bob(0)).await.unwrap();

        assert_eq!(ctx.user_id, UserId::from("bob"));
        assert_eq!(ctx.org_id, OrgId::from("acme"));
        assert_eq!(ctx.role_in(&SpaceId::from("eng")), Some(&Role::Member));
    }

    #[tokio::test]
    async fn jwt_roles_follow_membership_changes() {
        let (state, memberships) = state_with_bob().await;
        let token = sign_for_bob(0);

        // The token was signed while bob held member in `eng`; demote him.
        set_role(&memberships, "eng", Some(Role::Viewer)).await;
        let ctx = state.authenticate_jwt(&token).await.unwrap();
        assert_eq!(ctx.role_in(&SpaceId::from("eng")), Some(&Role::Viewer));

        // Removing him leaves a context with no roles at all.
        set_role(&memberships, "eng", None).await;
        let ctx = state.authenticate_jwt(&token).await.unwrap();
        assert!(ctx.space_roles.is_empty(), "removed member keeps no role");
    }

    #[tokio::test]
    async fn jwt_is_rejected_after_token_version_bump() {
        let (state, _) = state_with_bob().await;
        let token = sign_for_bob(0);
        state
            .users
            .bump_token_version(&UserId::from("bob"))
            .await
            .unwrap();

        let result = state.authenticate_jwt(&token).await;

        assert!(matches!(result, Err(SessionError::Rejected)), "{result:?}");
        // A token signed under the new generation works.
        assert!(state.authenticate_jwt(&sign_for_bob(1)).await.is_ok());
    }

    #[tokio::test]
    async fn jwt_is_rejected_when_user_is_deleted() {
        let (state, _) = state_with_bob().await;
        let token = sign_for_bob(0);
        state.users.delete_user(&UserId::from("bob")).await.unwrap();

        let result = state.authenticate_jwt(&token).await;

        assert!(matches!(result, Err(SessionError::Rejected)), "{result:?}");
    }

    #[tokio::test]
    async fn jwt_audience_decides_which_surface_accepts_it() {
        let (state, _) = state_with_bob().await;
        let mcp = "https://test.local/mcp";
        let token = jwt_manager()
            .sign_for_audience(&make_member_context(), "acme", "Bob", 0, mcp)
            .unwrap();

        assert!(matches!(
            state.authenticate_jwt(&token).await,
            Err(SessionError::Rejected)
        ));
        let ctx = state
            .authenticate_jwt_for_audience(&token, mcp)
            .await
            .unwrap();
        assert_eq!(ctx.user_id, UserId::from("bob"));
        assert!(matches!(
            state
                .authenticate_jwt_for_audience(&sign_for_bob(0), mcp)
                .await,
            Err(SessionError::Rejected)
        ));
    }

    #[tokio::test]
    async fn garbage_jwt_is_rejected() {
        let (state, _) = state_with_bob().await;

        let result = state.authenticate_jwt("not-a-jwt").await;

        assert!(matches!(result, Err(SessionError::Rejected)), "{result:?}");
    }

    // -- AuthState::bind_api_key --

    fn key_context(roles: &[(&str, Role)]) -> AuthContext {
        AuthContext {
            user_id: UserId::from("bob"),
            org_id: OrgId::from("acme"),
            email: String::new(),
            space_roles: roles
                .iter()
                .map(|(space, role)| (SpaceId::from(*space), role.clone()))
                .collect(),
            scopes: vec![Scope::new(ResourceKind::Personas, Action::Read)],
            client_id: "api_key".into(),
        }
    }

    #[tokio::test]
    async fn api_key_keeps_only_spaces_the_owner_still_belongs_to() {
        let (state, _) = state_with_bob().await;

        let ctx = state
            .bind_api_key(key_context(&[
                ("eng", Role::Member),
                ("mktg", Role::Member),
            ]))
            .await
            .unwrap();

        assert_eq!(ctx.space_roles.len(), 1);
        assert_eq!(ctx.role_in(&SpaceId::from("eng")), Some(&Role::Member));
    }

    #[tokio::test]
    async fn api_key_role_is_capped_by_the_live_role() {
        let (state, memberships) = state_with_bob().await;
        set_role(&memberships, "eng", Some(Role::Viewer)).await;

        let ctx = state
            .bind_api_key(key_context(&[("eng", Role::SpaceAdmin)]))
            .await
            .unwrap();

        assert_eq!(ctx.role_in(&SpaceId::from("eng")), Some(&Role::Viewer));
    }

    #[tokio::test]
    async fn api_key_of_a_deleted_user_is_rejected() {
        let (state, _) = state_with_bob().await;
        state.users.delete_user(&UserId::from("bob")).await.unwrap();

        let result = state
            .bind_api_key(key_context(&[("eng", Role::Member)]))
            .await;

        assert!(matches!(result, Err(SessionError::Rejected)), "{result:?}");
    }
}
