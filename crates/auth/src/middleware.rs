//! Axum auth middleware extractors.
//!
//! Provides [`AuthExtractor`] which resolves an [`AuthContext`] from incoming
//! requests by checking (in order):
//!
//! 1. `Authorization: Bearer <JWT>` header
//! 2. Session cookie (encrypted reference to a refresh token)
//! 3. API key (`ask_live_` prefix detection)
//!
//! Also provides [`RequireScope`] as a tower layer that checks permissions
//! after extraction.

use std::sync::Arc;

use anyhow::Result;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use assistant_core::auth::AuthContext;
use assistant_core::identity::{Action, ResourceKind, SpaceId};

use crate::api_keys::{self, ApiKeyStore};
use crate::jwt::JwtManager;

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
    /// Optional legacy token for backward compatibility.
    /// When set, requests with this token get a default org-admin context.
    pub legacy_token: Option<String>,
    /// Default AuthContext for legacy token authentication.
    pub legacy_context: Option<AuthContext>,
}

// -- AuthExtractor --

/// Extracts an [`AuthContext`] from an incoming request.
///
/// Resolution order:
/// 1. `Authorization: Bearer <token>` → try JWT, then API key
/// 2. Legacy `ASSISTANT_WEB_TOKEN` match (backward compatibility)
/// 3. Reject with 401
#[derive(Debug)]
pub struct AuthExtractor(pub AuthContext);

/// Error type returned when authentication fails.
#[derive(Debug)]
pub enum AuthError {
    /// No credentials provided.
    MissingCredentials,
    /// Credentials provided but invalid.
    InvalidCredentials(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            Self::MissingCredentials => {
                (StatusCode::UNAUTHORIZED, "authentication required").into_response()
            }
            Self::InvalidCredentials(msg) => (
                StatusCode::UNAUTHORIZED,
                format!("invalid credentials: {msg}"),
            )
                .into_response(),
        }
    }
}

impl FromRequestParts<AuthState> for AuthExtractor {
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AuthState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let state = state.clone();

        async move {
            let token = match &auth_header {
                Some(h) if h.starts_with("Bearer ") => Some(&h[7..]),
                _ => None,
            };

            let token = match token {
                Some(t) => t,
                None => return Err(AuthError::MissingCredentials),
            };

            // 1. Try JWT validation.
            if let Ok(ctx) = state.jwt_manager.validate_to_context(token) {
                return Ok(AuthExtractor(ctx));
            }

            // 2. Try API key resolution.
            if token.starts_with("ask_live_") {
                match api_keys::resolve_key(token, state.api_key_store.as_ref()).await {
                    Ok(ctx) => return Ok(AuthExtractor(ctx)),
                    Err(e) => {
                        return Err(AuthError::InvalidCredentials(format!("API key: {e}")));
                    }
                }
            }

            // 3. Try legacy token.
            if let Some(ref legacy) = state.legacy_token
                && token == legacy
                && let Some(ref ctx) = state.legacy_context
            {
                return Ok(AuthExtractor(ctx.clone()));
            }

            Err(AuthError::InvalidCredentials(
                "token is not a valid JWT or API key".into(),
            ))
        }
    }
}

// -- RequireScope --

