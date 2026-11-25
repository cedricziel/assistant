//! Shared authentication state and scope checks.
//!
//! [`AuthState`] holds everything the web layer needs to resolve tokens and
//! keys into an [`AuthContext`]. [`RequireScope`] checks permissions on an
//! already-resolved context.

use std::sync::Arc;

use anyhow::Result;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use assistant_core::auth::AuthContext;
use assistant_core::identity::{Action, ResourceKind, SpaceId};

use crate::api_keys::ApiKeyStore;
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

    use super::*;

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
}
