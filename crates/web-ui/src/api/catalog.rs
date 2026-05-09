//! REST JSON API for catalog management and subscriptions.
//!
//! ## Routes
//!
//! | Method | Path                                                          | Description                |
//! |--------|---------------------------------------------------------------|----------------------------|
//! | POST   | `/api/orgs/{org_id}/catalog`                                  | Publish to catalog         |
//! | GET    | `/api/orgs/{org_id}/catalog`                                  | List catalog items         |
//! | GET    | `/api/orgs/{org_id}/catalog/type/{type}`                      | List catalog by type       |
//! | DELETE | `/api/orgs/{org_id}/catalog/{item_id}`                        | Remove from catalog        |
//! | POST   | `/api/orgs/{org_id}/spaces/{space_id}/subscriptions`          | Subscribe space            |
//! | GET    | `/api/orgs/{org_id}/spaces/{space_id}/subscriptions`          | List subscriptions         |
//! | DELETE | `/api/orgs/{org_id}/spaces/{space_id}/subscriptions/{sub_id}` | Unsubscribe                |

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

use assistant_core::CatalogSubscription;
use assistant_core::auth::AuthContext;
use assistant_core::catalog::CatalogResourceType;
use assistant_core::identity::OrgId;
use assistant_core::store::{CatalogItem, CatalogItemStore, CatalogSubscriptionStore};
use assistant_storage::OrgStorageLayer;

use crate::errors::json_error;
use crate::openapi::ErrorBody;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct CatalogApiState {
    pub org_storage: Arc<OrgStorageLayer>,
}

