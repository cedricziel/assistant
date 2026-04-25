//! REST JSON API for persona template instantiation and onboarding.
//!
//! ## Routes
//!
//! | Method | Path                                                                  | Description               |
//! |--------|-----------------------------------------------------------------------|---------------------------|
//! | GET    | `/api/orgs/{org_id}/catalog/templates`                                | List templates            |
//! | POST   | `/api/orgs/{org_id}/spaces/{space_id}/personas/from-template`         | Create persona from tpl   |
//! | GET    | `/api/users/me/onboarding-status`                                     | Onboarding status         |

use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use assistant_core::auth::AuthContext;
use assistant_core::catalog::CatalogResourceType;
use assistant_core::identity::OrgId;
use assistant_core::store::CatalogItemStore;
use assistant_storage::OrgStorageLayer;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct TemplatesApiState {
    pub org_storage: Arc<OrgStorageLayer>,
    /// Main assistant.db pool for persona operations.
    pub pool: sqlx::SqlitePool,
}

// -- Request/response types --------------------------------------------------

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TemplateResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateFromTemplateRequest {
    /// Catalog item ID of the template to instantiate.
    pub template_id: String,
    /// Name for the new persona (optional — defaults to template name).
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PersonaFromTemplateResponse {
    pub persona_id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OnboardingStatusResponse {
    /// Whether the user has created at least one persona.
    pub has_persona: bool,
}

// -- Router ------------------------------------------------------------------

pub fn templates_api_router() -> Router<TemplatesApiState> {
    Router::new()
        .route("/orgs/{org_id}/catalog/templates", get(list_templates))
        .route(
            "/orgs/{org_id}/spaces/{space_id}/personas/from-template",
            post(create_from_template),
        )
        .route("/users/me/onboarding-status", get(onboarding_status))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/orgs/{org_id}/catalog/templates` — list available persona templates.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/catalog/templates",
    tag = "templates",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(("org_id" = String, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Templates", body = Vec<TemplateResponse>),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn list_templates(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<TemplatesApiState>,
    Path(org_id): Path<String>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return (StatusCode::FORBIDDEN, "access denied").into_response();
    }

    let store = state.org_storage.catalog_item_store();
    match store
        .list_items(&org_id, Some(&CatalogResourceType::Template))
        .await
    {
        Ok(items) => {
            let resp: Vec<_> = items
                .iter()
                .map(|i| TemplateResponse {
                    id: i.id.clone(),
                    name: i.name.clone(),
                    description: i.description.clone(),
                    created_at: i.created_at.to_rfc3339(),
                })
                .collect();
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list templates: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
        }
    }
}

/// `POST /api/orgs/{org_id}/spaces/{space_id}/personas/from-template` — create a persona from a template.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/spaces/{space_id}/personas/from-template",
    tag = "templates",
    security(("bearer_token" = []), ("oauth2" = ["spaces:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("space_id" = String, Path, description = "Space ID"),
    ),
    request_body = CreateFromTemplateRequest,
    responses(
        (status = 201, description = "Persona created", body = PersonaFromTemplateResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Template not found"),
    )
)]
pub async fn create_from_template(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<TemplatesApiState>,
    Path((org_id, _space_id)): Path<(String, String)>,
    Json(body): Json<CreateFromTemplateRequest>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return (StatusCode::FORBIDDEN, "access denied").into_response();
    }

    // Resolve the template from the catalog.
    let catalog = state.org_storage.catalog_item_store();
    let template = match catalog.get_item(&body.template_id).await {
        Ok(Some(item)) if item.resource_type == CatalogResourceType::Template => item,
        Ok(Some(_)) => {
            return (StatusCode::BAD_REQUEST, "catalog item is not a template").into_response();
        }
        Ok(None) => return (StatusCode::NOT_FOUND, "template not found").into_response(),
        Err(e) => {
            tracing::error!("failed to get template: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response();
        }
    };

    let persona_name = body.name.unwrap_or_else(|| template.name.clone());
    let persona_id = format!("persona_{}", uuid::Uuid::new_v4());
    let now = Utc::now();

    // Insert into the personas table in the main assistant.db.
    let result = sqlx::query(
        "INSERT INTO personas (id, name, is_default, owner_user_id, created_at, updated_at)
         VALUES (?, ?, 0, ?, ?, ?)",
    )
    .bind(&persona_id)
    .bind(&persona_name)
    .bind(&ctx.user_id.0)
    .bind(now)
    .bind(now)
    .execute(&state.pool)
    .await;

    if let Err(e) = result {
        tracing::error!("failed to create persona from template: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create persona",
        )
            .into_response();
    }

    let resp = PersonaFromTemplateResponse {
        persona_id,
        name: persona_name,
    };
    (StatusCode::CREATED, Json(resp)).into_response()
}

/// `GET /api/users/me/onboarding-status` — check if user has created at least one persona.
#[utoipa::path(
    get,
    path = "/api/users/me/onboarding-status",
    tag = "templates",
    security(("bearer_token" = []), ("oauth2" = [])),
    responses(
        (status = 200, description = "Onboarding status", body = OnboardingStatusResponse),
    )
)]
pub async fn onboarding_status(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<TemplatesApiState>,
) -> Response {
    let has_persona: bool =
        sqlx::query_scalar("SELECT COUNT(*) > 0 FROM personas WHERE owner_user_id = ?")
            .bind(&ctx.user_id.0)
            .fetch_one(&state.pool)
            .await
            .unwrap_or(false);

    Json(OnboardingStatusResponse { has_persona }).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_storage::StorageLayer;
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

    fn test_app(state: TemplatesApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(templates_api_router().with_state(state))
            .layer(Extension(ctx))
    }

    async fn setup_state() -> TemplatesApiState {
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

        // Create an in-memory assistant.db for persona operations.
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let pool = storage.pool.clone();

        TemplatesApiState { org_storage, pool }
    }

    #[tokio::test]
    async fn list_templates_empty() {
        let state = setup_state().await;
        let app = test_app(state, make_admin_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_1/catalog/templates")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let templates: Vec<TemplateResponse> = serde_json::from_slice(&body).unwrap();
        assert!(templates.is_empty());
    }

    #[tokio::test]
    async fn create_persona_from_template() {
        let state = setup_state().await;

        // Publish a template to the catalog.
        let catalog = state.org_storage.catalog_item_store();
        let now = Utc::now();
        catalog
            .create_item(&assistant_core::store::CatalogItem {
                id: "tpl_1".into(),
                org_id: OrgId::from("org_1"),
                resource_type: CatalogResourceType::Template,
                name: "chatbot".into(),
                description: "A friendly chatbot template".into(),
                created_at: now,
            })
            .await
            .unwrap();

        let app = test_app(state, make_admin_ctx());

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/spaces/sp_eng/personas/from-template")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "template_id": "tpl_1",
                            "name": "My Chatbot"
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
        let persona: PersonaFromTemplateResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(persona.name, "My Chatbot");
        assert!(persona.persona_id.starts_with("persona_"));
    }

    #[tokio::test]
    async fn onboarding_status_no_persona() {
        let state = setup_state().await;
        let app = test_app(state, make_admin_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/users/me/onboarding-status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: OnboardingStatusResponse = serde_json::from_slice(&body).unwrap();
        assert!(!status.has_persona);
    }
}
