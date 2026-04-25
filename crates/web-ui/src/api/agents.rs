//! REST JSON API for A2A agent registry management.
//!
//! ## Routes
//!
//! | Method | Path                          | Description                    |
//! |--------|-------------------------------|--------------------------------|
//! | GET    | `/api/agents`                 | List all registered agents     |
//! | POST   | `/api/agents`                 | Register a new agent           |
//! | GET    | `/api/agents/{id}`            | Get agent detail               |
//! | PUT    | `/api/agents/{id}`            | Update an agent card           |
//! | DELETE | `/api/agents/{id}`            | Remove an agent                |
//! | POST   | `/api/agents/{id}/set-default`| Set an agent as the default    |

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use assistant_a2a_json_schema::agent_card::AgentCard;
use assistant_core::auth::AuthContext;

use crate::a2a::agent_store::{AgentStore, RegisteredAgent};

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct AgentsApiState {
    pub agent_store: AgentStore,
}

// -- Response types ----------------------------------------------------------

/// A brief summary of a registered agent.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AgentSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub is_default: bool,
    pub skill_count: usize,
    pub interface_count: usize,
}

/// Full agent detail, including the serialized agent card.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AgentDetail {
    pub id: String,
    pub is_default: bool,
    /// Full AgentCard serialized as JSON.
    pub card: serde_json::Value,
}

// -- Request types -----------------------------------------------------------

/// Body for `POST /api/agents`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RegisterAgentRequest {
    /// Full AgentCard JSON.
    pub card: serde_json::Value,
    /// If `true`, make this agent the default after registration.
    pub set_default: Option<bool>,
}

/// Body for `PUT /api/agents/{id}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateAgentRequest {
    /// Updated full AgentCard JSON.
    pub card: serde_json::Value,
}

// -- Helpers -----------------------------------------------------------------

fn to_summary(agent: &RegisteredAgent) -> AgentSummary {
    AgentSummary {
        id: agent.id.clone(),
        name: agent.card.name.clone(),
        version: agent.card.version.clone(),
        description: agent.card.description.clone(),
        is_default: agent.is_default,
        skill_count: agent.card.skills.len(),
        interface_count: agent.card.supported_interfaces.len(),
    }
}

fn to_detail(agent: RegisteredAgent) -> AgentDetail {
    let card = serde_json::to_value(&agent.card).unwrap_or_default();
    AgentDetail {
        id: agent.id,
        is_default: agent.is_default,
        card,
    }
}

// -- Router ------------------------------------------------------------------

