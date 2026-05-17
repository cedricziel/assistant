//! REST JSON API for space management.
//!
//! ## Routes
//!
//! | Method | Path                            | Description       |
//! |--------|---------------------------------|-------------------|
//! | GET    | `/api/orgs/{org_id}/spaces`     | List spaces       |
//! | POST   | `/api/orgs/{org_id}/spaces`     | Create space      |
//! | GET    | `/api/orgs/{org_id}/spaces/{id}`| Get space         |
//! | PATCH  | `/api/orgs/{org_id}/spaces/{id}`| Update space      |
//! | DELETE | `/api/orgs/{org_id}/spaces/{id}`| Delete space      |

use assistant_core::clock::{Clock, SystemClock};
use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, SpaceId};
use assistant_core::store::Space;
use assistant_core::{MembershipStore, SpaceStore};
use assistant_storage::OrgStorageLayer;

use crate::errors::json_error;
use crate::openapi::ErrorBody;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct SpacesApiState {
    pub org_storage: Arc<OrgStorageLayer>,
}

// -- Request/response types --------------------------------------------------

/// Summary of a space.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SpaceSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
}

/// Full space detail.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SpaceDetail {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Body for `POST /api/orgs/{org_id}/spaces`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateSpaceRequest {
    pub name: String,
    pub slug: String,
}

/// Body for `PATCH /api/orgs/{org_id}/spaces/{id}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateSpaceRequest {
    pub name: Option<String>,
}

// -- Router ------------------------------------------------------------------

