//! Saved tool permission rules: list and revoke.
//!
//! | Method | Path                                | Description                         |
//! |--------|-------------------------------------|-------------------------------------|
//! | GET    | `/api/tool-permission-rules`        | The caller's rules, or the space's  |
//! | DELETE | `/api/tool-permission-rules/{id}`   | Revoke a rule                       |
//!
//! Rules are per space and per user. A space admin (or org admin) can list
//! every member's rules with `scope=space` and revoke any of them. Nobody can
//! create or edit a rule here; rules are saved from an approval answer.

use axum::{
    Json, Router,
    extract::{FromRef, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use assistant_core::permission::Rule;
use assistant_storage::{ToolPermissionRule, ToolPermissionRuleStore as _};

use crate::api::internal_error;
use crate::errors::json_error;
use crate::space::{Access, SpaceCtx, SpaceResolver};

// -- Router ------------------------------------------------------------------

/// Routes for saved tool permission rules. Mounted under `/api`.
pub fn tool_permission_rules_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    SpaceResolver: FromRef<S>,
{
    Router::new()
        .route("/tool-permission-rules", get(list_tool_permission_rules))
        .route(
            "/tool-permission-rules/{id}",
            axum::routing::delete(delete_tool_permission_rule),
        )
}

// -- Types -------------------------------------------------------------------

/// Whose rules `GET /api/tool-permission-rules` returns.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleScope {
    /// Only the caller's rules (default).
    #[default]
    Own,
    /// Every member's rules in the space. Space admins and org admins only.
    Space,
}

/// Query for `GET /api/tool-permission-rules`.
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct ListRulesQuery {
    /// `own` (default) or `space` (admins only).
    #[serde(default)]
    pub scope: RuleScope,
}

/// A saved rule.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ToolPermissionRuleSummary {
    pub id: String,
    /// The user the rule belongs to.
    pub user_id: String,
    /// `allow` or `deny`.
    pub effect: String,
    /// Rule text, for example `bash(git status*)`.
    pub rule: String,
    /// The tool part of the rule, for example `bash`.
    pub tool_name: String,
    /// RFC 3339 creation time.
    pub created_at: String,
}

