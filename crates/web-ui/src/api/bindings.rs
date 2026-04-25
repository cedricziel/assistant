//! REST JSON API for persona ↔ interface bindings.
//!
//! ## Routes
//!
//! | Method | Path                                                     | Description       |
//! |--------|----------------------------------------------------------|-------------------|
//! | POST   | `/api/orgs/{org_id}/spaces/{space_id}/bindings`          | Create binding    |
//! | GET    | `/api/orgs/{org_id}/spaces/{space_id}/bindings`          | List bindings     |
//! | DELETE | `/api/orgs/{org_id}/spaces/{space_id}/bindings/{id}`     | Delete binding    |

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

use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, SpaceId};
use assistant_core::store::{BindingStore, PersonaBinding};
use assistant_storage::OrgStorageLayer;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct BindingsApiState {
    pub org_storage: Arc<OrgStorageLayer>,
}

// -- Request/response types --------------------------------------------------

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BindingResponse {
    pub id: String,
    pub persona_id: String,
    pub interface_instance_id: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateBindingRequest {
    pub persona_id: String,
    pub interface_instance_id: String,
}

// -- Router ------------------------------------------------------------------

pub fn bindings_api_router() -> Router<BindingsApiState> {
    Router::new()
        .route(
            "/orgs/{org_id}/spaces/{space_id}/bindings",
            get(list_bindings).post(create_binding),
        )
        .route(
            "/orgs/{org_id}/spaces/{space_id}/bindings/{id}",
            axum::routing::delete(delete_binding),
        )
}

// -- Handlers ----------------------------------------------------------------

/// `POST /api/orgs/{org_id}/spaces/{space_id}/bindings` — bind a persona to an interface.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/spaces/{space_id}/bindings",
    tag = "bindings",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
    ),
    request_body = CreateBindingRequest,
    responses(
        (status = 201, description = "Binding created", body = BindingResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn create_binding(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<BindingsApiState>,
    Path((org_id, space_id)): Path<(String, String)>,
    Json(body): Json<CreateBindingRequest>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return (StatusCode::FORBIDDEN, "access denied").into_response();
    }

    let space_id = SpaceId::from(space_id);
    if !ctx.is_org_admin() {
        match ctx.role_in(&space_id) {
            Some(assistant_core::identity::Role::SpaceAdmin) => {}
            _ => {
                return (StatusCode::FORBIDDEN, "space admin or org admin required")
                    .into_response();
            }
        }
    }

    if body.persona_id.is_empty() || body.interface_instance_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "persona_id and interface_instance_id are required",
        )
            .into_response();
    }

    let now = Utc::now();
    let binding = PersonaBinding {
        id: format!("bind_{}", uuid::Uuid::new_v4()),
        space_id,
        persona_id: body.persona_id,
        interface_instance_id: body.interface_instance_id,
        created_at: now,
    };

    let store = state.org_storage.binding_store();
    if let Err(e) = store.create_binding(&binding).await {
        tracing::error!("failed to create binding: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create binding",
        )
            .into_response();
    }

    let resp = BindingResponse {
        id: binding.id,
        persona_id: binding.persona_id,
        interface_instance_id: binding.interface_instance_id,
        created_at: now.to_rfc3339(),
    };
    (StatusCode::CREATED, Json(resp)).into_response()
}

/// `GET /api/orgs/{org_id}/spaces/{space_id}/bindings` — list bindings.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/spaces/{space_id}/bindings",
    tag = "bindings",
    security(("bearer_token" = []), ("oauth2" = ["spaces:read"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
    ),
    responses(
        (status = 200, description = "Bindings", body = Vec<BindingResponse>),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn list_bindings(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<BindingsApiState>,
    Path((org_id, space_id)): Path<(String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return (StatusCode::FORBIDDEN, "access denied").into_response();
    }

    let space_id = SpaceId::from(space_id);
    let store = state.org_storage.binding_store();
    match store.list_bindings(&space_id).await {
        Ok(bindings) => {
            let resp: Vec<_> = bindings
                .iter()
                .map(|b| BindingResponse {
                    id: b.id.clone(),
                    persona_id: b.persona_id.clone(),
                    interface_instance_id: b.interface_instance_id.clone(),
                    created_at: b.created_at.to_rfc3339(),
                })
                .collect();
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list bindings: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
        }
    }
}

/// `DELETE /api/orgs/{org_id}/spaces/{space_id}/bindings/{id}` — delete a binding.
#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/spaces/{space_id}/bindings/{id}",
    tag = "bindings",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
        ("id" = String, Path, description = "Binding ID"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn delete_binding(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<BindingsApiState>,
    Path((org_id, _space_id, id)): Path<(String, String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return (StatusCode::FORBIDDEN, "access denied").into_response();
    }
    if !ctx.is_org_admin() {
        return (StatusCode::FORBIDDEN, "org admin required").into_response();
    }

    let store = state.org_storage.binding_store();
    match store.delete_binding(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "binding not found").into_response(),
        Err(e) => {
            tracing::error!("failed to delete binding: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
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

    use assistant_core::identity::{Role, UserId};

    fn make_admin_ctx() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("sp_eng"), Role::OrgAdmin);
        AuthContext {
            user_id: UserId::from("admin"),
            org_id: OrgId::from("org_1"),
            email: "admin@test.local".into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    fn test_app(state: BindingsApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(bindings_api_router().with_state(state))
            .layer(Extension(ctx))
    }

    async fn setup_state() -> BindingsApiState {
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        sqlx::query("INSERT INTO organizations (id, name, slug) VALUES ('org_1', 'Acme', 'acme')")
            .execute(org_storage.pool())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_eng', 'org_1', 'Engineering', 'eng')",
        )
        .execute(org_storage.pool())
        .await
        .unwrap();
        // Seed an interface instance for FK constraint.
        sqlx::query(
            "INSERT INTO interface_instances (id, org_id, space_id, interface_type, config)
             VALUES ('ii_1', 'org_1', 'sp_eng', 'slack', '{}')",
        )
        .execute(org_storage.pool())
        .await
        .unwrap();
        BindingsApiState { org_storage }
    }

    #[tokio::test]
    async fn create_and_list_bindings() {
        let state = setup_state().await;
        let app = test_app(state, make_admin_ctx());

        // Create
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces/sp_eng/bindings")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "persona_id": "persona_ops",
                            "interface_instance_id": "ii_1"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // List
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_1/spaces/sp_eng/bindings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let bindings: Vec<BindingResponse> = serde_json::from_slice(&body).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].persona_id, "persona_ops");
    }

    #[tokio::test]
    async fn delete_binding_lifecycle() {
        let state = setup_state().await;
        let app = test_app(state, make_admin_ctx());

        // Create
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces/sp_eng/bindings")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "persona_id": "persona_ops",
                            "interface_instance_id": "ii_1"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let binding: BindingResponse = serde_json::from_slice(&body).unwrap();

        // Delete
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(&format!(
                        "/orgs/org_1/spaces/sp_eng/bindings/{}",
                        binding.id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}
