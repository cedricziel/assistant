//! REST JSON API for skill discovery and management.
//!
//! ## Routes
//!
//! | Method | Path                           | Description              |
//! |--------|--------------------------------|--------------------------|
//! | GET    | `/api/personas/{id}/skills`    | List skills for persona  |
//! | GET    | `/api/skills`                  | List all skills          |
//! | POST   | `/api/skills`                  | Create a skill           |
//! | GET    | `/api/skills/{name}`           | Get skill detail         |
//! | PUT    | `/api/skills/{name}`           | Update a skill           |
//! | DELETE | `/api/skills/{name}`           | Delete a skill           |

use assistant_storage::PersonaSkillAccessStore as _;
use assistant_storage::PersonaStore as _;
use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::warn;

use assistant_core::auth::AuthContext;
use assistant_storage::registry::SkillRegistry;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct SkillsApiState {
    pub pool: SqlitePool,
    pub registry: Arc<SkillRegistry>,
}

// -- Response types ----------------------------------------------------------

/// A skill entry returned by the API.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SkillEntryResponse {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Full skill detail returned by the API.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SkillDetail {
    pub name: String,
    pub description: String,
    pub body: String,
    pub source: String,
    pub is_builtin: bool,
}

// -- Request types -----------------------------------------------------------

/// Body for `POST /api/skills`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateSkillRequest {
    pub name: String,
    pub description: String,
    pub body: String,
}

/// Body for `PUT /api/skills/{name}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateSkillRequest {
    pub description: String,
    pub body: String,
}

// -- Router ------------------------------------------------------------------

