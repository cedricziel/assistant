//! REST JSON API for interface instance management.
//!
//! ## Routes
//!
//! | Method | Path                                                       | Description            |
//! |--------|------------------------------------------------------------|------------------------|
//! | POST   | `/api/orgs/{org_id}/spaces/{space_id}/interfaces`          | Create instance        |
//! | GET    | `/api/orgs/{org_id}/spaces/{space_id}/interfaces`          | List instances         |
//! | DELETE | `/api/orgs/{org_id}/spaces/{space_id}/interfaces/{id}`     | Delete instance        |

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
use assistant_core::store::{InterfaceInstance, InterfaceInstanceStore};
use assistant_storage::OrgStorageLayer;

use crate::errors::json_error;
use crate::openapi::ErrorBody;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct InterfacesApiState {
    pub org_storage: Arc<OrgStorageLayer>,
}

// -- Request/response types --------------------------------------------------

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InterfaceInstanceResponse {
    pub id: String,
    pub space_id: String,
    pub interface_type: String,
    pub config: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateInterfaceInstanceRequest {
    /// Interface type: `"slack"`, `"matrix"`, `"mattermost"`, `"signal"`, `"nextcloud"`.
    pub interface_type: String,
    /// JSON configuration (tokens, channels, etc.).
    #[serde(default = "default_config")]
    pub config: serde_json::Value,
}

fn default_config() -> serde_json::Value {
    serde_json::json!({})
}

// -- Router ------------------------------------------------------------------

pub fn interfaces_api_router() -> Router<InterfacesApiState> {
    Router::new()
        .route(
            "/orgs/{org_id}/spaces/{space_id}/interfaces",
            get(list_interfaces).post(create_interface),
        )
        .route(
            "/orgs/{org_id}/spaces/{space_id}/interfaces/{id}",
            axum::routing::delete(delete_interface),
        )
}

// -- Handlers ----------------------------------------------------------------

/// `POST /api/orgs/{org_id}/spaces/{space_id}/interfaces` — create an interface instance.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/spaces/{space_id}/interfaces",
    tag = "interfaces",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
    ),
    request_body = CreateInterfaceInstanceRequest,
    responses(
        (status = 201, description = "Created", body = InterfaceInstanceResponse),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn create_interface(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<InterfacesApiState>,
    Path((org_id, space_id)): Path<(String, String)>,
    Json(body): Json<CreateInterfaceInstanceRequest>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let space_id_typed = SpaceId::from(space_id);
    if !ctx.is_org_admin() {
        match ctx.role_in(&space_id_typed) {
            Some(assistant_core::identity::Role::SpaceAdmin) => {}
            _ => {
                return json_error(StatusCode::FORBIDDEN, "space admin or org admin required");
            }
        }
    }

    if body.interface_type.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "interface_type is required");
    }

    let config_value = body.config;
    let config_str = serde_json::to_string(&config_value).unwrap_or_else(|_| "{}".into());

    let instance = InterfaceInstance {
        id: format!("ii_{}", uuid::Uuid::new_v4()),
        org_id,
        space_id: space_id_typed,
        interface_type: body.interface_type,
        config: config_str,
        created_at: Utc::now(),
    };

    let store = state.org_storage.interface_instance_store();
    if let Err(e) = store.create_instance(&instance).await {
        tracing::error!("failed to create interface instance: {e}");
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to create instance");
    }

    let resp = InterfaceInstanceResponse {
        id: instance.id,
        space_id: instance.space_id.0,
        interface_type: instance.interface_type,
        config: config_value,
        created_at: instance.created_at.to_rfc3339(),
    };
    (StatusCode::CREATED, Json(resp)).into_response()
}