impl From<ToolPermissionRule> for ToolPermissionRuleSummary {
    fn from(row: ToolPermissionRule) -> Self {
        let tool_name = row
            .rule
            .parse::<Rule>()
            .map(|r| r.tool().to_owned())
            .unwrap_or_else(|_| row.rule.clone());
        Self {
            id: row.id.to_string(),
            user_id: row.user_id,
            effect: row.effect.as_str().to_owned(),
            rule: row.rule,
            tool_name,
            created_at: row
                .created_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }
    }
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/tool-permission-rules`
///
/// The caller's saved rules in the selected space, oldest first. With
/// `scope=space`, every member's rules (space admins and org admins only).
#[utoipa::path(
    get,
    path = "/api/tool-permission-rules",
    operation_id = "list_tool_permission_rules",
    tag = "tool-permission-rules",
    params(
        ListRulesQuery,
        ("X-Space-Id" = Option<String>, Header, description = "Space to read rules from"),
    ),
    responses(
        (status = 200, description = "Saved rules", body = Vec<ToolPermissionRuleSummary>),
        (status = 401, description = "Unauthorized", body = crate::openapi::ErrorBody),
        (status = 403, description = "scope=space without admin access", body = crate::openapi::ErrorBody),
        (status = 500, description = "Internal server error", body = crate::openapi::ErrorBody),
    ),
    security(("bearer_token" = [])),
)]
pub async fn list_tool_permission_rules(
    ctx: SpaceCtx,
    Query(query): Query<ListRulesQuery>,
) -> Result<Json<Vec<ToolPermissionRuleSummary>>, Response> {
    ctx.require(Access::Read)
        .map_err(IntoResponse::into_response)?;
    let store = ctx.storage.tool_permission_rule_store();
    let rows = match query.scope {
        RuleScope::Own => store.list_for_user(ctx.auth.user_id.as_ref()).await,
        RuleScope::Space => {
            ctx.require(Access::Admin)
                .map_err(IntoResponse::into_response)?;
            store.list_for_space().await
        }
    }
    .map_err(|e| internal_error(e).into_response())?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// `DELETE /api/tool-permission-rules/{id}`
///
/// Revoke a saved rule. Members can revoke their own rules; a space admin or
/// org admin can revoke any rule in the selected space.
#[utoipa::path(
    delete,
    path = "/api/tool-permission-rules/{id}",
    operation_id = "delete_tool_permission_rule",
    tag = "tool-permission-rules",
    params(
        ("id" = String, Path, description = "Rule id"),
        ("X-Space-Id" = Option<String>, Header, description = "Space the rule belongs to"),
    ),
    responses(
        (status = 204, description = "Rule revoked"),
        (status = 401, description = "Unauthorized", body = crate::openapi::ErrorBody),
        (status = 403, description = "Viewers cannot revoke rules", body = crate::openapi::ErrorBody),
        (status = 404, description = "No such rule visible to the caller", body = crate::openapi::ErrorBody),
        (status = 500, description = "Internal server error", body = crate::openapi::ErrorBody),
    ),
    security(("bearer_token" = [])),
)]
pub async fn delete_tool_permission_rule(
    ctx: SpaceCtx,
    Path(id): Path<String>,
) -> Result<StatusCode, Response> {
    ctx.require(Access::Write)
        .map_err(IntoResponse::into_response)?;
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(not_found());
    };
    let store = ctx.storage.tool_permission_rule_store();
    let rule = store
        .get(id)
        .await
        .map_err(|e| internal_error(e).into_response())?
        .ok_or_else(not_found)?;

    let actor = ctx.auth.user_id.as_ref();
    let is_owner = rule.user_id == actor;
    if !is_owner && ctx.require(Access::Admin).is_err() {
        return Err(not_found());
    }
    // Only an admin may delete a row it does not own.
    let owner_filter = if is_owner { Some(actor) } else { None };
    let deleted = store
        .delete(id, owner_filter)
        .await
        .map_err(|e| internal_error(e).into_response())?;
    if !deleted {
        return Err(not_found());
    }
    if !is_owner {
        info!(
            target: "assistant::permission",
            actor_user_id = %actor,
            owner_user_id = %rule.user_id,
            rule = %rule.rule,
            rule_id = %rule.id,
            "tool permission rule revoked"
        );
    }
    Ok(StatusCode::NO_CONTENT)
}

fn not_found() -> Response {
    json_error(StatusCode::NOT_FOUND, "tool permission rule not found")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    use tracing_subscriber::fmt::MakeWriter;

    use assistant_core::auth::AuthContext;
    use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
    use assistant_storage::{RuleEffect, StorageLayer};

    use crate::api::test_helpers::{TestSpace, body_json, resolver_for};

    use super::*;

    const ALPHA: &str = "spc_alpha";
    const BETA: &str = "spc_beta";

    fn caller(user: &str, roles: &[(&str, Role)]) -> AuthContext {
        AuthContext {
            user_id: UserId::from(user),
            org_id: OrgId::from("org_acme"),
            email: format!("{user}@example.test"),
            space_roles: roles
                .iter()
                .map(|(space, role)| (SpaceId::from(*space), role.clone()))
                .collect::<HashMap<_, _>>(),
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    struct Fixture {
        alpha: Arc<StorageLayer>,
        beta: Arc<StorageLayer>,
        resolver: SpaceResolver,
    }

    async fn fixture() -> Fixture {
        let alpha = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let beta = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let space = |id, slug, storage: &Arc<StorageLayer>| TestSpace {
            id,
            org_id: "org_acme",
            org_slug: "acme",
            slug,
            storage: storage.clone(),
        };
        let resolver =
            resolver_for(&[space(ALPHA, "alpha", &alpha), space(BETA, "beta", &beta)]).await;
        Fixture {
            alpha,
            beta,
            resolver,
        }
    }

    async fn save(storage: &StorageLayer, user: &str, effect: RuleEffect, rule: &str) -> Uuid {
        storage
            .tool_permission_rule_store()
            .upsert(user, effect, rule, None)
            .await
            .unwrap()
            .id
    }

    async fn call(
        fx: &Fixture,
        auth: AuthContext,
        method: &str,
        uri: &str,
    ) -> (StatusCode, serde_json::Value) {
        let response = tool_permission_rules_router::<SpaceResolver>()
            .layer(axum::Extension(auth))
            .with_state(fx.resolver.clone())
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("X-Space-Id", ALPHA)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, body)
    }

    fn assert_enveloped(body: &serde_json::Value) {
        assert!(
            body["error"].is_string(),
            "error responses use the {{\"error\": ...}} envelope, got {body}"
        );
    }

    async fn rule_exists(storage: &StorageLayer, id: Uuid) -> bool {
        storage
            .tool_permission_rule_store()
            .get(id)
            .await
            .unwrap()
            .is_some()
    }

    // -- Own rules -------------------------------------------------------

    #[tokio::test]
    async fn list_returns_a_bare_array_of_only_the_callers_rules() {
        let fx = fixture().await;
        save(&fx.alpha, "alice", RuleEffect::Allow, "bash(git status*)").await;
        save(&fx.alpha, "bob", RuleEffect::Deny, "bash(rm *)").await;
        save(
            &fx.beta,
            "alice",
            RuleEffect::Allow,
            "web-fetch(https://a.test/*)",
        )
        .await;

        let (status, body) = call(
            &fx,
            caller("alice", &[(ALPHA, Role::Member)]),
            "GET",
            "/tool-permission-rules",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let rows = body.as_array().expect("bare array");
        assert_eq!(rows.len(), 1, "only alice's rule in the selected space");
        assert_eq!(rows[0]["user_id"], "alice");
        assert_eq!(rows[0]["effect"], "allow");
        assert_eq!(rows[0]["rule"], "bash(git status*)");
        assert_eq!(rows[0]["tool_name"], "bash");
        assert!(rows[0]["id"].is_string() && rows[0]["created_at"].is_string());
    }

    #[tokio::test]
    async fn list_reads_the_space_from_the_x_space_id_header() {
        let fx = fixture().await;
        save(&fx.alpha, "alice", RuleEffect::Allow, "bash(ls*)").await;
        save(&fx.beta, "alice", RuleEffect::Allow, "bash(pwd*)").await;
        let auth = caller("alice", &[(ALPHA, Role::Member), (BETA, Role::Member)]);

        let response = tool_permission_rules_router::<SpaceResolver>()
            .layer(axum::Extension(auth))
            .with_state(fx.resolver.clone())
            .oneshot(
                Request::builder()
                    .uri("/tool-permission-rules")
                    .header("X-Space-Id", BETA)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = body_json(response.into_body()).await;
        assert_eq!(body[0]["rule"], "bash(pwd*)");
        assert_eq!(body.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn owner_revokes_their_rule() {
        let fx = fixture().await;
        let id = save(&fx.alpha, "alice", RuleEffect::Allow, "bash(ls*)").await;

        let (status, _) = call(
            &fx,
            caller("alice", &[(ALPHA, Role::Member)]),
            "DELETE",
            &format!("/tool-permission-rules/{id}"),
        )
        .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(!rule_exists(&fx.alpha, id).await);
    }

    #[tokio::test]
    async fn member_gets_404_for_another_members_rule_and_it_remains() {
        let fx = fixture().await;
        let id = save(&fx.alpha, "bob", RuleEffect::Allow, "bash(ls*)").await;

        let (status, body) = call(
            &fx,
            caller("alice", &[(ALPHA, Role::Member)]),
            "DELETE",
            &format!("/tool-permission-rules/{id}"),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_enveloped(&body);
        assert!(rule_exists(&fx.alpha, id).await);
    }

    #[tokio::test]
    async fn viewer_cannot_revoke() {
        let fx = fixture().await;
        let id = save(&fx.alpha, "alice", RuleEffect::Allow, "bash(ls*)").await;

        let (status, body) = call(
            &fx,
            caller("alice", &[(ALPHA, Role::Viewer)]),
            "DELETE",
            &format!("/tool-permission-rules/{id}"),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_enveloped(&body);
        assert!(rule_exists(&fx.alpha, id).await);
    }

    #[tokio::test]
    async fn unknown_and_malformed_ids_are_404_with_the_envelope() {
        let fx = fixture().await;
        let auth = || caller("alice", &[(ALPHA, Role::Member)]);

        for id in [Uuid::new_v4().to_string(), "not-a-uuid".to_string()] {
            let (status, body) = call(
                &fx,
                auth(),
                "DELETE",
                &format!("/tool-permission-rules/{id}"),
            )
            .await;
            assert_eq!(status, StatusCode::NOT_FOUND, "id {id}");
            assert_enveloped(&body);
        }
    }

    // -- Admin ------------------------------------------------------------

    #[tokio::test]
    async fn space_and_org_admins_list_every_members_rules_with_user_ids() {
        let fx = fixture().await;
        save(&fx.alpha, "alice", RuleEffect::Allow, "bash(ls*)").await;
        save(&fx.alpha, "bob", RuleEffect::Deny, "bash(rm *)").await;
        save(&fx.beta, "carol", RuleEffect::Allow, "bash(pwd*)").await;

        // A space admin and an org admin without an explicit membership.
        for auth in [
            caller("root", &[(ALPHA, Role::SpaceAdmin)]),
            caller("boss", &[(ALPHA, Role::OrgAdmin)]),
        ] {
            let (status, body) = call(&fx, auth, "GET", "/tool-permission-rules?scope=space").await;
            assert_eq!(status, StatusCode::OK);
            let mut users: Vec<_> = body
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["user_id"].as_str().unwrap().to_owned())
                .collect();
            users.sort();
            assert_eq!(users, ["alice", "bob"], "only the selected space's rules");
        }
    }

    #[tokio::test]
    async fn members_and_viewers_cannot_list_the_spaces_rules() {
        let fx = fixture().await;
        for role in [Role::Member, Role::Viewer] {
            let (status, body) = call(
                &fx,
                caller("alice", &[(ALPHA, role)]),
                "GET",
                "/tool-permission-rules?scope=space",
            )
            .await;
            assert_eq!(status, StatusCode::FORBIDDEN);
            assert_enveloped(&body);
        }
    }

    #[tokio::test]
    async fn own_scope_is_unchanged_for_admins() {
        let fx = fixture().await;
        save(&fx.alpha, "root", RuleEffect::Allow, "bash(ls*)").await;
        save(&fx.alpha, "bob", RuleEffect::Allow, "bash(pwd*)").await;

        for uri in ["/tool-permission-rules", "/tool-permission-rules?scope=own"] {
            let (status, body) = call(
                &fx,
                caller("root", &[(ALPHA, Role::SpaceAdmin)]),
                "GET",
                uri,
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body.as_array().unwrap().len(), 1, "{uri}");
            assert_eq!(body[0]["user_id"], "root");
        }
    }

    #[tokio::test]
    async fn space_admin_revokes_a_members_rule() {
        let fx = fixture().await;
        let id = save(&fx.alpha, "bob", RuleEffect::Allow, "bash(ls*)").await;

        let (status, _) = call(
            &fx,
            caller("root", &[(ALPHA, Role::SpaceAdmin)]),
            "DELETE",
            &format!("/tool-permission-rules/{id}"),
        )
        .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(!rule_exists(&fx.alpha, id).await);
    }

    #[tokio::test]
    async fn admin_of_another_space_gets_404_for_a_rule_in_this_one() {
        let fx = fixture().await;
        let id = save(&fx.beta, "bob", RuleEffect::Allow, "bash(ls*)").await;

        // Admin of ALPHA (the selected space) deleting a rule that lives in BETA.
        let (status, body) = call(
            &fx,
            caller("root", &[(ALPHA, Role::SpaceAdmin)]),
            "DELETE",
            &format!("/tool-permission-rules/{id}"),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_enveloped(&body);
        assert!(rule_exists(&fx.beta, id).await);
    }

    // -- Audit log --------------------------------------------------------

    #[derive(Clone, Default)]
    struct LogBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for LogBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for LogBuf {
        type Writer = LogBuf;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[tokio::test]
    async fn an_admin_revoke_logs_actor_owner_rule_and_id() {
        let fx = fixture().await;
        let id = save(&fx.alpha, "bob", RuleEffect::Allow, "bash(git status*)").await;
        let logs = LogBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(logs.clone())
            .with_ansi(false)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let (status, _) = call(
            &fx,
            caller("root", &[(ALPHA, Role::SpaceAdmin)]),
            "DELETE",
            &format!("/tool-permission-rules/{id}"),
        )
        .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        let out = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
        for needle in [
            "tool permission rule revoked",
            "actor_user_id=root",
            "owner_user_id=bob",
            "rule=bash(git status*)",
            &format!("rule_id={id}"),
        ] {
            assert!(
                out.contains(needle),
                "log should contain `{needle}`:\n{out}"
            );
        }
    }

    #[tokio::test]
    async fn an_owners_own_revoke_is_not_logged_as_an_admin_revoke() {
        let fx = fixture().await;
        let id = save(&fx.alpha, "root", RuleEffect::Allow, "bash(ls*)").await;
        let logs = LogBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(logs.clone())
            .with_ansi(false)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let (status, _) = call(
            &fx,
            caller("root", &[(ALPHA, Role::SpaceAdmin)]),
            "DELETE",
            &format!("/tool-permission-rules/{id}"),
        )
        .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        let out = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
        assert!(!out.contains("tool permission rule revoked"), "{out}");
    }
}
