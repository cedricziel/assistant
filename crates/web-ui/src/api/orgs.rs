//! REST JSON API for organization management.
//!
//! ## Routes
//!
//! | Method | Path              | Description               |
//! |--------|-------------------|---------------------------|
//! | GET    | `/api/orgs`       | List orgs (filtered)      |
//! | POST   | `/api/orgs`       | Create an organization    |
//! | GET    | `/api/orgs/{id}`  | Get org detail            |
//! | PATCH  | `/api/orgs/{id}`  | Update org settings       |

use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use assistant_core::OrgStore;
use assistant_core::auth::AuthContext;
use assistant_core::identity::OrgId;
use assistant_core::store::Organization;
use assistant_storage::OrgStorageLayer;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct OrgsApiState {
    pub org_storage: Arc<OrgStorageLayer>,
}

// -- Request/response types --------------------------------------------------

/// Summary of an organization.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OrgSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub auth_mode: String,
}

/// Full organization detail.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrgDetail {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub auth_mode: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Body for `POST /api/orgs`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateOrgRequest {
    pub name: String,
    pub slug: String,
    /// Auth mode: `"password"` or `"oidc"`.
    pub auth_mode: Option<String>,
}

/// Body for `PATCH /api/orgs/{id}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
    pub auth_mode: Option<String>,
}

// -- Router ------------------------------------------------------------------