/// `GET /api/orgs/{org_id}/spaces/{space_id}/interfaces` — list interface instances.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/spaces/{space_id}/interfaces",
    tag = "interfaces",
    security(("bearer_token" = []), ("oauth2" = ["spaces:read"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
    ),
    responses(
        (status = 200, description = "Interface instances", body = Vec<InterfaceInstanceResponse>),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn list_interfaces(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<InterfacesApiState>,
    Path((org_id, space_id)): Path<(String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let space_id = SpaceId::from(space_id);
    let store = state.org_storage.interface_instance_store();
    match store.list_instances(&space_id).await {
        Ok(instances) => {
            let resp: Vec<_> = instances
                .iter()
                .map(|i| {
                    let config_value: serde_json::Value =
                        serde_json::from_str(&i.config).unwrap_or(serde_json::json!({}));
                    InterfaceInstanceResponse {
                        id: i.id.clone(),
                        space_id: i.space_id.0.clone(),
                        interface_type: i.interface_type.clone(),
                        config: config_value,
                        created_at: i.created_at.to_rfc3339(),
                    }
                })
                .collect();
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list interfaces: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

/// `DELETE /api/orgs/{org_id}/spaces/{space_id}/interfaces/{id}` — delete an interface instance.
#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/spaces/{space_id}/interfaces/{id}",
    tag = "interfaces",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
        ("id" = String, Path, description = "Interface instance ID"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
    )
)]
pub async fn delete_interface(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<InterfacesApiState>,
    Path((org_id, space_id, id)): Path<(String, String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let space_id = SpaceId::from(space_id);
    if !ctx.is_org_admin() {
        match ctx.role_in(&space_id) {
            Some(assistant_core::identity::Role::SpaceAdmin) => {}
            _ => {
                return json_error(StatusCode::FORBIDDEN, "space admin or org admin required");
            }
        }
    }

    let store = state.org_storage.interface_instance_store();

    // Verify the instance belongs to this org and space.
    match store.get_instance(&id).await {
        Ok(Some(inst)) if inst.org_id == org_id && inst.space_id == space_id => {}
        Ok(Some(_)) => return json_error(StatusCode::FORBIDDEN, "access denied"),
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "interface instance not found"),
        Err(e) => {
            tracing::error!("failed to get interface instance: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    }

    match store.delete_instance(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "interface instance not found"),
        Err(e) => {
            tracing::error!("failed to delete interface instance: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
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

    fn test_app(state: InterfacesApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(interfaces_api_router().with_state(state))
            .layer(Extension(ctx))
    }

    async fn setup_state() -> InterfacesApiState {
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
        InterfacesApiState { org_storage }
    }

    #[tokio::test]
    async fn create_and_list_interfaces() {
        let state = setup_state().await;
        let app = test_app(state, make_admin_ctx());

        // Create
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces/sp_eng/interfaces")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "interface_type": "slack",
                            "config": {"token": "xoxb-test"}
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
                    .uri("/orgs/org_1/spaces/sp_eng/interfaces")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let instances: Vec<InterfaceInstanceResponse> = serde_json::from_slice(&body).unwrap();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].interface_type, "slack");
    }

    #[tokio::test]
    async fn delete_interface_instance() {
        let state = setup_state().await;
        let app = test_app(state, make_admin_ctx());

        // Create
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces/sp_eng/interfaces")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "interface_type": "matrix"
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
        let inst: InterfaceInstanceResponse = serde_json::from_slice(&body).unwrap();

        // Delete
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/orgs/org_1/spaces/sp_eng/interfaces/{}", inst.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_interface_wrong_space_denied() {
        let state = setup_state().await;

        // Seed a second space.
        sqlx::query(
            "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_sales', 'org_1', 'Sales', 'sales')",
        )
        .execute(state.org_storage.pool())
        .await
        .unwrap();

        // Create an interface instance in sp_eng.
        let store = state.org_storage.interface_instance_store();
        store
            .create_instance(&InterfaceInstance {
                id: "ii_eng".into(),
                org_id: OrgId::from("org_1"),
                space_id: SpaceId::from("sp_eng"),
                interface_type: "slack".into(),
                config: "{}".into(),
                created_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        let app = test_app(state, make_admin_ctx());

        // Try to delete sp_eng's instance via the sp_sales path.
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orgs/org_1/spaces/sp_sales/interfaces/ii_eng")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "deleting an interface via the wrong space path must be denied"
        );
    }

    #[tokio::test]
    async fn delete_interface_cross_org_denied() {
        let state = setup_state().await;

        // Seed org_other with a space and interface.
        sqlx::query(
            "INSERT INTO organizations (id, name, slug) VALUES ('org_other', 'Evil', 'evil')",
        )
        .execute(state.org_storage.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_evil', 'org_other', 'Evil Space', 'evil')",
        )
        .execute(state.org_storage.pool())
        .await
        .unwrap();
        let store = state.org_storage.interface_instance_store();
        store
            .create_instance(&InterfaceInstance {
                id: "ii_evil".into(),
                org_id: OrgId::from("org_other"),
                space_id: SpaceId::from("sp_evil"),
                interface_type: "slack".into(),
                config: "{}".into(),
                created_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        let app = test_app(state, make_admin_ctx());

        // Try to delete org_other's instance via org_1's path.
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orgs/org_1/spaces/sp_eng/interfaces/ii_evil")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "deleting a cross-org interface instance must be denied"
        );
    }
}