// -- Request/response types --------------------------------------------------

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CatalogItemResponse {
    pub id: String,
    pub resource_type: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PublishCatalogItemRequest {
    /// Resource type: `"skill"`, `"template"`, or `"interface"`.
    #[schema(value_type = String, example = "skill")]
    pub resource_type: CatalogResourceType,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SubscriptionResponse {
    pub id: String,
    pub catalog_item_id: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateSubscriptionRequest {
    pub catalog_item_id: String,
}

// -- Router ------------------------------------------------------------------

pub fn catalog_api_router() -> Router<CatalogApiState> {
    Router::new()
        .route(
            "/orgs/{org_id}/catalog",
            get(list_catalog).post(publish_catalog_item),
        )
        .route(
            "/orgs/{org_id}/catalog/type/{type}",
            get(list_catalog_by_type),
        )
        .route(
            "/orgs/{org_id}/catalog/{item_id}",
            axum::routing::delete(delete_catalog_item),
        )
        .route(
            "/orgs/{org_id}/spaces/{space_id}/subscriptions",
            get(list_subscriptions).post(create_subscription),
        )
        .route(
            "/orgs/{org_id}/spaces/{space_id}/subscriptions/{sub_id}",
            axum::routing::delete(delete_subscription),
        )
}

// -- Helpers -----------------------------------------------------------------

fn parse_resource_type(s: &str) -> Option<CatalogResourceType> {
    match s {
        "skill" | "skills" => Some(CatalogResourceType::Skill),
        "template" | "templates" => Some(CatalogResourceType::Template),
        "interface" | "interfaces" => Some(CatalogResourceType::Interface),
        _ => None,
    }
}

fn item_to_response(item: &CatalogItem) -> CatalogItemResponse {
    CatalogItemResponse {
        id: item.id.clone(),
        resource_type: item.resource_type.to_string(),
        name: item.name.clone(),
        description: item.description.clone(),
        created_at: item.created_at.to_rfc3339(),
    }
}

// -- Handlers: catalog items -------------------------------------------------

/// `POST /api/orgs/{org_id}/catalog` — publish a resource to the org catalog.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/catalog",
    tag = "catalog",
    security(("bearer_token" = []), ("oauth2" = ["org:manage"])),
    params(("org_id" = String, Path, description = "Organization ID")),
    request_body = PublishCatalogItemRequest,
    responses(
        (status = 201, description = "Published", body = CatalogItemResponse),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn publish_catalog_item(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<CatalogApiState>,
    Path(org_id): Path<String>,
    Json(body): Json<PublishCatalogItemRequest>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }
    if !ctx.is_org_admin() {
        return json_error(StatusCode::FORBIDDEN, "org admin required");
    }

    if body.name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required");
    }

    let item = CatalogItem {
        id: format!("ci_{}", uuid::Uuid::new_v4()),
        org_id,
        resource_type: body.resource_type,
        name: body.name,
        description: body.description,
        created_at: Utc::now(),
    };

    let store = state.org_storage.catalog_item_store();
    if let Err(e) = store.create_item(&item).await {
        tracing::error!("failed to publish catalog item: {e}");
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to publish");
    }

    (StatusCode::CREATED, Json(item_to_response(&item))).into_response()
}

/// `GET /api/orgs/{org_id}/catalog` — list all catalog items.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/catalog",
    tag = "catalog",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(("org_id" = String, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Catalog items", body = Vec<CatalogItemResponse>),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn list_catalog(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<CatalogApiState>,
    Path(org_id): Path<String>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let store = state.org_storage.catalog_item_store();
    match store.list_items(&org_id, None).await {
        Ok(items) => {
            let resp: Vec<_> = items.iter().map(item_to_response).collect();
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list catalog: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

/// `GET /api/orgs/{org_id}/catalog/type/{type}` — list catalog items by type.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/catalog/type/{type}",
    tag = "catalog",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("type" = String, Path, description = "Resource type: skill, template, interface"),
    ),
    responses(
        (status = 200, description = "Catalog items by type", body = Vec<CatalogItemResponse>),
        (status = 400, description = "Invalid type", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn list_catalog_by_type(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<CatalogApiState>,
    Path((org_id, type_str)): Path<(String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let resource_type = match parse_resource_type(&type_str) {
        Some(rt) => rt,
        None => {
            return json_error(StatusCode::BAD_REQUEST, "invalid resource type");
        }
    };

    let store = state.org_storage.catalog_item_store();
    match store.list_items(&org_id, Some(&resource_type)).await {
        Ok(items) => {
            let resp: Vec<_> = items.iter().map(item_to_response).collect();
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list catalog by type: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

/// `DELETE /api/orgs/{org_id}/catalog/{item_id}` — remove from catalog.
#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/catalog/{item_id}",
    tag = "catalog",
    security(("bearer_token" = []), ("oauth2" = ["org:manage"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("item_id" = String, Path, description = "Catalog item ID"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
    )
)]
pub async fn delete_catalog_item(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<CatalogApiState>,
    Path((org_id, item_id)): Path<(String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }
    if !ctx.is_org_admin() {
        return json_error(StatusCode::FORBIDDEN, "org admin required");
    }

    let store = state.org_storage.catalog_item_store();

    // Verify the item belongs to this org.
    match store.get_item(&item_id).await {
        Ok(Some(item)) if item.org_id == org_id => {}
        Ok(Some(_)) => return json_error(StatusCode::FORBIDDEN, "access denied"),
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "catalog item not found"),
        Err(e) => {
            tracing::error!("failed to get catalog item: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    }

    match store.delete_item(&item_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "catalog item not found"),
        Err(e) => {
            tracing::error!("failed to delete catalog item: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

// -- Handlers: subscriptions -------------------------------------------------

/// `POST /api/orgs/{org_id}/spaces/{space_id}/subscriptions` — subscribe a space to a catalog item.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/spaces/{space_id}/subscriptions",
    tag = "catalog",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
    ),
    request_body = CreateSubscriptionRequest,
    responses(
        (status = 201, description = "Subscribed", body = SubscriptionResponse),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn create_subscription(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<CatalogApiState>,
    Path((org_id, space_id)): Path<(String, String)>,
    Json(body): Json<CreateSubscriptionRequest>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }
    if !ctx.is_org_admin() {
        let space_id_ref = assistant_core::identity::SpaceId::from(space_id.clone());
        match ctx.role_in(&space_id_ref) {
            Some(assistant_core::identity::Role::SpaceAdmin) => {}
            _ => {
                return json_error(StatusCode::FORBIDDEN, "space admin or org admin required");
            }
        }
    }

    // Verify catalog item belongs to this org.
    let catalog_store = state.org_storage.catalog_item_store();
    match catalog_store.get_item(&body.catalog_item_id).await {
        Ok(Some(item)) if item.org_id == org_id => {}
        Ok(Some(_)) => {
            return json_error(StatusCode::FORBIDDEN, "catalog item belongs to another org");
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "catalog item not found"),
        Err(e) => {
            tracing::error!("failed to get catalog item: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    }

    let now = Utc::now();
    let sub = CatalogSubscription {
        id: format!("sub_{}", uuid::Uuid::new_v4()),
        space_id: assistant_core::identity::SpaceId::from(space_id),
        catalog_item_id: body.catalog_item_id,
        created_at: now,
    };

    let store = state.org_storage.catalog_subscription_store();
    if let Err(e) = store.create_subscription(&sub).await {
        tracing::error!("failed to create subscription: {e}");
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to subscribe");
    }

    let resp = SubscriptionResponse {
        id: sub.id,
        catalog_item_id: sub.catalog_item_id,
        created_at: now.to_rfc3339(),
    };
    (StatusCode::CREATED, Json(resp)).into_response()
}

/// `GET /api/orgs/{org_id}/spaces/{space_id}/subscriptions` — list subscriptions.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/spaces/{space_id}/subscriptions",
    tag = "catalog",
    security(("bearer_token" = []), ("oauth2" = ["spaces:read"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
    ),
    responses(
        (status = 200, description = "Subscriptions", body = Vec<SubscriptionResponse>),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn list_subscriptions(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<CatalogApiState>,
    Path((org_id, space_id)): Path<(String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let space_id = assistant_core::identity::SpaceId::from(space_id);
    let store = state.org_storage.catalog_subscription_store();
    match store.list_subscriptions(&space_id).await {
        Ok(subs) => {
            let resp: Vec<_> = subs
                .iter()
                .map(|s| SubscriptionResponse {
                    id: s.id.clone(),
                    catalog_item_id: s.catalog_item_id.clone(),
                    created_at: s.created_at.to_rfc3339(),
                })
                .collect();
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list subscriptions: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

/// `DELETE /api/orgs/{org_id}/spaces/{space_id}/subscriptions/{sub_id}` — unsubscribe.
#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/spaces/{space_id}/subscriptions/{sub_id}",
    tag = "catalog",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
        ("sub_id" = String, Path, description = "Subscription ID"),
    ),
    responses(
        (status = 204, description = "Unsubscribed"),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
    )
)]
pub async fn delete_subscription(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<CatalogApiState>,
    Path((org_id, space_id, sub_id)): Path<(String, String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let space_id = assistant_core::identity::SpaceId::from(space_id);
    if !ctx.is_org_admin() {
        match ctx.role_in(&space_id) {
            Some(assistant_core::identity::Role::SpaceAdmin) => {}
            _ => {
                return json_error(StatusCode::FORBIDDEN, "space admin or org admin required");
            }
        }
    }

    let store = state.org_storage.catalog_subscription_store();

    // Verify the subscription belongs to this space.
    match store.get_subscription(&sub_id).await {
        Ok(Some(sub)) if sub.space_id == space_id => {}
        Ok(Some(_)) => {
            return json_error(
                StatusCode::FORBIDDEN,
                "subscription belongs to another space",
            );
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "subscription not found"),
        Err(e) => {
            tracing::error!("failed to get subscription: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    }

    match store.delete_subscription(&sub_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "subscription not found"),
        Err(e) => {
            tracing::error!("failed to delete subscription: {e}");
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

    use assistant_core::identity::{Role, SpaceId, UserId};

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

    fn make_member_ctx() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("sp_eng"), Role::Member);
        AuthContext {
            user_id: UserId::from("alice"),
            org_id: OrgId::from("org_1"),
            email: "alice@test.local".into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    fn test_app(state: CatalogApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(catalog_api_router().with_state(state))
            .layer(Extension(ctx))
    }

    async fn setup_state() -> CatalogApiState {
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        // Seed org + space for FK constraints.
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
        CatalogApiState { org_storage }
    }

    #[tokio::test]
    async fn publish_and_list_catalog() {
        let state = setup_state().await;
        let app = test_app(state.clone(), make_admin_ctx());

        // Publish a skill
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/catalog")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "resource_type": "skill",
                            "name": "web-fetch",
                            "description": "Fetch web pages"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // List all
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_1/catalog")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let items: Vec<CatalogItemResponse> = serde_json::from_slice(&body).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "web-fetch");

        // List by type
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_1/catalog/type/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn member_cannot_publish() {
        let state = setup_state().await;
        let app = test_app(state, make_member_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/catalog")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "resource_type": "skill",
                            "name": "evil-skill"
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
    async fn subscription_lifecycle() {
        let state = setup_state().await;
        let app = test_app(state.clone(), make_admin_ctx());

        // Publish an item first
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/catalog")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "resource_type": "skill",
                            "name": "web-fetch"
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
        let item: CatalogItemResponse = serde_json::from_slice(&body).unwrap();

        // Subscribe
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces/sp_eng/subscriptions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "catalog_item_id": item.id
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
        let sub: SubscriptionResponse = serde_json::from_slice(&body).unwrap();

        // List subscriptions
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_1/spaces/sp_eng/subscriptions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let subs: Vec<SubscriptionResponse> = serde_json::from_slice(&body).unwrap();
        assert_eq!(subs.len(), 1);

        // Delete subscription
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/orgs/org_1/spaces/sp_eng/subscriptions/{}",
                        sub.id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn cross_tenant_catalog_denied() {
        let state = setup_state().await;
        let app = test_app(state, make_admin_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_other/catalog")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn subscribe_to_cross_org_catalog_item_denied() {
        let state = setup_state().await;

        // Seed a catalog item owned by org_other.
        sqlx::query(
            "INSERT INTO organizations (id, name, slug) VALUES ('org_other', 'Evil', 'evil')",
        )
        .execute(state.org_storage.pool())
        .await
        .unwrap();
        let catalog = state.org_storage.catalog_item_store();
        catalog
            .create_item(&assistant_core::store::CatalogItem {
                id: "ci_other".into(),
                org_id: OrgId::from("org_other"),
                resource_type: assistant_core::catalog::CatalogResourceType::Skill,
                name: "stolen-skill".into(),
                description: "".into(),
                created_at: Utc::now(),
            })
            .await
            .unwrap();

        let app = test_app(state, make_admin_ctx());

        // Try to subscribe org_1/sp_eng to org_other's catalog item.
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces/sp_eng/subscriptions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "catalog_item_id": "ci_other"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "subscribing to a cross-org catalog item must be denied"
        );
    }

    #[tokio::test]
    async fn delete_subscription_wrong_space_denied() {
        let state = setup_state().await;

        // Seed a second space.
        sqlx::query(
            "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_sales', 'org_1', 'Sales', 'sales')",
        )
        .execute(state.org_storage.pool())
        .await
        .unwrap();

        // Publish a catalog item and subscribe sp_eng.
        let catalog = state.org_storage.catalog_item_store();
        catalog
            .create_item(&assistant_core::store::CatalogItem {
                id: "ci_1".into(),
                org_id: OrgId::from("org_1"),
                resource_type: assistant_core::catalog::CatalogResourceType::Skill,
                name: "web-fetch".into(),
                description: "".into(),
                created_at: Utc::now(),
            })
            .await
            .unwrap();

        let sub_store = state.org_storage.catalog_subscription_store();
        let sub = assistant_core::CatalogSubscription {
            id: "sub_1".into(),
            space_id: assistant_core::identity::SpaceId::from("sp_eng"),
            catalog_item_id: "ci_1".into(),
            created_at: Utc::now(),
        };
        sub_store.create_subscription(&sub).await.unwrap();

        let app = test_app(state, make_admin_ctx());

        // Try to delete sp_eng's subscription via the sp_sales path.
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orgs/org_1/spaces/sp_sales/subscriptions/sub_1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "deleting a subscription via the wrong space path must be denied"
        );
    }

    #[tokio::test]
    async fn delete_catalog_item_cross_org_denied() {
        let state = setup_state().await;

        // Seed a catalog item owned by org_other.
        sqlx::query(
            "INSERT INTO organizations (id, name, slug) VALUES ('org_other', 'Evil', 'evil')",
        )
        .execute(state.org_storage.pool())
        .await
        .unwrap();
        let catalog = state.org_storage.catalog_item_store();
        catalog
            .create_item(&assistant_core::store::CatalogItem {
                id: "ci_other".into(),
                org_id: OrgId::from("org_other"),
                resource_type: assistant_core::catalog::CatalogResourceType::Skill,
                name: "stolen-skill".into(),
                description: "".into(),
                created_at: Utc::now(),
            })
            .await
            .unwrap();

        let app = test_app(state, make_admin_ctx());

        // Try to delete org_other's item via org_1's path.
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orgs/org_1/catalog/ci_other")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "deleting a cross-org catalog item must be denied"
        );
    }
}