/// A permission check that can be used in handler signatures.
///
/// Verifies that the extracted [`AuthContext`] has the required scope
/// for the given space, resource, and action.
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

    use axum::http::Request;

    use assistant_core::identity::{OrgId, Role, Scope, UserId};

    use crate::api_keys::InMemoryApiKeyStore;
    use crate::jwt::JwtKeyPair;

    use super::*;

    fn test_auth_state() -> (AuthState, JwtManager) {
        let kp = JwtKeyPair::generate().unwrap();
        let jwt_mgr = JwtManager::new(kp, "https://test.local".into(), "https://test.local".into());
        // We need another JwtManager for the state since JwtKeyPair isn't Clone.
        // Generate a shared secret and build two managers from it.
        let secret = b"test-secret-that-is-long-enough!!";
        let kp_state = JwtKeyPair::from_secret(secret);
        let kp_sign = JwtKeyPair::from_secret(secret);
        let jwt_state = JwtManager::new(
            kp_state,
            "https://test.local".into(),
            "https://test.local".into(),
        );
        let jwt_sign = JwtManager::new(
            kp_sign,
            "https://test.local".into(),
            "https://test.local".into(),
        );

        let state = AuthState {
            jwt_manager: Arc::new(jwt_state),
            api_key_store: Arc::new(InMemoryApiKeyStore::new()),
            legacy_token: Some("legacy-token-123".into()),
            legacy_context: Some(make_admin_context()),
        };

        drop(jwt_mgr); // unused, we use the shared-secret pair instead
        (state, jwt_sign)
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

    async fn extract_from_header(
        state: &AuthState,
        auth_value: &str,
    ) -> Result<AuthContext, AuthError> {
        let req = Request::builder()
            .header("authorization", auth_value)
            .body(())
            .unwrap();
        let (mut parts, _body) = req.into_parts();

        AuthExtractor::from_request_parts(&mut parts, state)
            .await
            .map(|e| e.0)
    }

    #[tokio::test]
    async fn extract_valid_jwt() {
        let (state, jwt_sign) = test_auth_state();
        let ctx = make_member_context();
        let token = jwt_sign.sign(&ctx, "acme", "Bob").unwrap();

        let result = extract_from_header(&state, &format!("Bearer {token}")).await;
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert_eq!(extracted.user_id, UserId::from("bob"));
        assert_eq!(extracted.org_id, OrgId::from("acme"));
    }

    #[tokio::test]
    async fn extract_valid_api_key() {
        let (state, _jwt_sign) = test_auth_state();

        // Store an API key.
        let key = crate::api_keys::generate_key();
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        let record = crate::api_keys::ApiKeyRecord {
            id: "key_test".into(),
            user_id: UserId::from("alice"),
            name: "Test".into(),
            key_hash: key.key_hash.clone(),
            key_prefix: key.key_prefix.clone(),
            org_id: OrgId::from("acme"),
            space_roles,
            scopes: vec![Scope::new(ResourceKind::Personas, Action::Read)],
            expires_at: None,
            created_at: chrono::Utc::now(),
        };
        state.api_key_store.store_key(record).await.unwrap();

        let result = extract_from_header(&state, &format!("Bearer {}", key.plaintext)).await;
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert_eq!(extracted.user_id, UserId::from("alice"));
        assert_eq!(extracted.client_id, "api_key");
    }

    #[tokio::test]
    async fn extract_legacy_token() {
        let (state, _jwt_sign) = test_auth_state();

        let result = extract_from_header(&state, "Bearer legacy-token-123").await;
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert_eq!(extracted.user_id, UserId::from("admin"));
        assert!(extracted.is_org_admin());
    }

    #[tokio::test]
    async fn extract_missing_header_returns_401() {
        let (state, _jwt_sign) = test_auth_state();

        let req = Request::builder().body(()).unwrap();
        let (mut parts, _body) = req.into_parts();

        let result = AuthExtractor::from_request_parts(&mut parts, &state).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AuthError::MissingCredentials => {}
            other => panic!("expected MissingCredentials, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn extract_invalid_token_returns_401() {
        let (state, _jwt_sign) = test_auth_state();

        let result = extract_from_header(&state, "Bearer invalid-garbage-token").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AuthError::InvalidCredentials(_) => {}
            other => panic!("expected InvalidCredentials, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn extract_expired_jwt_falls_through() {
        let (state, _jwt_sign) = test_auth_state();

        // Sign with a different secret so validation fails.
        let other_kp = JwtKeyPair::from_secret(b"other-secret-that-is-long-enough");
        let other_mgr = JwtManager::new(
            other_kp,
            "https://test.local".into(),
            "https://test.local".into(),
        );
        let ctx = make_member_context();
        let bad_token = other_mgr.sign(&ctx, "acme", "Bob").unwrap();

        let result = extract_from_header(&state, &format!("Bearer {bad_token}")).await;
        assert!(result.is_err());
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
}
