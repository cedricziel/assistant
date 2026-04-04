//! REST JSON API for persona management.
//!
//! ## Routes
//!
//! | Method | Path                        | Description                  |
//! |--------|-----------------------------|------------------------------|
//! | GET    | `/api/personas`             | List all personas            |
//! | POST   | `/api/personas/active`      | Switch active persona        |

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tracing::warn;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct PersonaApiState {
    pub pool: SqlitePool,
    /// Shared live agent ID — updated when the user switches personas.
    pub agent_id: Arc<RwLock<String>>,
}

// -- Response types ----------------------------------------------------------

/// A persona summary returned by the API.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PersonaSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

// -- Request types -----------------------------------------------------------

/// Body for `POST /api/personas/active`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SetActivePersonaRequest {
    pub id: String,
}

// -- Router ------------------------------------------------------------------

/// Build the personas API sub-router.  Mounted under `/api`.
pub fn personas_router() -> Router<PersonaApiState> {
    Router::new()
        .route("/personas", get(list_personas))
        .route("/personas/active", post(set_active_persona))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/personas` — list all personas defined on the server.
#[utoipa::path(
    get,
    path = "/api/personas",
    tag = "personas",
    responses(
        (status = 200, description = "List of personas", body = Vec<PersonaSummary>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_personas(State(state): State<PersonaApiState>) -> Response {
    let store = assistant_storage::personas::PersonaStore::new(state.pool);
    match store.list().await {
        Ok(personas) => {
            let summaries: Vec<PersonaSummary> = personas
                .into_iter()
                .map(|p| PersonaSummary {
                    id: p.id,
                    name: p.name,
                    description: String::new(),
                    is_default: p.is_default,
                })
                .collect();
            Json(summaries).into_response()
        }
        Err(e) => {
            warn!("Failed to list personas: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list personas").into_response()
        }
    }
}

/// `POST /api/personas/active` — switch the active persona for the session.
#[utoipa::path(
    post,
    path = "/api/personas/active",
    tag = "personas",
    request_body = SetActivePersonaRequest,
    responses(
        (status = 200, description = "Switched persona", body = PersonaSummary),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Persona not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn set_active_persona(
    State(state): State<PersonaApiState>,
    Json(body): Json<SetActivePersonaRequest>,
) -> Response {
    let id = body.id.trim().to_string();
    if id.is_empty() {
        return (StatusCode::BAD_REQUEST, "Persona ID is required").into_response();
    }

    let store = assistant_storage::personas::PersonaStore::new(state.pool.clone());

    // Verify the persona exists.
    let persona = match store.get(&id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Persona not found"})),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Failed to get persona {id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Update the shared agent ID.
    *state.agent_id.write().await = id.clone();

    Json(PersonaSummary {
        id: persona.id,
        name: persona.name,
        description: String::new(),
        is_default: persona.is_default,
    })
    .into_response()
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    use assistant_storage::StorageLayer;

    use super::{personas_router, PersonaApiState};

    async fn test_state() -> (PersonaApiState, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        storage.persona_store().ensure_default().await.unwrap();
        let state = PersonaApiState {
            pool: storage.pool.clone(),
            agent_id: Arc::new(RwLock::new("default".to_string())),
        };
        (state, storage)
    }

    fn app(state: PersonaApiState) -> axum::Router {
        personas_router().with_state(state)
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&b).unwrap()
    }

    #[tokio::test]
    async fn list_personas_returns_default() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        let arr = json.as_array().unwrap();
        assert!(!arr.is_empty(), "should have at least the default persona");
        assert_eq!(arr[0]["id"], "default");
    }

    #[tokio::test]
    async fn set_active_persona_updates_agent_id() {
        let (state, storage) = test_state().await;

        // Create a second persona.
        storage
            .persona_store()
            .create("work", "Work Mode")
            .await
            .unwrap();

        let agent_id_ref = state.agent_id.clone();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/personas/active")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"id":"work"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["id"], "work");
        assert_eq!(*agent_id_ref.read().await, "work");
    }

    #[tokio::test]
    async fn set_active_persona_unknown_id_returns_404() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/personas/active")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"id":"nonexistent"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
