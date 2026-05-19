//! REST API for workflow management.
//!
//! All routes are mounted under `/api` by the main router.
//!
//! | Method | Path                                    | Description                     |
//! |--------|-----------------------------------------|---------------------------------|
//! | GET    | `/api/workflows`                        | List all workflows              |
//! | POST   | `/api/workflows`                        | Create a workflow               |
//! | GET    | `/api/workflows/{id}`                   | Get workflow detail             |
//! | PUT    | `/api/workflows/{id}`                   | Update a workflow               |
//! | DELETE | `/api/workflows/{id}`                   | Delete a workflow               |
//! | POST   | `/api/workflows/{id}/activate`          | Activate a workflow             |
//! | POST   | `/api/workflows/{id}/deactivate`        | Deactivate a workflow           |
//! | POST   | `/api/workflows/{id}/test-run`          | Queue a manual test run         |
//! | GET    | `/api/workflows/{id}/webhook-secrets`   | Reveal webhook URL + token      |
//! | GET    | `/api/workflows/{id}/runs`              | List recent runs                |
//! | GET    | `/api/workflows/{id}/runs/{run_id}`     | Run detail with steps           |

use assistant_core::clock::{Clock, SystemClock};
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use uuid::Uuid;

use assistant_storage::{SqliteWorkflowStore, WorkflowGraph, WorkflowStore, WorkflowTriggerKind};

use crate::api::internal_error;

// -- State -------------------------------------------------------------------

/// Shared state for workflow API handlers.
#[derive(Clone)]
pub struct WorkflowsApiState {
    pub pool: SqlitePool,
    pub agent_id: Arc<RwLock<String>>,
}

// -- Response types ----------------------------------------------------------

/// Workflow list item.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkflowSummary {
    /// Workflow UUID.
    pub id: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub node_count: usize,
    pub edge_count: usize,
    pub trigger_count: usize,
    pub action_count: usize,
    pub max_steps: u32,
    pub max_visits_per_node: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Full workflow detail including the graph definition.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkflowDetail {
    /// Workflow UUID.
    pub id: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    /// Workflow graph as a JSON object (nodes, edges, execution limits).
    pub graph: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Single workflow run.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkflowRunSummary {
    pub id: String,
    pub workflow_id: String,
    pub trigger_type: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub trigger_payload: Value,
}

/// Single step within a workflow run.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkflowRunStep {
    pub id: String,
    pub step_index: i64,
    pub node_id: String,
    pub node_kind: String,
    pub note: Option<String>,
    pub output: Option<Value>,
    pub occurred_at: DateTime<Utc>,
}

/// Workflow run detail including all steps.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkflowRunDetail {
    pub run: WorkflowRunSummary,
    pub steps: Vec<WorkflowRunStep>,
}

/// Response returned when a test-run is accepted.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkflowRunPreview {
    pub workflow_id: String,
    pub run_id: String,
    pub status: String,
    pub message: String,
    pub started_at: DateTime<Utc>,
    pub max_steps: u32,
    pub max_visits_per_node: u32,
}

/// Webhook URL and token for a workflow trigger.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkflowWebhookSecrets {
    pub webhook_url: String,
    pub webhook_token: String,
}

// -- Request types -----------------------------------------------------------

/// Body for `POST /api/workflows` and `PUT /api/workflows/{id}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct WorkflowUpsertRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Workflow graph as a JSON object.
    pub graph: Value,
    #[serde(default)]
    pub active: bool,
}

// -- Router ------------------------------------------------------------------