pub fn agents_api_router() -> Router<AgentsApiState> {
    Router::new()
        .route("/agents", get(list_agents).post(register_agent))
        .route(
            "/agents/{id}",
            get(get_agent).put(update_agent).delete(delete_agent),
        )
        .route("/agents/{id}/set-default", post(set_default_agent))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/agents` — list all registered agents.
#[utoipa::path(
    get,
    path = "/api/agents",
    tag = "agents",
    responses(
        (status = 200, description = "List of registered agents", body = Vec<AgentSummary>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_agents(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<AgentsApiState>,
) -> Response {
    match state.agent_store.list().await {
        Ok(agents) => {
            let summaries: Vec<AgentSummary> = agents.iter().map(to_summary).collect();
            Json(summaries).into_response()
        }
        Err(e) => {
            warn!("Failed to list agents: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list agents").into_response()
        }
    }
}

/// `POST /api/agents` — register a new agent.
#[utoipa::path(
    post,
    path = "/api/agents",
    tag = "agents",
    request_body = RegisterAgentRequest,
    responses(
        (status = 201, description = "Agent registered", body = AgentDetail),
        (status = 400, description = "Invalid agent card"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = []))
)]
pub async fn register_agent(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<AgentsApiState>,
    Json(body): Json<RegisterAgentRequest>,
) -> Response {
    let card: AgentCard = match serde_json::from_value(body.card) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid agent card: {e}")})),
            )
                .into_response();
        }
    };

    let set_default = body.set_default.unwrap_or(false);
    match state.agent_store.register(card, set_default).await {
        Ok(id) => match state.agent_store.get(&id).await {
            Some(agent) => (StatusCode::CREATED, Json(to_detail(agent))).into_response(),
            None => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch registered agent",
            )
                .into_response(),
        },
        Err(e) => {
            warn!("Failed to register agent: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to register agent",
            )
                .into_response()
        }
    }
}

/// `GET /api/agents/{id}` — get an agent by ID.
#[utoipa::path(
    get,
    path = "/api/agents/{id}",
    tag = "agents",
    params(("id" = String, Path, description = "Agent ID")),
    responses(
        (status = 200, description = "Agent detail", body = AgentDetail),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_agent(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<AgentsApiState>,
    Path(id): Path<String>,
) -> Response {
    match state.agent_store.get(&id).await {
        Some(agent) => Json(to_detail(agent)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Agent not found"})),
        )
            .into_response(),
    }
}

/// `PUT /api/agents/{id}` — update an agent's card.
#[utoipa::path(
    put,
    path = "/api/agents/{id}",
    tag = "agents",
    params(("id" = String, Path, description = "Agent ID")),
    request_body = UpdateAgentRequest,
    responses(
        (status = 200, description = "Updated agent detail", body = AgentDetail),
        (status = 400, description = "Invalid agent card"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn update_agent(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<AgentsApiState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateAgentRequest>,
) -> Response {
    let card: AgentCard = match serde_json::from_value(body.card) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid agent card: {e}")})),
            )
                .into_response();
        }
    };

    if state.agent_store.update(&id, card).await {
        match state.agent_store.get(&id).await {
            Some(agent) => Json(to_detail(agent)).into_response(),
            None => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch updated agent",
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Agent not found"})),
        )
            .into_response()
    }
}

/// `DELETE /api/agents/{id}` — remove a registered agent.
#[utoipa::path(
    delete,
    path = "/api/agents/{id}",
    tag = "agents",
    params(("id" = String, Path, description = "Agent ID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn delete_agent(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<AgentsApiState>,
    Path(id): Path<String>,
) -> Response {
    if state.agent_store.remove(&id).await {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Agent not found"})),
        )
            .into_response()
    }
}

/// `POST /api/agents/{id}/set-default` — set an agent as the default.
#[utoipa::path(
    post,
    path = "/api/agents/{id}/set-default",
    tag = "agents",
    params(("id" = String, Path, description = "Agent ID")),
    responses(
        (status = 200, description = "Updated agent detail", body = AgentDetail),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn set_default_agent(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<AgentsApiState>,
    Path(id): Path<String>,
) -> Response {
    if state.agent_store.set_default(&id).await {
        match state.agent_store.get(&id).await {
            Some(agent) => Json(to_detail(agent)).into_response(),
            None => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch agent after set-default",
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Agent not found"})),
        )
            .into_response()
    }
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::Extension;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tempfile::TempDir;
    use tower::ServiceExt;

    use assistant_core::auth::AuthContext;
    use assistant_core::identity::{OrgId, UserId};

    use crate::a2a::agent_store::AgentStore;

    use super::{AgentsApiState, agents_api_router};

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

    async fn test_state() -> (AgentsApiState, TempDir) {
        let tmp = TempDir::new().unwrap();
        let store = AgentStore::new(tmp.path().to_path_buf()).unwrap();
        let state = AgentsApiState { agent_store: store };
        (state, tmp)
    }

    fn app(state: AgentsApiState) -> axum::Router {
        agents_api_router()
            .with_state(state)
            .layer(Extension(make_test_ctx()))
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&b).unwrap()
    }

    #[tokio::test]
    async fn list_agents_returns_empty_array() {
        let (state, _tmp) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/agents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert!(json.is_array(), "should return an array");
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn get_agent_unknown_returns_404() {
        let (state, _tmp) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/agents/no-such-agent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn register_agent_invalid_card_returns_400() {
        let (state, _tmp) = test_state().await;
        // Missing required `name` field.
        let body = r#"{"card":{},"set_default":false}"#;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/agents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_agent_unknown_returns_404() {
        let (state, _tmp) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/agents/no-such-agent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
