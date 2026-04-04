//! REST JSON API for skill discovery.
//!
//! ## Routes
//!
//! | Method | Path                           | Description              |
//! |--------|--------------------------------|--------------------------|
//! | GET    | `/api/personas/{id}/skills`    | List skills for persona  |

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use sqlx::SqlitePool;
use tracing::warn;

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

// -- Router ------------------------------------------------------------------

pub fn skills_router() -> Router<SkillsApiState> {
    Router::new().route("/personas/{persona_id}/skills", get(list_persona_skills))
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
    State(state): State<SkillsApiState>,
    Path(persona_id): Path<String>,
) -> Response {
    let persona_store = assistant_storage::personas::PersonaStore::new(state.pool.clone());

    // Verify the persona exists.
    match persona_store.get(&persona_id).await {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Persona not found"})),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Failed to get persona {persona_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
        _ => {}
    }

    let skill_access_store =
        assistant_storage::persona_skill_access::PersonaSkillAccessStore::new(state.pool.clone());

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

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use assistant_storage::{registry::SkillRegistry, StorageLayer};

    use super::{skills_router, SkillsApiState};

    async fn test_state() -> (SkillsApiState, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        storage.persona_store().ensure_default().await.unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let state = SkillsApiState {
            pool: storage.pool.clone(),
            registry,
        };
        (state, storage)
    }

    fn app(state: SkillsApiState) -> axum::Router {
        skills_router().with_state(state)
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&b).unwrap()
    }

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
}