pub fn workflows_api_router() -> Router<WorkflowsApiState> {
    Router::new()
        .route("/workflows", get(list_workflows).post(create_workflow))
        .route(
            "/workflows/{id}",
            get(get_workflow)
                .put(update_workflow)
                .delete(delete_workflow),
        )
        .route("/workflows/{id}/activate", post(activate_workflow))
        .route("/workflows/{id}/deactivate", post(deactivate_workflow))
        .route("/workflows/{id}/test-run", post(test_run_workflow))
        .route(
            "/workflows/{id}/webhook-secrets",
            get(get_workflow_webhook_secrets),
        )
        .route("/workflows/{id}/runs", get(list_workflow_runs))
        .route("/workflows/{id}/runs/{run_id}", get(get_workflow_run))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/workflows` — list all workflows.
#[utoipa::path(
    get,
    path = "/api/workflows",
    tag = "workflows",
    responses(
        (status = 200, description = "List of workflows", body = Vec<WorkflowSummary>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_workflows(
    State(state): State<WorkflowsApiState>,
) -> Result<Json<Vec<WorkflowSummary>>, (StatusCode, String)> {
    let store = workflow_store(&state).await;
    let rows = store.list().await.map_err(internal_error)?;
    Ok(Json(rows.iter().map(to_summary).collect()))
}

/// `POST /api/workflows` — create a new workflow.
#[utoipa::path(
    post,
    path = "/api/workflows",
    tag = "workflows",
    request_body = WorkflowUpsertRequest,
    responses(
        (status = 201, description = "Created workflow", body = WorkflowDetail),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn create_workflow(
    State(state): State<WorkflowsApiState>,
    Json(req): Json<WorkflowUpsertRequest>,
) -> Result<(StatusCode, Json<WorkflowDetail>), (StatusCode, String)> {
    let graph = parse_graph(&req.graph)?;
    let store = workflow_store(&state).await;
    let id = store
        .create(req.name.trim(), req.description.trim(), &graph, req.active)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let saved = store.get(id).await.map_err(internal_error)?.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "created workflow missing".to_string(),
    ))?;
    Ok((StatusCode::CREATED, Json(to_detail(saved))))
}

/// `GET /api/workflows/{id}` — fetch a workflow.
#[utoipa::path(
    get,
    path = "/api/workflows/{id}",
    tag = "workflows",
    params(("id" = String, Path, description = "Workflow UUID")),
    responses(
        (status = 200, description = "Workflow detail", body = WorkflowDetail),
        (status = 400, description = "Invalid UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_workflow(
    State(state): State<WorkflowsApiState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowDetail>, (StatusCode, String)> {
    let uid = parse_uuid(&id)?;
    let store = workflow_store(&state).await;
    let w = store
        .get(uid)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;
    Ok(Json(to_detail(w)))
}

/// `PUT /api/workflows/{id}` — update a workflow.
#[utoipa::path(
    put,
    path = "/api/workflows/{id}",
    tag = "workflows",
    params(("id" = String, Path, description = "Workflow UUID")),
    request_body = WorkflowUpsertRequest,
    responses(
        (status = 200, description = "Updated workflow", body = WorkflowDetail),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn update_workflow(
    State(state): State<WorkflowsApiState>,
    Path(id): Path<String>,
    Json(req): Json<WorkflowUpsertRequest>,
) -> Result<Json<WorkflowDetail>, (StatusCode, String)> {
    let uid = parse_uuid(&id)?;
    let graph = parse_graph(&req.graph)?;
    let store = workflow_store(&state).await;
    let updated = store
        .update(
            uid,
            req.name.trim(),
            req.description.trim(),
            &graph,
            req.active,
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if !updated {
        return Err((StatusCode::NOT_FOUND, "Workflow not found".to_string()));
    }
    let w = store
        .get(uid)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;
    Ok(Json(to_detail(w)))
}

/// `DELETE /api/workflows/{id}` — delete a workflow.
#[utoipa::path(
    delete,
    path = "/api/workflows/{id}",
    tag = "workflows",
    params(("id" = String, Path, description = "Workflow UUID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 400, description = "Invalid UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn delete_workflow(
    State(state): State<WorkflowsApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uid = parse_uuid(&id)?;
    let store = workflow_store(&state).await;
    let deleted = store.delete(uid).await.map_err(internal_error)?;
    if !deleted {
        return Err((StatusCode::NOT_FOUND, "Workflow not found".to_string()));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/workflows/{id}/activate` — activate a workflow.
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/activate",
    tag = "workflows",
    params(("id" = String, Path, description = "Workflow UUID")),
    responses(
        (status = 200, description = "Activated workflow", body = WorkflowDetail),
        (status = 400, description = "Invalid UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn activate_workflow(
    State(state): State<WorkflowsApiState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowDetail>, (StatusCode, String)> {
    set_active(state, id, true).await
}

/// `POST /api/workflows/{id}/deactivate` — deactivate a workflow.
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/deactivate",
    tag = "workflows",
    params(("id" = String, Path, description = "Workflow UUID")),
    responses(
        (status = 200, description = "Deactivated workflow", body = WorkflowDetail),
        (status = 400, description = "Invalid UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn deactivate_workflow(
    State(state): State<WorkflowsApiState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowDetail>, (StatusCode, String)> {
    set_active(state, id, false).await
}

/// `POST /api/workflows/{id}/test-run` — queue a manual test run.
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/test-run",
    tag = "workflows",
    params(("id" = String, Path, description = "Workflow UUID")),
    responses(
        (status = 200, description = "Run accepted", body = WorkflowRunPreview),
        (status = 400, description = "Invalid UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn test_run_workflow(
    State(state): State<WorkflowsApiState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowRunPreview>, (StatusCode, String)> {
    let uid = parse_uuid(&id)?;
    let store = workflow_store(&state).await;
    let w = store
        .get(uid)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;

    let run_id = store
        .create_run(w.id, WorkflowTriggerKind::Manual, &serde_json::json!({}))
        .await
        .map_err(internal_error)?;

    Ok(Json(WorkflowRunPreview {
        workflow_id: w.id.to_string(),
        run_id: run_id.to_string(),
        status: "accepted".to_string(),
        message: "Workflow run accepted and queued for execution worker".to_string(),
        started_at: SystemClock.now(),
        max_steps: w.graph.execution.max_steps,
        max_visits_per_node: w.graph.execution.max_visits_per_node,
    }))
}

/// `GET /api/workflows/{id}/webhook-secrets` — reveal webhook URL and token.
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/webhook-secrets",
    tag = "workflows",
    params(("id" = String, Path, description = "Workflow UUID")),
    responses(
        (status = 200, description = "Webhook URL and token", body = WorkflowWebhookSecrets),
        (status = 400, description = "Invalid UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_workflow_webhook_secrets(
    State(state): State<WorkflowsApiState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowWebhookSecrets>, (StatusCode, String)> {
    let uid = parse_uuid(&id)?;
    let store = workflow_store(&state).await;
    let w = store
        .get(uid)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;
    let endpoint = store
        .get_webhook_endpoint(uid)
        .await
        .map_err(internal_error)?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Workflow webhook endpoint not found".to_string(),
        ))?;
    Ok(Json(WorkflowWebhookSecrets {
        webhook_url: format!("/workflow-hooks/{}/{}", w.id, endpoint.token),
        webhook_token: endpoint.token,
    }))
}

/// `GET /api/workflows/{id}/runs` — list recent runs (up to 50).
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/runs",
    tag = "workflows",
    params(("id" = String, Path, description = "Workflow UUID")),
    responses(
        (status = 200, description = "List of runs", body = Vec<WorkflowRunSummary>),
        (status = 400, description = "Invalid UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_workflow_runs(
    State(state): State<WorkflowsApiState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<WorkflowRunSummary>>, (StatusCode, String)> {
    let uid = parse_uuid(&id)?;
    let store = workflow_store(&state).await;
    let exists = store.get(uid).await.map_err(internal_error)?.is_some();
    if !exists {
        return Err((StatusCode::NOT_FOUND, "Workflow not found".to_string()));
    }
    let runs = store.list_runs(uid, 50).await.map_err(internal_error)?;
    Ok(Json(runs.into_iter().map(to_run_summary).collect()))
}

/// `GET /api/workflows/{id}/runs/{run_id}` — workflow run detail with steps.
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/runs/{run_id}",
    tag = "workflows",
    params(
        ("id" = String, Path, description = "Workflow UUID"),
        ("run_id" = String, Path, description = "Run UUID"),
    ),
    responses(
        (status = 200, description = "Run detail with steps", body = WorkflowRunDetail),
        (status = 400, description = "Invalid UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_workflow_run(
    State(state): State<WorkflowsApiState>,
    Path((id, run_id)): Path<(String, String)>,
) -> Result<Json<WorkflowRunDetail>, (StatusCode, String)> {
    let uid = parse_uuid(&id)?;
    let run_uid = parse_uuid(&run_id)?;
    let store = workflow_store(&state).await;
    let run = store
        .get_run(uid, run_uid)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Run not found".to_string()))?;
    let steps = store
        .list_run_steps(run_uid, 500)
        .await
        .map_err(internal_error)?;
    Ok(Json(WorkflowRunDetail {
        run: to_run_summary(run),
        steps: steps
            .into_iter()
            .map(|s| WorkflowRunStep {
                id: s.id.to_string(),
                step_index: s.step_index,
                node_id: s.node_id,
                node_kind: s.node_kind,
                note: s.note,
                output: s.output,
                occurred_at: s.occurred_at,
            })
            .collect(),
    }))
}

// -- Helpers -----------------------------------------------------------------

async fn workflow_store(state: &WorkflowsApiState) -> SqliteWorkflowStore {
    let agent_id = state.agent_id.read().await.clone();
    SqliteWorkflowStore::for_agent(state.pool.clone(), agent_id)
}

fn parse_uuid(s: &str) -> Result<Uuid, (StatusCode, String)> {
    Uuid::parse_str(s).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid workflow ID".to_string()))
}

fn parse_graph(v: &Value) -> Result<WorkflowGraph, (StatusCode, String)> {
    serde_json::from_value(v.clone())
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid graph: {e}")))
}

fn to_summary(w: &assistant_storage::WorkflowRecord) -> WorkflowSummary {
    let (triggers, actions) = count_nodes(&w.graph.nodes);
    WorkflowSummary {
        id: w.id.to_string(),
        name: w.name.clone(),
        description: w.description.clone(),
        active: w.active,
        node_count: w.graph.nodes.len(),
        edge_count: w.graph.edges.len(),
        trigger_count: triggers,
        action_count: actions,
        max_steps: w.graph.execution.max_steps,
        max_visits_per_node: w.graph.execution.max_visits_per_node,
        created_at: w.created_at,
        updated_at: w.updated_at,
    }
}

fn to_detail(w: assistant_storage::WorkflowRecord) -> WorkflowDetail {
    WorkflowDetail {
        id: w.id.to_string(),
        name: w.name,
        description: w.description,
        active: w.active,
        graph: serde_json::to_value(&w.graph).unwrap_or(Value::Null),
        created_at: w.created_at,
        updated_at: w.updated_at,
    }
}

fn to_run_summary(r: assistant_storage::WorkflowRunRecord) -> WorkflowRunSummary {
    WorkflowRunSummary {
        id: r.id.to_string(),
        workflow_id: r.workflow_id.to_string(),
        trigger_type: r.trigger_type,
        status: r.status,
        started_at: r.started_at,
        finished_at: r.finished_at,
        error_message: r.error_message,
        trigger_payload: r.trigger_payload,
    }
}

fn count_nodes(nodes: &[assistant_storage::WorkflowNode]) -> (usize, usize) {
    use assistant_storage::WorkflowNodeKind;
    let triggers = nodes
        .iter()
        .filter(|n| matches!(n.kind, WorkflowNodeKind::Trigger))
        .count();
    let actions = nodes.len() - triggers;
    (triggers, actions)
}

async fn set_active(
    state: WorkflowsApiState,
    id: String,
    active: bool,
) -> Result<Json<WorkflowDetail>, (StatusCode, String)> {
    let uid = parse_uuid(&id)?;
    let store = workflow_store(&state).await;
    let changed = store
        .set_active(uid, active)
        .await
        .map_err(internal_error)?;
    if !changed {
        return Err((StatusCode::NOT_FOUND, "Workflow not found".to_string()));
    }
    let w = store
        .get(uid)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;
    Ok(Json(to_detail(w)))
}

// -- Public webhook trigger --------------------------------------------------

/// Accepted response for a public webhook trigger.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkflowWebhookTriggerAccepted {
    workflow_id: String,
    run_id: String,
    status: String,
}

/// Public webhook trigger endpoint — no auth required.
///
/// Receives an inbound HTTP POST from an external system and queues a workflow
/// run with trigger type `Webhook`.  The `token` path parameter is the
/// per-workflow HMAC secret that acts as authentication.
#[utoipa::path(
    post,
    path = "/workflow-hooks/{id}/{token}",
    tag = "workflows",
    security(()),
    params(
        ("id" = String, Path, description = "Workflow ID"),
        ("token" = String, Path, description = "Webhook HMAC token"),
    ),
    request_body(content = serde_json::Value, content_type = "application/json"),
    responses(
        (status = 202, description = "Webhook accepted, run queued", body = WorkflowWebhookTriggerAccepted),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Workflow webhook not found"),
    )
)]
pub async fn public_webhook_trigger(
    State(state): State<WorkflowsApiState>,
    Path((id, token)): Path<(String, String)>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<WorkflowWebhookTriggerAccepted>), (StatusCode, String)> {
    use tracing::{info, warn};
    let workflow_id = parse_uuid(&id)?;
    let store = workflow_store(&state).await;
    let workflow = store
        .resolve_by_webhook(workflow_id, &token)
        .await
        .map_err(internal_error)?;

    let Some(workflow) = workflow else {
        warn!(workflow_id = %id, "Rejected workflow webhook trigger due to invalid token/workflow");
        return Err((
            StatusCode::NOT_FOUND,
            "Workflow webhook not found".to_string(),
        ));
    };

    let run_id = store
        .create_run(workflow.id, WorkflowTriggerKind::Webhook, &payload)
        .await
        .map_err(internal_error)?;
    info!(workflow_id = %workflow.id, run_id = %run_id, "Accepted workflow webhook trigger");

    Ok((
        StatusCode::ACCEPTED,
        Json(WorkflowWebhookTriggerAccepted {
            workflow_id: workflow.id.to_string(),
            run_id: run_id.to_string(),
            status: "accepted".to_string(),
        }),
    ))
}

/// Public router for inbound workflow webhook triggers.
///
/// Mount this **outside** the auth middleware — the token in the URL path
/// serves as the authentication credential.
pub fn workflow_public_router() -> Router<WorkflowsApiState> {
    Router::new().route("/workflow-hooks/{id}/{token}", post(public_webhook_trigger))
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    use assistant_storage::StorageLayer;

    use super::{WorkflowsApiState, workflows_api_router};

    async fn test_state() -> (WorkflowsApiState, std::sync::Arc<StorageLayer>) {
        let storage = std::sync::Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let state = WorkflowsApiState {
            pool: storage.pool.clone(),
            agent_id: std::sync::Arc::new(RwLock::new("default".to_string())),
        };
        (state, storage)
    }

    fn app(state: WorkflowsApiState) -> axum::Router {
        workflows_api_router().with_state(state)
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&b).unwrap()
    }

    fn minimal_graph() -> serde_json::Value {
        serde_json::json!({
            "version": 1,
            "nodes": [
                { "id": "t1", "kind": "trigger", "config": { "type": "manual" } },
                { "id": "a1", "kind": "action",  "config": { "type": "assistant_turn", "prompt": "hello" } },
            ],
            "edges": [{ "from": "t1", "to": "a1" }],
            "execution": { "max_steps": 100, "max_visits_per_node": 10 }
        })
    }

    #[tokio::test]
    async fn list_workflows_empty() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/workflows")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json, serde_json::json!([]), "empty list expected");
    }

    #[tokio::test]
    async fn create_workflow_returns_201() {
        let (state, _) = test_state().await;
        let body = serde_json::json!({
            "name": "My Flow",
            "description": "desc",
            "graph": minimal_graph(),
            "active": false,
        });
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/workflows")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED, "should be 201 Created");
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["name"], "My Flow");
    }

    #[tokio::test]
    async fn get_workflow_not_found_returns_404() {
        let (state, _) = test_state().await;
        let id = uuid::Uuid::new_v4();
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/workflows/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_workflow_returns_204() {
        let (state, _storage) = test_state().await;
        // Create first.
        let create_body = serde_json::json!({
            "name": "ToDelete",
            "graph": minimal_graph(),
            "active": false,
        });
        let create_resp = app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/workflows")
                    .header("Content-Type", "application/json")
                    .body(Body::from(create_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let json = body_json(create_resp.into_body()).await;
        let id = json["id"].as_str().unwrap().to_string();

        // Delete.
        let del_resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/workflows/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn list_runs_for_unknown_workflow_returns_404() {
        let (state, _) = test_state().await;
        let id = uuid::Uuid::new_v4();
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/workflows/{id}/runs"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ── Additional handler coverage ─────────────────────────────────────

    async fn create_and_get_id(state: WorkflowsApiState, name: &str, active: bool) -> String {
        let body = serde_json::json!({
            "name": name,
            "graph": minimal_graph(),
            "active": active,
        });
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/workflows")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        json["id"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn list_workflows_returns_created_items() {
        let (state, _) = test_state().await;
        let _ = create_and_get_id(state.clone(), "Alpha", false).await;
        let _ = create_and_get_id(state.clone(), "Beta", true).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/workflows")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        let names: Vec<&str> = arr.iter().map(|w| w["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"Alpha"));
        assert!(names.contains(&"Beta"));
    }

    #[tokio::test]
    async fn get_workflow_returns_detail_with_graph() {
        let (state, _) = test_state().await;
        let id = create_and_get_id(state.clone(), "Detailed", false).await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/workflows/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["name"], "Detailed");
        // Graph is serialised back in detail responses.
        assert!(
            json["graph"]["nodes"].is_array(),
            "detail must include graph"
        );
    }

    #[tokio::test]
    async fn update_workflow_renames_via_put() {
        let (state, _) = test_state().await;
        let id = create_and_get_id(state.clone(), "Old", false).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/workflows/{id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "name": "Renamed",
                            "description": "",
                            "graph": minimal_graph(),
                            "active": false,
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["name"], "Renamed");
    }

    #[tokio::test]
    async fn activate_and_deactivate_workflow_round_trip() {
        let (state, _) = test_state().await;
        let id = create_and_get_id(state.clone(), "Toggle", false).await;

        // Activate
        let resp = app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/workflows/{id}/activate"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "activate must succeed: {}",
            resp.status()
        );

        // Re-fetch to confirm active=true
        let detail_resp = app(state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/workflows/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let detail = body_json(detail_resp.into_body()).await;
        assert_eq!(detail["active"], true);

        // Deactivate
        let resp = app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/workflows/{id}/deactivate"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(resp.status().is_success());

        let detail_resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/workflows/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let detail = body_json(detail_resp.into_body()).await;
        assert_eq!(detail["active"], false);
    }

    #[tokio::test]
    async fn delete_workflow_unknown_returns_404() {
        let (state, _) = test_state().await;
        let id = uuid::Uuid::new_v4();
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/workflows/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_workflow_bad_uuid_returns_400() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/workflows/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_workflow_invalid_graph_returns_400() {
        let (state, _) = test_state().await;
        // Graph missing required fields.
        let body = serde_json::json!({
            "name": "Bad",
            "graph": { "version": 1 },
            "active": false,
        });
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/workflows")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_runs_for_existing_workflow_returns_empty_array() {
        let (state, _) = test_state().await;
        let id = create_and_get_id(state.clone(), "WithRuns", false).await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/workflows/{id}/runs"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json, serde_json::json!([]));
    }

    // ── Pure helpers ─────────────────────────────────────────────────────

    use super::{count_nodes, parse_graph, parse_uuid};

    #[test]
    fn parse_uuid_accepts_valid_string() {
        let s = uuid::Uuid::new_v4().to_string();
        assert!(parse_uuid(&s).is_ok());
    }

    #[test]
    fn parse_uuid_rejects_garbage() {
        let err = parse_uuid("not-a-uuid").unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(err.1.contains("workflow ID"));
    }

    #[test]
    fn parse_graph_round_trips_minimal_payload() {
        let v = serde_json::json!({
            "version": 1,
            "nodes": [
                { "id": "t1", "kind": "trigger", "config": { "type": "manual" } },
                { "id": "a1", "kind": "action", "config": { "type": "assistant_turn", "prompt": "hi" } },
            ],
            "edges": [{ "from": "t1", "to": "a1" }],
            "execution": { "max_steps": 100, "max_visits_per_node": 10 }
        });
        let g = parse_graph(&v).unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 1);
        let (triggers, actions) = count_nodes(&g.nodes);
        assert_eq!(triggers, 1);
        assert_eq!(actions, 1);
    }

    #[test]
    fn parse_graph_rejects_missing_fields() {
        let v = serde_json::json!({ "version": 1 });
        let err = parse_graph(&v).unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
}