pub fn orgs_api_router() -> Router<OrgsApiState> {
    Router::new()
        .route("/orgs", get(list_orgs).post(create_org))
        .route("/orgs/{id}", get(get_org).patch(update_org))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/orgs` — list organizations the user has access to.
#[utoipa::path(
    get,
    path = "/api/orgs",
    tag = "orgs",
    responses(
        (status = 200, description = "List of organizations", body = Vec<OrgSummary>),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_orgs(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<OrgsApiState>,
) -> Response {
    // Non-admin users only see their own org.
    let store = state.org_storage.org_store();
    match store.get_org(&ctx.org_id).await {
        Ok(Some(org)) => {
            let summary = OrgSummary {
                id: org.id.0.clone(),
                name: org.name,
                slug: org.slug,
                auth_mode: org.auth_mode,
            };
            Json(vec![summary]).into_response()
        }
        Ok(None) => Json(Vec::<OrgSummary>::new()).into_response(),
        Err(e) => {
            tracing::error!("failed to list orgs: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
        }
    }
}

/// `POST /api/orgs` — create a new organization.
#[utoipa::path(
    post,
    path = "/api/orgs",
    tag = "orgs",
    request_body = CreateOrgRequest,
    responses(
        (status = 201, description = "Organization created", body = OrgDetail),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn create_org(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<OrgsApiState>,
    Json(body): Json<CreateOrgRequest>,
) -> Response {
    if !ctx.is_org_admin() {
        return (StatusCode::FORBIDDEN, "org admin required").into_response();
    }

    if body.name.is_empty() || body.slug.is_empty() {
        return (StatusCode::BAD_REQUEST, "name and slug are required").into_response();
    }

    let now = Utc::now();
    let org = Organization {
        id: OrgId::from(format!("org_{}", uuid::Uuid::new_v4())),
        name: body.name,
        slug: body.slug,
        auth_mode: body.auth_mode.unwrap_or_else(|| "password".into()),
        created_at: now,
        updated_at: now,
    };

    let store = state.org_storage.org_store();
    if let Err(e) = store.create_org(&org).await {
        tracing::error!("failed to create org: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to create org").into_response();
    }

    let detail = OrgDetail {
        id: org.id.0,
        name: org.name,
        slug: org.slug,
        auth_mode: org.auth_mode,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    };
    (StatusCode::CREATED, Json(detail)).into_response()
}

/// `GET /api/orgs/{id}` — get organization detail.
#[utoipa::path(
    get,
    path = "/api/orgs/{id}",
    tag = "orgs",
    params(("id" = String, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Organization detail", body = OrgDetail),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn get_org(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<OrgsApiState>,
    Path(id): Path<String>,
) -> Response {
    let org_id = OrgId::from(id);

    // Users can only see their own org; org admins can see any.
    if ctx.org_id != org_id && !ctx.is_org_admin() {
        return (StatusCode::FORBIDDEN, "access denied").into_response();
    }

    let store = state.org_storage.org_store();
    match store.get_org(&org_id).await {
        Ok(Some(org)) => {
            let detail = OrgDetail {
                id: org.id.0,
                name: org.name,
                slug: org.slug,
                auth_mode: org.auth_mode,
                created_at: org.created_at.to_rfc3339(),
                updated_at: org.updated_at.to_rfc3339(),
            };
            Json(detail).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "org not found").into_response(),
        Err(e) => {
            tracing::error!("failed to get org: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
        }
    }
}

/// `PATCH /api/orgs/{id}` — update organization settings.
#[utoipa::path(
    patch,
    path = "/api/orgs/{id}",
    tag = "orgs",
    params(("id" = String, Path, description = "Organization ID")),
    request_body = UpdateOrgRequest,
    responses(
        (status = 200, description = "Organization updated", body = OrgDetail),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn update_org(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<OrgsApiState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateOrgRequest>,
) -> Response {
    if !ctx.is_org_admin() {
        return (StatusCode::FORBIDDEN, "org admin required").into_response();
    }

    let org_id = OrgId::from(id);
    let store = state.org_storage.org_store();

    let mut org = match store.get_org(&org_id).await {
        Ok(Some(o)) => o,
        Ok(None) => return (StatusCode::NOT_FOUND, "org not found").into_response(),
        Err(e) => {
            tracing::error!("failed to get org: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response();
        }
    };

    if let Some(name) = body.name {
        org.name = name;
    }
    if let Some(auth_mode) = body.auth_mode {
        org.auth_mode = auth_mode;
    }
    org.updated_at = Utc::now();

    if let Err(e) = store.update_org(&org).await {
        tracing::error!("failed to update org: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to update org").into_response();
    }

    let detail = OrgDetail {
        id: org.id.0,
        name: org.name,
        slug: org.slug,
        auth_mode: org.auth_mode,
        created_at: org.created_at.to_rfc3339(),
        updated_at: org.updated_at.to_rfc3339(),
    };
    Json(detail).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::collections::HashMap;
    use tower::ServiceExt;

    use assistant_core::identity::{Role, SpaceId, UserId};

    fn make_admin_ctx() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("default"), Role::OrgAdmin);
        AuthContext {
            user_id: UserId::from("admin"),
            org_id: OrgId::from("org_1"),
            email: "admin@test.local".into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    fn make_member_ctx() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        AuthContext {
            user_id: UserId::from("alice"),
            org_id: OrgId::from("org_1"),
            email: "alice@test.local".into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    fn test_app(state: OrgsApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(orgs_api_router().with_state(state))
            .layer(Extension(ctx))
    }

    async fn setup_state() -> OrgsApiState {
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        OrgsApiState { org_storage }
    }

    #[tokio::test]
    async fn create_and_get_org() {
        let state = setup_state().await;
        let app = test_app(state.clone(), make_admin_ctx());

        // Create
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "Acme Corp",
                            "slug": "acme"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: OrgDetail = serde_json::from_slice(&body).unwrap();
        assert_eq!(detail.name, "Acme Corp");
        assert_eq!(detail.slug, "acme");

        // Get
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/orgs/{}", detail.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn member_cannot_create_org() {
        let state = setup_state().await;
        let app = test_app(state, make_member_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "Evil Corp",
                            "slug": "evil"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn update_org_name() {
        let state = setup_state().await;

        // Seed an org.
        let now = Utc::now();
        let org = Organization {
            id: OrgId::from("org_1"),
            name: "Old Name".into(),
            slug: "old".into(),
            auth_mode: "password".into(),
            created_at: now,
            updated_at: now,
        };
        state
            .org_storage
            .org_store()
            .create_org(&org)
            .await
            .unwrap();

        let app = test_app(state, make_admin_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/orgs/org_1")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "New Name"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: OrgDetail = serde_json::from_slice(&body).unwrap();
        assert_eq!(detail.name, "New Name");
    }
}