pub fn spaces_api_router() -> Router<SpacesApiState> {
    Router::new()
        .route("/orgs/{org_id}/spaces", get(list_spaces).post(create_space))
        .route(
            "/orgs/{org_id}/spaces/{id}",
            get(get_space).patch(update_space).delete(delete_space),
        )
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/orgs/{org_id}/spaces` — list spaces filtered by membership.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/spaces",
    tag = "spaces",
    security(("bearer_token" = []), ("oauth2" = ["spaces:read"])),
    params(("org_id" = String, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "List of spaces", body = Vec<SpaceSummary>),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn list_spaces(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<SpacesApiState>,
    Path(org_id): Path<String>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let space_store = state.org_storage.space_store();

    if ctx.is_org_admin() {
        // Org admins see all spaces.
        match space_store.list_spaces(&org_id).await {
            Ok(spaces) => {
                let summaries: Vec<SpaceSummary> = spaces
                    .into_iter()
                    .map(|s| SpaceSummary {
                        id: s.id.0,
                        name: s.name,
                        slug: s.slug,
                    })
                    .collect();
                Json(summaries).into_response()
            }
            Err(e) => {
                tracing::error!("failed to list spaces: {e}");
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        }
    } else {
        // Non-admins see only spaces they're members of.
        let membership_store = state.org_storage.membership_store();
        let memberships = match membership_store
            .get_memberships_for_user(&ctx.user_id)
            .await
        {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("failed to get memberships: {e}");
                return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
            }
        };

        let mut summaries = Vec::new();
        for m in memberships {
            if let Ok(Some(space)) = space_store.get_space(&m.space_id).await {
                // Only include spaces belonging to the requested org.
                if space.org_id != org_id {
                    continue;
                }
                summaries.push(SpaceSummary {
                    id: space.id.0,
                    name: space.name,
                    slug: space.slug,
                });
            }
        }
        Json(summaries).into_response()
    }
}

/// `POST /api/orgs/{org_id}/spaces` — create a space (org-admin only).
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/spaces",
    tag = "spaces",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(("org_id" = String, Path, description = "Organization ID")),
    request_body = CreateSpaceRequest,
    responses(
        (status = 201, description = "Space created", body = SpaceDetail),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn create_space(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<SpacesApiState>,
    Path(org_id): Path<String>,
    Json(body): Json<CreateSpaceRequest>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }
    if !ctx.is_org_admin() {
        return json_error(StatusCode::FORBIDDEN, "org admin required");
    }

    if body.name.is_empty() || body.slug.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name and slug are required");
    }
    let now = SystemClock.now();
    let space = Space {
        id: SpaceId::from(format!("spc_{}", uuid::Uuid::new_v4())),
        org_id,
        name: body.name,
        slug: body.slug,
        created_at: now,
        updated_at: now,
    };

    let store = state.org_storage.space_store();
    if let Err(e) = store.create_space(&space).await {
        tracing::error!("failed to create space: {e}");
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to create space");
    }

    let detail = SpaceDetail {
        id: space.id.0,
        org_id: space.org_id.0,
        name: space.name,
        slug: space.slug,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    };
    (StatusCode::CREATED, Json(detail)).into_response()
}

/// `GET /api/orgs/{org_id}/spaces/{id}` — get space detail.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/spaces/{id}",
    tag = "spaces",
    security(("bearer_token" = []), ("oauth2" = ["spaces:read"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("id" = String, Path, description = "Space ID"),
    ),
    responses(
        (status = 200, description = "Space detail", body = SpaceDetail),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
    )
)]
pub async fn get_space(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<SpacesApiState>,
    Path((org_id, id)): Path<(String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let space_id = SpaceId::from(id);

    if !ctx.is_org_admin() && ctx.role_in(&space_id).is_none() {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let store = state.org_storage.space_store();
    match store.get_space(&space_id).await {
        Ok(Some(space)) if space.org_id == org_id => {
            let detail = SpaceDetail {
                id: space.id.0,
                org_id: space.org_id.0,
                name: space.name,
                slug: space.slug,
                created_at: space.created_at.to_rfc3339(),
                updated_at: space.updated_at.to_rfc3339(),
            };
            Json(detail).into_response()
        }
        Ok(Some(_)) => json_error(StatusCode::NOT_FOUND, "space not found"),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "space not found"),
        Err(e) => {
            tracing::error!("failed to get space: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

/// `PATCH /api/orgs/{org_id}/spaces/{id}` — update space name.
#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/spaces/{id}",
    tag = "spaces",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("id" = String, Path, description = "Space ID"),
    ),
    request_body = UpdateSpaceRequest,
    responses(
        (status = 200, description = "Space updated", body = SpaceDetail),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
    )
)]
pub async fn update_space(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<SpacesApiState>,
    Path((org_id, id)): Path<(String, String)>,
    Json(body): Json<UpdateSpaceRequest>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let space_id = SpaceId::from(id);

    // Space-admins or org-admins can update.
    if !ctx.is_org_admin() {
        match ctx.role_in(&space_id) {
            Some(assistant_core::identity::Role::SpaceAdmin) => {}
            _ => return json_error(StatusCode::FORBIDDEN, "space admin required"),
        }
    }

    let store = state.org_storage.space_store();
    let space = match store.get_space(&space_id).await {
        Ok(Some(s)) if s.org_id == org_id => s,
        Ok(Some(_)) => return json_error(StatusCode::NOT_FOUND, "space not found"),
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "space not found"),
        Err(e) => {
            tracing::error!("failed to get space: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    };

    let mut space = space;
    if let Some(name) = body.name {
        space.name = name;
    }
    space.updated_at = SystemClock.now();

    if let Err(e) = store.update_space(&space).await {
        tracing::error!("failed to update space: {e}");
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to update space");
    }

    let detail = SpaceDetail {
        id: space.id.0,
        org_id: space.org_id.0,
        name: space.name,
        slug: space.slug,
        created_at: space.created_at.to_rfc3339(),
        updated_at: space.updated_at.to_rfc3339(),
    };
    Json(detail).into_response()
}

/// `DELETE /api/orgs/{org_id}/spaces/{id}` — delete a space (org-admin only).
#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/spaces/{id}",
    tag = "spaces",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("id" = String, Path, description = "Space ID"),
    ),
    responses(
        (status = 204, description = "Space deleted"),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
    )
)]
pub async fn delete_space(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<SpacesApiState>,
    Path((org_id, id)): Path<(String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }
    if !ctx.is_org_admin() {
        return json_error(StatusCode::FORBIDDEN, "org admin required");
    }

    let space_id = SpaceId::from(id);
    let store = state.org_storage.space_store();

    // Verify space belongs to this org before deleting.
    match store.get_space(&space_id).await {
        Ok(Some(s)) if s.org_id != org_id => {
            return json_error(StatusCode::NOT_FOUND, "space not found");
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "space not found"),
        Err(e) => {
            tracing::error!("failed to get space: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
        _ => {}
    }

    match store.delete_space(&space_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "space not found"),
        Err(e) => {
            tracing::error!("failed to delete space: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to delete space")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::collections::HashMap;
    use tower::ServiceExt;

    use assistant_core::OrgStore;
    use assistant_core::identity::{Role, UserId};
    use assistant_core::store::Organization;

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

    fn test_app(state: SpacesApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(spaces_api_router().with_state(state))
            .layer(Extension(ctx))
    }

    async fn setup_state() -> SpacesApiState {
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        // Seed the org so FK constraints on spaces.org_id are satisfied.
        let org = Organization {
            id: OrgId::from("org_1"),
            name: "Test Org".into(),
            slug: "test".into(),
            auth_mode: "password".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        org_storage.org_store().create_org(&org).await.unwrap();
        SpacesApiState { org_storage }
    }

    #[tokio::test]
    async fn create_and_list_spaces() {
        let state = setup_state().await;
        let app = test_app(state.clone(), make_admin_ctx());

        // Create
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "Engineering",
                            "slug": "eng"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // List (admin sees all)
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_1/spaces")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let spaces: Vec<SpaceSummary> = serde_json::from_slice(&body).unwrap();
        assert_eq!(spaces.len(), 1);
        assert_eq!(spaces[0].name, "Engineering");
    }

    #[tokio::test]
    async fn member_cannot_create_space() {
        let state = setup_state().await;
        let app = test_app(state, make_member_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "Secret",
                            "slug": "secret"
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
    async fn delete_space_requires_admin() {
        let state = setup_state().await;

        // Create a space directly.
        let now = chrono::Utc::now();
        let space = Space {
            id: SpaceId::from("spc_del"),
            org_id: OrgId::from("org_1"),
            name: "To Delete".into(),
            slug: "del".into(),
            created_at: now,
            updated_at: now,
        };
        state
            .org_storage
            .space_store()
            .create_space(&space)
            .await
            .unwrap();

        // Member cannot delete.
        let app = test_app(state.clone(), make_member_ctx());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orgs/org_1/spaces/spc_del")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // Admin can delete.
        let app = test_app(state, make_admin_ctx());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orgs/org_1/spaces/spc_del")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}
