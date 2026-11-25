//! `/api/integrations/policy`: what lower layers may install.

use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
    response::Response,
};
use chrono::{DateTime, Utc};

use assistant_core::auth::AuthContext;
use assistant_core::integrations::IntegrationPolicy;

use super::IntegrationsApiState;
use super::error::{ApiError, ApiResult};
use super::manage::into_response;
use super::types::{IntegrationPolicyResponse, UpdateIntegrationPolicyRequest};
use crate::openapi::ErrorBody;

/// `GET /api/integrations/policy` — the org's integration policy.
///
/// Open to every member: the install flow needs to explain what is allowed.
#[utoipa::path(
    get,
    path = "/api/integrations/policy",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    responses(
        (status = 200, description = "The policy; the default when an org admin never saved one", body = IntegrationPolicyResponse),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
    )
)]
pub async fn get_integration_policy(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
) -> Response {
    let result = async { Ok(Json(response(state.store.get_policy(&ctx.org_id).await?))) }.await;
    into_response(result)
}

/// `PUT /api/integrations/policy` — replace the org's integration policy.
///
/// Org admins only. Tightening it deletes nothing: integrations that no longer
/// comply report status `blocked_by_policy` and contribute no tools.
#[utoipa::path(
    put,
    path = "/api/integrations/policy",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    request_body = UpdateIntegrationPolicyRequest,
    responses(
        (status = 200, description = "The new policy", body = IntegrationPolicyResponse),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 403, description = "Not an org admin", body = ErrorBody),
        (status = 422, description = "Invalid body", body = ErrorBody),
    )
)]
pub async fn put_integration_policy(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    body: Result<Json<UpdateIntegrationPolicyRequest>, JsonRejection>,
) -> Response {
    let result: ApiResult<_> = async {
        if !ctx.is_org_admin() {
            return Err(ApiError::forbidden("org admin required"));
        }
        let Json(request) = body?;
        let mut allowed_catalog_ids = Vec::new();
        for id in request.allowed_catalog_ids {
            if !allowed_catalog_ids.contains(&id) {
                allowed_catalog_ids.push(id);
            }
        }
        let policy = IntegrationPolicy {
            org_id: ctx.org_id.clone(),
            catalog_mode: request.catalog_mode.into(),
            allowed_catalog_ids,
            allow_custom_urls: request.allow_custom_urls,
            allow_user_integrations: request.allow_user_integrations,
            updated_at: Utc::now(),
        };
        state.store.put_policy(&policy).await?;
        Ok(Json(response(policy)))
    }
    .await;
    into_response(result)
}