pub fn skills_router() -> Router<SkillsApiState> {
    Router::new()
        .route("/personas/{persona_id}/skills", get(list_persona_skills))
        .route("/skills", get(list_skills).post(create_skill))
        .route(
            "/skills/{name}",
            get(get_skill).put(update_skill).delete(delete_skill),
        )
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/personas/{persona_id}/skills` — list skills for a persona.
#[utoipa::path(
    get,
    path = "/api/personas/{persona_id}/skills",
    tag = "skills",
    params(("persona_id" = String, Path, description = "Persona ID")),
    responses(
        (status = 200, description = "List of skills for the persona", body = Vec<SkillEntryResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Persona not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_persona_skills(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<SkillsApiState>,
    Path(persona_id): Path<String>,
) -> Response {
    let persona_store = assistant_storage::personas::SqlitePersonaStore::new(state.pool.clone());

    // Verify the persona exists and the caller owns it.
    match persona_store.get(&persona_id).await {
        Ok(Some(persona)) => {
            if persona
                .owner_user_id
                .as_ref()
                .is_some_and(|owner| owner != &ctx.user_id.0)
            {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "Access denied"})),
                )
                    .into_response();
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Persona not found"})),
            )
                .into_response();
        }
        Err(e) => {
            warn!("Failed to get persona {persona_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    }

    let skill_access_store =
        assistant_storage::persona_skill_access::SqlitePersonaSkillAccessStore::new(
            state.pool.clone(),
        );

    // Determine which skills are enabled for this persona.
    let mode = skill_access_store
        .get_mode(&persona_id)
        .await
        .unwrap_or_else(|_| "all".to_string());

    let listed_skills = skill_access_store
        .list_skill_names(&persona_id)
        .await
        .unwrap_or_default();

    let all_skills = state.registry.list().await;

    let entries: Vec<SkillEntryResponse> = all_skills
        .into_iter()
        .map(|s| {
            let enabled = match mode.as_str() {
                "whitelist" => listed_skills.contains(&s.name),
                "blacklist" => !listed_skills.contains(&s.name),
                _ => true, // "all" mode
            };
            SkillEntryResponse {
                name: s.name,
                description: s.description,
                enabled,
            }
        })
        .collect();

    Json(entries).into_response()
}

/// `GET /api/skills` — list all skills.
#[utoipa::path(
    get,
    path = "/api/skills",
    tag = "skills",
    responses(
        (status = 200, description = "List of all skills", body = Vec<SkillDetail>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_skills(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<SkillsApiState>,
) -> Response {
    let skills = state.registry.list().await;
    let details: Vec<SkillDetail> = skills
        .into_iter()
        .map(|s| {
            let is_builtin = s.is_builtin();
            SkillDetail {
                name: s.name,
                description: s.description,
                body: s.body,
                source: format!("{:?}", s.source).to_lowercase(),
                is_builtin,
            }
        })
        .collect();
    Json(details).into_response()
}

/// `POST /api/skills` — create a new user skill.
#[utoipa::path(
    post,
    path = "/api/skills",
    tag = "skills",
    request_body = CreateSkillRequest,
    responses(
        (status = 201, description = "Skill created", body = SkillDetail),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Skill already exists"),
    ),
    security(("bearer_token" = []))
)]
pub async fn create_skill(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<SkillsApiState>,
    Json(body): Json<CreateSkillRequest>,
) -> Response {
    match state
        .registry
        .create_user_skill(&body.name, &body.description, &body.body)
        .await
    {
        Ok(skill) => {
            let is_builtin = skill.is_builtin();
            let detail = SkillDetail {
                name: skill.name,
                description: skill.description,
                body: skill.body,
                source: format!("{:?}", skill.source).to_lowercase(),
                is_builtin,
            };
            (StatusCode::CREATED, Json(detail)).into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already exists") || msg.contains("UNIQUE") || msg.contains("unique") {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({"error": "Skill already exists"})),
                )
                    .into_response();
            }
            warn!("Failed to create skill {}: {e}", body.name);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create skill").into_response()
        }
    }
}

/// `GET /api/skills/{name}` — get a skill by name.
#[utoipa::path(
    get,
    path = "/api/skills/{name}",
    tag = "skills",
    params(("name" = String, Path, description = "Skill name")),
    responses(
        (status = 200, description = "Skill detail", body = SkillDetail),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Skill not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_skill(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<SkillsApiState>,
    Path(name): Path<String>,
) -> Response {
    match state.registry.get(&name).await {
        Some(skill) => {
            let is_builtin = skill.is_builtin();
            let detail = SkillDetail {
                name: skill.name,
                description: skill.description,
                body: skill.body,
                source: format!("{:?}", skill.source).to_lowercase(),
                is_builtin,
            };
            Json(detail).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Skill not found"})),
        )
            .into_response(),
    }
}

/// `PUT /api/skills/{name}` — update a user skill.
#[utoipa::path(
    put,
    path = "/api/skills/{name}",
    tag = "skills",
    params(("name" = String, Path, description = "Skill name")),
    request_body = UpdateSkillRequest,
    responses(
        (status = 200, description = "Updated skill detail", body = SkillDetail),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Cannot modify a builtin skill"),
        (status = 404, description = "Skill not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn update_skill(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<SkillsApiState>,
    Path(name): Path<String>,
    Json(body): Json<UpdateSkillRequest>,
) -> Response {
    match state
        .registry
        .update_user_skill(&name, &body.description, &body.body)
        .await
    {
        Ok(()) => match state.registry.get(&name).await {
            Some(skill) => {
                let is_builtin = skill.is_builtin();
                let detail = SkillDetail {
                    name: skill.name,
                    description: skill.description,
                    body: skill.body,
                    source: format!("{:?}", skill.source).to_lowercase(),
                    is_builtin,
                };
                Json(detail).into_response()
            }
            None => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Skill not found"})),
            )
                .into_response(),
        },
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") || msg.contains("does not exist") {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Skill not found"})),
                )
                    .into_response();
            }
            if msg.contains("builtin") || msg.contains("cannot modify") || msg.contains("read-only")
            {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "Cannot modify a builtin skill"})),
                )
                    .into_response();
            }
            warn!("Failed to update skill {name}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update skill").into_response()
        }
    }
}

/// `DELETE /api/skills/{name}` — delete a user skill.
#[utoipa::path(
    delete,
    path = "/api/skills/{name}",
    tag = "skills",
    params(("name" = String, Path, description = "Skill name")),
    responses(
        (status = 204, description = "Skill deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Skill not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn delete_skill(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<SkillsApiState>,
    Path(name): Path<String>,
) -> Response {
    match state.registry.delete_user_skill(&name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") || msg.contains("does not exist") {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Skill not found"})),
                )
                    .into_response();
            }
            warn!("Failed to delete skill {name}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete skill").into_response()
        }
    }
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use assistant_storage::PersonaStore as _;
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::Extension;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use assistant_core::auth::AuthContext;
    use assistant_core::identity::{OrgId, UserId};
    use assistant_storage::{StorageLayer, registry::SkillRegistry};

    use super::{SkillsApiState, skills_router};

    fn make_test_ctx() -> AuthContext {
        AuthContext {
            user_id: UserId::from("test-user"),
            org_id: OrgId::from("org_1"),
            email: "test@test.local".into(),
            space_roles: HashMap::new(),
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    async fn test_state() -> (SkillsApiState, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        storage
            .persona_store()
            .ensure_exists("default")
            .await
            .unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let state = SkillsApiState {
            pool: storage.pool.clone(),
            registry,
        };
        (state, storage)
    }

    fn app(state: SkillsApiState) -> axum::Router {
        skills_router()
            .with_state(state)
            .layer(Extension(make_test_ctx()))
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&b).unwrap()
    }

    // -- existing tests --

    #[tokio::test]
    async fn list_persona_skills_for_default_returns_ok() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/default/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert!(json.is_array(), "should return an array");
    }

    #[tokio::test]
    async fn list_persona_skills_unknown_persona_returns_404() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/nonexistent/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -- new tests --

    #[tokio::test]
    async fn list_skills_returns_array() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert!(json.is_array(), "should return an array");
    }

    #[tokio::test]
    async fn create_skill_returns_201() {
        let (state, _) = test_state().await;

        let body =
            r#"{"name":"my-skill","description":"A test skill","body":"My Skill body content."}"#;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/skills")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["name"], "my-skill");
        assert_eq!(json["description"], "A test skill");
        assert_eq!(json["is_builtin"], false);
    }

    #[tokio::test]
    async fn create_skill_duplicate_returns_409() {
        let (state, _) = test_state().await;

        let body = r#"{"name":"dup-skill","description":"Test","body":"body"}"#;

        // Create once.
        let resp1 = app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/skills")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp1.status(), StatusCode::CREATED);

        // Create again.
        let resp2 = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/skills")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp2.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn get_skill_unknown_returns_404() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/skills/no-such-skill")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_skill_updates_description_and_body() {
        let (state, _) = test_state().await;

        // Create the skill first.
        let create_body =
            r#"{"name":"update-skill","description":"Original","body":"original body"}"#;
        app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/skills")
                    .header("Content-Type", "application/json")
                    .body(Body::from(create_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Update it.
        let update_body = r#"{"description":"Updated desc","body":"updated body"}"#;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/skills/update-skill")
                    .header("Content-Type", "application/json")
                    .body(Body::from(update_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["description"], "Updated desc");
        assert_eq!(json["body"], "updated body");
    }

    #[tokio::test]
    async fn delete_skill_returns_204() {
        let (state, _) = test_state().await;

        // Create first.
        let create_body = r#"{"name":"del-skill","description":"To delete","body":"body content"}"#;
        app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/skills")
                    .header("Content-Type", "application/json")
                    .body(Body::from(create_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Delete it.
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/skills/del-skill")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_skill_unknown_returns_404() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/skills/no-such-skill")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_persona_skills_other_user_denied() {
        let (state, storage) = test_state().await;

        // Create a persona owned by another user.
        let persona_store = storage.persona_store();
        persona_store
            .create_owned("persona_other", "Other Persona", "other-user")
            .await
            .unwrap();

        // Our test user is "test-user" (from make_test_ctx), not "other-user".
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/persona_other/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "listing skills for another user's persona must be denied"
        );
    }

    #[tokio::test]
    async fn get_skill_returns_detail_for_existing_skill() {
        let (state, _) = test_state().await;
        // Seed via POST /skills.
        let body = r#"{"name":"detail-skill","description":"D","body":"B"}"#;
        app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/skills")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/skills/detail-skill")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["name"], "detail-skill");
        assert_eq!(json["description"], "D");
        assert_eq!(json["body"], "B");
        assert_eq!(json["is_builtin"], false);
    }

    #[tokio::test]
    async fn update_skill_unknown_returns_404() {
        let (state, _) = test_state().await;
        let body = r#"{"description":"x","body":"y"}"#;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/skills/no-such-skill")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::FORBIDDEN,
            "unknown or non-user skill must 404/403; got {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn delete_unknown_skill_returns_404_or_403() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/skills/no-such-skill-xyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::FORBIDDEN,
            "unknown or non-user skill must 404/403; got {}",
            resp.status()
        );
    }
}