fn response(policy: IntegrationPolicy) -> IntegrationPolicyResponse {
    IntegrationPolicyResponse {
        catalog_mode: policy.catalog_mode.into(),
        allowed_catalog_ids: policy.allowed_catalog_ids,
        allow_custom_urls: policy.allow_custom_urls,
        allow_user_integrations: policy.allow_user_integrations,
        updated_at: (policy.updated_at != DateTime::UNIX_EPOCH).then_some(policy.updated_at),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use serde_json::{Value, json};

    use super::super::testing::{Harness, Who};

    const OK: StatusCode = StatusCode::OK;
    const CREATED: StatusCode = StatusCode::CREATED;
    const FORBIDDEN: StatusCode = StatusCode::FORBIDDEN;

    fn install(layer: &str, entry: &str) -> Value {
        let mut body = json!({
            "layer": layer,
            "catalog_entry_id": entry,
            "slug": "svc",
        });
        if layer == "space" {
            body["space_id"] = json!("sp_eng");
        }
        body
    }

    fn custom(layer: &str) -> Value {
        let mut body = json!({
            "layer": layer,
            "connect_url": "https://mcp.example.com/mcp",
            "name": "Custom",
            "auth_kind": "none",
            "slug": "svc",
        });
        if layer == "space" {
            body["space_id"] = json!("sp_eng");
        }
        body
    }

    fn policy(mode: &str, allowed: &[&str], custom: bool, users: bool) -> Value {
        json!({
            "catalog_mode": mode,
            "allowed_catalog_ids": allowed,
            "allow_custom_urls": custom,
            "allow_user_integrations": users,
        })
    }

    // -- GET and PUT ------------------------------------------------------------

    #[tokio::test]
    async fn the_default_policy_is_open_but_without_custom_urls() {
        let h = Harness::new().await;

        let (status, body) = h.get(Who::Member, "/api/integrations/policy").await;

        assert_eq!(status, OK);
        assert_eq!(
            body,
            json!({
                "catalog_mode": "all",
                "allowed_catalog_ids": [],
                "allow_custom_urls": false,
                "allow_user_integrations": true,
            })
        );
    }

    #[tokio::test]
    async fn only_org_admins_replace_the_policy() {
        for who in Who::ALL {
            let h = Harness::new().await;
            let (status, body) = h
                .put(
                    who,
                    "/api/integrations/policy",
                    policy("none", &[], true, false),
                )
                .await;
            let want = if matches!(who, Who::Admin) {
                OK
            } else {
                FORBIDDEN
            };
            assert_eq!(status, want, "{who:?}: {body}");
        }
    }

    #[tokio::test]
    async fn put_replaces_the_policy_and_get_reads_it_back() {
        let h = Harness::new().await;

        let (status, put) = h
            .put(
                Who::Admin,
                "/api/integrations/policy",
                policy("allowlist", &["curated/linear-app-mcp"], true, false),
            )
            .await;
        let (_, got) = h.get(Who::Member, "/api/integrations/policy").await;

        assert_eq!(status, OK);
        assert_eq!(put["catalog_mode"], "allowlist");
        assert_eq!(
            put["allowed_catalog_ids"],
            json!(["curated/linear-app-mcp"])
        );
        assert!(put["updated_at"].is_string(), "{put}");
        assert_eq!(got, put);
    }

    #[tokio::test]
    async fn put_rejects_an_unknown_mode_with_the_error_envelope() {
        let h = Harness::new().await;

        let (status, body) = h
            .put(
                Who::Admin,
                "/api/integrations/policy",
                policy("sometimes", &[], false, true),
            )
            .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(body["error"].is_string(), "{body}");
    }

    // -- Install-time enforcement ------------------------------------------------

    #[tokio::test]
    async fn allowlist_limits_lower_layers_to_the_listed_entries() {
        let h = Harness::new().await;
        h.set_policy(|p| {
            p.catalog_mode = assistant_core::integrations::CatalogMode::Allowlist;
            p.allowed_catalog_ids = vec!["curated/linear-app-mcp".into()];
        })
        .await;

        let (denied, body) = h
            .post(
                Who::Member,
                "/api/integrations",
                install("user", "curated/notion-com-mcp"),
            )
            .await;
        let (allowed, _) = h
            .post(
                Who::Member,
                "/api/integrations",
                install("user", "curated/linear-app-mcp"),
            )
            .await;
        let (space_denied, _) = h
            .post(
                Who::EngAdmin,
                "/api/integrations",
                install("space", "curated/notion-com-mcp"),
            )
            .await;
        let (org_admin, _) = h
            .post(
                Who::Admin,
                "/api/integrations",
                install("org", "curated/notion-com-mcp"),
            )
            .await;

        assert_eq!(denied, FORBIDDEN, "{body}");
        assert!(body["error"].as_str().unwrap().contains("catalog entry"));
        assert_eq!(allowed, CREATED);
        assert_eq!(space_denied, FORBIDDEN);
        assert_eq!(
            org_admin, CREATED,
            "org admins at the org layer are unrestricted"
        );
    }

    #[tokio::test]
    async fn mode_none_stops_lower_layers_but_not_the_org_layer() {
        let h = Harness::new().await;
        h.set_policy(|p| p.catalog_mode = assistant_core::integrations::CatalogMode::None)
            .await;

        let (space, _) = h
            .post(
                Who::EngAdmin,
                "/api/integrations",
                install("space", "curated/linear-app-mcp"),
            )
            .await;
        let (user, _) = h
            .post(
                Who::Member,
                "/api/integrations",
                install("user", "curated/linear-app-mcp"),
            )
            .await;
        let (org, _) = h
            .post(
                Who::Admin,
                "/api/integrations",
                install("org", "curated/linear-app-mcp"),
            )
            .await;
        let (admin_at_space, _) = h
            .post(
                Who::Admin,
                "/api/integrations",
                install("space", "curated/linear-app-mcp"),
            )
            .await;

        assert_eq!((space, user), (FORBIDDEN, FORBIDDEN));
        assert_eq!(org, CREATED);
        assert_eq!(
            admin_at_space, FORBIDDEN,
            "the policy binds the space layer, even for admins"
        );
    }

    #[tokio::test]
    async fn custom_urls_follow_allow_custom_urls() {
        let h = Harness::new().await;

        let (space, body) = h
            .post(Who::EngAdmin, "/api/integrations", custom("space"))
            .await;
        let (user, _) = h
            .post(Who::Member, "/api/integrations", custom("user"))
            .await;
        let (org, _) = h.post(Who::Admin, "/api/integrations", custom("org")).await;
        h.put(
            Who::Admin,
            "/api/integrations/policy",
            policy("all", &[], true, true),
        )
        .await;
        let (space_after, _) = h
            .post(Who::EngAdmin, "/api/integrations", custom("space"))
            .await;

        assert_eq!(space, FORBIDDEN, "{body}");
        assert!(body["error"].as_str().unwrap().contains("custom"));
        assert_eq!(user, FORBIDDEN);
        assert_eq!(org, CREATED);
        assert_eq!(space_after, CREATED, "allowed once the policy says so");
    }

    #[tokio::test]
    async fn personal_integrations_follow_allow_user_integrations() {
        let h = Harness::new().await;
        h.set_policy(|p| p.allow_user_integrations = false).await;

        let (user, body) = h
            .post(
                Who::Member,
                "/api/integrations",
                install("user", "curated/linear-app-mcp"),
            )
            .await;
        let (space, _) = h
            .post(
                Who::EngAdmin,
                "/api/integrations",
                install("space", "curated/linear-app-mcp"),
            )
            .await;

        assert_eq!(user, FORBIDDEN, "{body}");
        assert_eq!(space, CREATED);
    }

    #[tokio::test]
    async fn tightening_the_policy_blocks_existing_integrations_without_deleting_them() {
        let h = Harness::new().await;
        let (_, created) = h
            .post(
                Who::Member,
                "/api/integrations",
                install("user", "curated/notion-com-mcp"),
            )
            .await;
        let id = created["id"].as_str().unwrap().to_owned();
        assert_eq!(created["status"], "needs_connection");

        let (status, _) = h
            .put(
                Who::Admin,
                "/api/integrations/policy",
                policy("all", &[], false, false),
            )
            .await;
        let (_, detail) = h.get(Who::Member, &format!("/api/integrations/{id}")).await;
        let (_, list) = h.get(Who::Member, "/api/integrations").await;

        assert_eq!(status, OK);
        assert_eq!(detail["status"], "blocked_by_policy");
        assert_eq!(list["items"][0]["status"], "blocked_by_policy");
        let (admin, _) = h.get(Who::Admin, &format!("/api/integrations/{id}")).await;
        assert_eq!(admin, OK, "still there");
    }

    // -- installable_layers -------------------------------------------------------

    async fn layers(h: &Harness, who: Who, uri: &str) -> Value {
        let (status, body) = h.get(who, uri).await;
        assert_eq!(status, OK, "{body}");
        body["installable_layers"].clone()
    }

    const LINEAR: &str = "/api/integrations/catalog/curated/linear-app-mcp";

    #[tokio::test]
    async fn catalog_detail_lists_the_layers_the_caller_may_install_at() {
        let h = Harness::new().await;

        assert_eq!(
            layers(&h, Who::Admin, LINEAR).await,
            json!(["org", "space", "user"])
        );
        assert_eq!(
            layers(&h, Who::EngAdmin, LINEAR).await,
            json!(["space", "user"])
        );
        assert_eq!(
            layers(&h, Who::SalesAdmin, LINEAR).await,
            json!(["space", "user"])
        );
        assert_eq!(layers(&h, Who::Member, LINEAR).await, json!(["user"]));
        assert_eq!(layers(&h, Who::Viewer, LINEAR).await, json!(["user"]));
    }

    #[tokio::test]
    async fn installable_layers_follow_the_policy() {
        let h = Harness::new().await;
        h.set_policy(|p| {
            p.catalog_mode = assistant_core::integrations::CatalogMode::Allowlist;
            p.allowed_catalog_ids = vec!["curated/notion-com-mcp".into()];
        })
        .await;

        assert_eq!(layers(&h, Who::Admin, LINEAR).await, json!(["org"]));
        assert_eq!(layers(&h, Who::Member, LINEAR).await, json!([]));
        assert_eq!(
            layers(
                &h,
                Who::Member,
                "/api/integrations/catalog/curated/notion-com-mcp"
            )
            .await,
            json!(["user"])
        );

        h.set_policy(|p| p.allow_user_integrations = false).await;
        assert_eq!(layers(&h, Who::Member, LINEAR).await, json!([]));
        assert_eq!(layers(&h, Who::EngAdmin, LINEAR).await, json!(["space"]));
    }

    #[tokio::test]
    async fn catalog_list_items_carry_installable_layers_too() {
        let h = Harness::new().await;

        let (status, body) = h
            .get(Who::Member, "/api/integrations/catalog?q=linear")
            .await;

        assert_eq!(status, OK);
        assert_eq!(body["items"][0]["installable_layers"], json!(["user"]));
    }
}
