//! Server-side rendered pages and JSON API for workflow graphs.

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Redirect, Response};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

use assistant_storage::{
    WorkflowExecutionLimits, WorkflowGraph, WorkflowNode, WorkflowNodeKind, WorkflowRecord,
    WorkflowRunRecord, WorkflowRunStepRecord, WorkflowStore, WorkflowTriggerKind,
};

use crate::common::{internal_error, render_template, StaticUrls};

/// Shared state for workflow handlers.
#[derive(Clone)]
pub struct WorkflowPagesState {
    pub pool: SqlitePool,
}

struct WorkflowListRow {
    id: String,
    short_id: String,
    name: String,
    description: String,
    active: bool,
    node_count: usize,
    edge_count: usize,
    trigger_count: usize,
    action_count: usize,
    max_steps: u32,
    max_visits_per_node: u32,
}

#[derive(Template)]
#[template(path = "workflows/list.html")]
struct WorkflowListTemplate {
    active_page: &'static str,
    count: usize,
    rows: Vec<WorkflowListRow>,
}

impl StaticUrls for WorkflowListTemplate {}

#[derive(Template)]
#[template(path = "workflows/form.html")]
struct WorkflowFormTemplate {
    active_page: &'static str,
    heading: String,
    action: String,
    submit_label: String,
    name: String,
    description: String,
    active_checked: bool,
    graph_json: String,
}

impl StaticUrls for WorkflowFormTemplate {}

#[derive(Template)]
#[template(path = "workflows/detail.html")]
struct WorkflowDetailTemplate {
    active_page: &'static str,
    id: String,
    short_id: String,
    name: String,
    description: String,
    active: bool,
    toggle_label: String,
    node_count: usize,
    edge_count: usize,
    trigger_count: usize,
    action_count: usize,
    max_steps: u32,
    max_visits_per_node: u32,
    created_at: String,
    updated_at: String,
    graph_json: String,
    webhook_url: String,
    webhook_token: String,
    webhook_updated_at: String,
    recent_runs: Vec<WorkflowRunView>,
}

impl StaticUrls for WorkflowDetailTemplate {}

#[derive(Template)]
#[template(path = "workflows/editor.html")]
struct WorkflowEditorTemplate {
    active_page: &'static str,
    id: String,
    name: String,
    graph_json: String,
}

impl StaticUrls for WorkflowEditorTemplate {}

struct WorkflowRunView {
    id: String,
    run_id: String,
    trigger_type: String,
    status: String,
    started_at: String,
}

#[derive(Template)]
#[template(path = "workflows/run_detail.html")]
struct WorkflowRunDetailTemplate {
    active_page: &'static str,
    workflow_id: String,
    workflow_name: String,
    run_id: String,
    trigger_type: String,
    status: String,
    started_at: String,
    finished_at: String,
    error_message: String,
    has_error: bool,
    trigger_payload_json: String,
    steps: Vec<WorkflowRunStepView>,
}

impl StaticUrls for WorkflowRunDetailTemplate {}

struct WorkflowRunStepView {
    step_index: i64,
    node_id: String,
    node_kind: String,
    note_text: String,
    output_json: String,
    occurred_at: String,
}

#[derive(Deserialize)]
pub struct WorkflowFormData {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    graph_json: String,
    active: Option<String>,
}

/// `GET /workflows` -- List all workflows.
pub async fn list_workflows(
    State(state): State<WorkflowPagesState>,
) -> Result<Response, (StatusCode, String)> {
    let store = WorkflowStore::new(state.pool);
    let workflows = store.list().await.map_err(internal_error)?;
    let count = workflows.len();
    let rows = workflows
        .iter()
        .map(to_row)
        .collect::<Vec<WorkflowListRow>>();

    Ok(render_template(WorkflowListTemplate {
        active_page: "workflows",
        count,
        rows,
    }))
}

/// `GET /workflows/new` -- Create form.
pub async fn new_workflow_form(State(_state): State<WorkflowPagesState>) -> Response {
    let tmpl = WorkflowFormTemplate {
        active_page: "workflows",
        heading: "Create Workflow".to_string(),
        action: "/workflows".to_string(),
        submit_label: "Create".to_string(),
        name: String::new(),
        description: String::new(),
        active_checked: true,
        graph_json: default_graph_json(),
    };
    render_template(tmpl)
}

/// `POST /workflows` -- Create workflow.
pub async fn create_workflow(
    State(state): State<WorkflowPagesState>,
    Form(form): Form<WorkflowFormData>,
) -> Response {
    let store = WorkflowStore::new(state.pool);
    let graph = match parse_graph_json(&form.graph_json) {
        Ok(graph) => graph,
        Err(err) => return (StatusCode::BAD_REQUEST, err).into_response(),
    };

    match store
        .create(
            form.name.trim(),
            form.description.trim(),
            &graph,
            form.active.is_some(),
        )
        .await
    {
        Ok(id) => {
            info!(workflow_id = %id, name = %form.name, "Workflow created");
            Redirect::to(&format!("/workflows/{id}")).into_response()
        }
        Err(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    }
}

/// `GET /workflows/:id` -- Workflow detail page.
pub async fn show_workflow(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let store = WorkflowStore::new(state.pool);
    let workflow = store
        .get(workflow_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;

    let (trigger_count, action_count) = count_node_types(&workflow.graph.nodes);
    let graph_json =
        serde_json::to_string_pretty(&workflow.graph).unwrap_or_else(|_| "{}".to_string());
    let short_id = id[..8.min(id.len())].to_string();
    let endpoint = store
        .ensure_webhook_endpoint(workflow_id)
        .await
        .map_err(internal_error)?;
    let webhook_url = format!("/workflow-hooks/{}/{}", id, endpoint.token);
    let recent_runs = store
        .list_runs(workflow_id, 10)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(to_run_view)
        .collect();

    Ok(render_template(WorkflowDetailTemplate {
        active_page: "workflows",
        id,
        short_id,
        name: workflow.name,
        description: workflow.description,
        active: workflow.active,
        toggle_label: if workflow.active {
            "Disable".to_string()
        } else {
            "Enable".to_string()
        },
        node_count: workflow.graph.nodes.len(),
        edge_count: workflow.graph.edges.len(),
        trigger_count,
        action_count,
        max_steps: workflow.graph.execution.max_steps,
        max_visits_per_node: workflow.graph.execution.max_visits_per_node,
        created_at: format_ts(workflow.created_at),
        updated_at: format_ts(workflow.updated_at),
        graph_json,
        webhook_url,
        webhook_token: endpoint.token,
        webhook_updated_at: format_ts(endpoint.updated_at),
        recent_runs,
    }))
}

/// `GET /workflows/:id/edit` -- Edit form.
pub async fn edit_workflow_form(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let store = WorkflowStore::new(state.pool);
    let workflow = store
        .get(workflow_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;

    let graph_json =
        serde_json::to_string_pretty(&workflow.graph).unwrap_or_else(|_| default_graph_json());

    Ok(render_template(WorkflowFormTemplate {
        active_page: "workflows",
        heading: "Edit Workflow".to_string(),
        action: format!("/workflows/{id}/edit"),
        submit_label: "Save Changes".to_string(),
        name: workflow.name,
        description: workflow.description,
        active_checked: workflow.active,
        graph_json,
    }))
}

/// `POST /workflows/:id/edit` -- Update workflow.
pub async fn update_workflow(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
    Form(form): Form<WorkflowFormData>,
) -> Response {
    let workflow_id = match parse_uuid(&id) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };

    let graph = match parse_graph_json(&form.graph_json) {
        Ok(graph) => graph,
        Err(err) => return (StatusCode::BAD_REQUEST, err).into_response(),
    };

    let store = WorkflowStore::new(state.pool);
    match store
        .update(
            workflow_id,
            form.name.trim(),
            form.description.trim(),
            &graph,
            form.active.is_some(),
        )
        .await
    {
        Ok(true) => Redirect::to(&format!("/workflows/{id}")).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Workflow not found").into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    }
}

/// `POST /workflows/:id/delete` -- Delete workflow.
pub async fn delete_workflow(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Response {
    let workflow_id = match parse_uuid(&id) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };

    let store = WorkflowStore::new(state.pool);
    match store.delete(workflow_id).await {
        Ok(true) => Redirect::to("/workflows").into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Workflow not found").into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

/// `POST /workflows/:id/toggle` -- Toggle active state.
pub async fn toggle_workflow(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Response {
    let workflow_id = match parse_uuid(&id) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };

    let store = WorkflowStore::new(state.pool.clone());
    let Some(workflow) = store.get(workflow_id).await.unwrap_or(None) else {
        return (StatusCode::NOT_FOUND, "Workflow not found").into_response();
    };

    match store.set_active(workflow_id, !workflow.active).await {
        Ok(true) => Redirect::to(&format!("/workflows/{id}")).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Workflow not found").into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

/// `POST /workflows/:id/webhook/rotate` -- Rotate inbound webhook token.
pub async fn rotate_workflow_webhook(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Response {
    let workflow_id = match parse_uuid(&id) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };

    let store = WorkflowStore::new(state.pool);
    match store.get(workflow_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "Workflow not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }

    let rotate_result = match store.get_webhook_endpoint(workflow_id).await {
        Ok(Some(_)) => store.rotate_webhook_token(workflow_id).await,
        Ok(None) => store.ensure_webhook_endpoint(workflow_id).await.map(Some),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    };

    match rotate_result {
        Ok(Some(_)) => Redirect::to(&format!("/workflows/{id}")).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Webhook endpoint not found").into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

/// `GET /workflows/:id/editor` -- Graph editor page.
pub async fn editor_page(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let store = WorkflowStore::new(state.pool);
    let workflow = store
        .get(workflow_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;

    Ok(render_template(WorkflowEditorTemplate {
        active_page: "workflows",
        id,
        name: workflow.name,
        graph_json: serde_json::to_string_pretty(&workflow.graph)
            .unwrap_or_else(|_| default_graph_json()),
    }))
}

/// `GET /workflows/:id/runs/:run_id` -- Workflow run detail page.
pub async fn show_workflow_run(
    State(state): State<WorkflowPagesState>,
    Path((id, run_id)): Path<(String, String)>,
) -> Result<Response, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let run_uuid = parse_uuid(&run_id)?;
    let store = WorkflowStore::new(state.pool);

    let workflow = store
        .get(workflow_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;
    let run = store
        .get_run(workflow_id, run_uuid)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow run not found".to_string()))?;
    let steps = store
        .list_run_steps(run_uuid, 500)
        .await
        .map_err(internal_error)?;

    Ok(render_template(WorkflowRunDetailTemplate {
        active_page: "workflows",
        workflow_id: workflow.id.to_string(),
        workflow_name: workflow.name,
        run_id,
        trigger_type: run.trigger_type,
        status: run.status,
        started_at: format_ts(run.started_at),
        finished_at: run
            .finished_at
            .map(format_ts)
            .unwrap_or_else(|| "-".to_string()),
        has_error: run.error_message.is_some(),
        error_message: run.error_message.unwrap_or_default(),
        trigger_payload_json: serde_json::to_string_pretty(&run.trigger_payload)
            .unwrap_or_else(|_| "{}".to_string()),
        steps: steps.into_iter().map(to_run_step_view).collect(),
    }))
}

#[derive(Serialize)]
pub struct WorkflowApiItem {
    id: String,
    name: String,
    description: String,
    active: bool,
    node_count: usize,
    edge_count: usize,
    trigger_count: usize,
    action_count: usize,
    max_steps: u32,
    max_visits_per_node: u32,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
pub struct WorkflowApiDetail {
    id: String,
    name: String,
    description: String,
    active: bool,
    graph: WorkflowGraph,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
pub struct WorkflowApiUpsert {
    name: String,
    #[serde(default)]
    description: String,
    graph: WorkflowGraph,
    #[serde(default)]
    active: bool,
}

#[derive(Serialize)]
pub struct WorkflowApiRunPreview {
    workflow_id: String,
    run_id: String,
    status: String,
    message: String,
    started_at: String,
    execution_limits: WorkflowExecutionLimits,
}

#[derive(Serialize)]
pub struct WorkflowApiRunItem {
    id: String,
    workflow_id: String,
    trigger_type: String,
    status: String,
    started_at: String,
    finished_at: Option<String>,
    error_message: Option<String>,
    trigger_payload: Value,
}

#[derive(Serialize)]
pub struct WorkflowApiRunStepItem {
    id: String,
    step_index: i64,
    node_id: String,
    node_kind: String,
    note: Option<String>,
    output: Option<Value>,
    occurred_at: String,
}

#[derive(Serialize)]
pub struct WorkflowApiRunDetail {
    run: WorkflowApiRunItem,
    steps: Vec<WorkflowApiRunStepItem>,
}

#[derive(Serialize)]
pub struct WorkflowWebhookTriggerAccepted {
    workflow_id: String,
    run_id: String,
    status: String,
}

/// `GET /api/workflows` -- List workflows.
pub async fn api_list_workflows(
    State(state): State<WorkflowPagesState>,
) -> Result<Json<Vec<WorkflowApiItem>>, (StatusCode, String)> {
    let store = WorkflowStore::new(state.pool);
    let rows = store.list().await.map_err(internal_error)?;
    Ok(Json(rows.iter().map(to_api_item).collect()))
}

/// `POST /api/workflows` -- Create workflow.
pub async fn api_create_workflow(
    State(state): State<WorkflowPagesState>,
    Json(req): Json<WorkflowApiUpsert>,
) -> Result<(StatusCode, Json<WorkflowApiDetail>), (StatusCode, String)> {
    let store = WorkflowStore::new(state.pool.clone());
    let id = store
        .create(
            req.name.trim(),
            req.description.trim(),
            &req.graph,
            req.active,
        )
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

    let saved = store.get(id).await.map_err(internal_error)?.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "created workflow missing".to_string(),
    ))?;
    Ok((StatusCode::CREATED, Json(to_api_detail(saved))))
}

/// `GET /api/workflows/:id` -- Fetch one workflow.
pub async fn api_get_workflow(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowApiDetail>, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let store = WorkflowStore::new(state.pool);
    let workflow = store
        .get(workflow_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;
    Ok(Json(to_api_detail(workflow)))
}

/// `PUT /api/workflows/:id` -- Update workflow.
pub async fn api_update_workflow(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
    Json(req): Json<WorkflowApiUpsert>,
) -> Result<Json<WorkflowApiDetail>, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let store = WorkflowStore::new(state.pool.clone());

    let updated = store
        .update(
            workflow_id,
            req.name.trim(),
            req.description.trim(),
            &req.graph,
            req.active,
        )
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;
    if !updated {
        return Err((StatusCode::NOT_FOUND, "Workflow not found".to_string()));
    }

    let workflow = store
        .get(workflow_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;
    Ok(Json(to_api_detail(workflow)))
}

/// `POST /api/workflows/:id/activate` -- Activate workflow.
pub async fn api_activate_workflow(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowApiDetail>, (StatusCode, String)> {
    set_active_api(state, id, true).await
}

/// `POST /api/workflows/:id/deactivate` -- Deactivate workflow.
pub async fn api_deactivate_workflow(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowApiDetail>, (StatusCode, String)> {
    set_active_api(state, id, false).await
}

/// `POST /api/workflows/:id/test-run` -- Dry-run preview for loop limits.
pub async fn api_test_run_workflow(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowApiRunPreview>, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let store = WorkflowStore::new(state.pool);
    let workflow = store
        .get(workflow_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;

    let run_id = store
        .create_run(
            workflow.id,
            WorkflowTriggerKind::Manual,
            &serde_json::json!({}),
        )
        .await
        .map_err(internal_error)?;

    let now = Utc::now();
    let preview = WorkflowApiRunPreview {
        workflow_id: workflow.id.to_string(),
        run_id: run_id.to_string(),
        status: "accepted".to_string(),
        message: "Workflow run accepted and queued for execution worker".to_string(),
        started_at: now.to_rfc3339(),
        execution_limits: workflow.graph.execution,
    };
    Ok(Json(preview))
}

/// `GET /api/workflows/:id/runs` -- List recent workflow runs.
pub async fn api_list_workflow_runs(
    State(state): State<WorkflowPagesState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<WorkflowApiRunItem>>, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let store = WorkflowStore::new(state.pool.clone());
    let exists = store
        .get(workflow_id)
        .await
        .map_err(internal_error)?
        .is_some();
    if !exists {
        return Err((StatusCode::NOT_FOUND, "Workflow not found".to_string()));
    }

    let runs = store
        .list_runs(workflow_id, 50)
        .await
        .map_err(internal_error)?;
    Ok(Json(runs.into_iter().map(to_api_run_item).collect()))
}

/// `GET /api/workflows/:id/runs/:run_id` -- Workflow run detail with steps.
pub async fn api_get_workflow_run(
    State(state): State<WorkflowPagesState>,
    Path((id, run_id)): Path<(String, String)>,
) -> Result<Json<WorkflowApiRunDetail>, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let run_uuid = parse_uuid(&run_id)?;
    let store = WorkflowStore::new(state.pool.clone());

    let run = store
        .get_run(workflow_id, run_uuid)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow run not found".to_string()))?;
    let steps = store
        .list_run_steps(run_uuid, 500)
        .await
        .map_err(internal_error)?;

    Ok(Json(WorkflowApiRunDetail {
        run: to_api_run_item(run),
        steps: steps.into_iter().map(to_api_run_step_item).collect(),
    }))
}

/// `POST /workflow-hooks/:id/:token` -- Public incoming webhook trigger.
pub async fn public_webhook_trigger(
    State(state): State<WorkflowPagesState>,
    Path((id, token)): Path<(String, String)>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<WorkflowWebhookTriggerAccepted>), (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let store = WorkflowStore::new(state.pool);
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

async fn set_active_api(
    state: WorkflowPagesState,
    id: String,
    active: bool,
) -> Result<Json<WorkflowApiDetail>, (StatusCode, String)> {
    let workflow_id = parse_uuid(&id)?;
    let store = WorkflowStore::new(state.pool.clone());
    let changed = store
        .set_active(workflow_id, active)
        .await
        .map_err(internal_error)?;
    if !changed {
        return Err((StatusCode::NOT_FOUND, "Workflow not found".to_string()));
    }

    let workflow = store
        .get(workflow_id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;
    Ok(Json(to_api_detail(workflow)))
}

fn parse_graph_json(raw: &str) -> Result<WorkflowGraph, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return serde_json::from_str(&default_graph_json()).map_err(|err| err.to_string());
    }
    serde_json::from_str(trimmed).map_err(|err| format!("Invalid graph_json: {err}"))
}

fn parse_uuid(input: &str) -> Result<Uuid, (StatusCode, String)> {
    Uuid::parse_str(input).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid workflow id".to_string()))
}

fn to_row(workflow: &WorkflowRecord) -> WorkflowListRow {
    let (trigger_count, action_count) = count_node_types(&workflow.graph.nodes);
    WorkflowListRow {
        id: workflow.id.to_string(),
        short_id: workflow.id.to_string()[..8].to_string(),
        name: workflow.name.clone(),
        description: workflow.description.clone(),
        active: workflow.active,
        node_count: workflow.graph.nodes.len(),
        edge_count: workflow.graph.edges.len(),
        trigger_count,
        action_count,
        max_steps: workflow.graph.execution.max_steps,
        max_visits_per_node: workflow.graph.execution.max_visits_per_node,
    }
}

fn to_api_item(workflow: &WorkflowRecord) -> WorkflowApiItem {
    let (trigger_count, action_count) = count_node_types(&workflow.graph.nodes);
    WorkflowApiItem {
        id: workflow.id.to_string(),
        name: workflow.name.clone(),
        description: workflow.description.clone(),
        active: workflow.active,
        node_count: workflow.graph.nodes.len(),
        edge_count: workflow.graph.edges.len(),
        trigger_count,
        action_count,
        max_steps: workflow.graph.execution.max_steps,
        max_visits_per_node: workflow.graph.execution.max_visits_per_node,
        created_at: format_ts(workflow.created_at),
        updated_at: format_ts(workflow.updated_at),
    }
}

fn to_api_detail(workflow: WorkflowRecord) -> WorkflowApiDetail {
    WorkflowApiDetail {
        id: workflow.id.to_string(),
        name: workflow.name,
        description: workflow.description,
        active: workflow.active,
        graph: workflow.graph,
        created_at: format_ts(workflow.created_at),
        updated_at: format_ts(workflow.updated_at),
    }
}

fn to_api_run_item(run: WorkflowRunRecord) -> WorkflowApiRunItem {
    WorkflowApiRunItem {
        id: run.id.to_string(),
        workflow_id: run.workflow_id.to_string(),
        trigger_type: run.trigger_type,
        status: run.status,
        started_at: format_ts(run.started_at),
        finished_at: run.finished_at.map(format_ts),
        error_message: run.error_message,
        trigger_payload: run.trigger_payload,
    }
}

fn to_api_run_step_item(step: WorkflowRunStepRecord) -> WorkflowApiRunStepItem {
    WorkflowApiRunStepItem {
        id: step.id.to_string(),
        step_index: step.step_index,
        node_id: step.node_id,
        node_kind: step.node_kind,
        note: step.note,
        output: step.output,
        occurred_at: format_ts(step.occurred_at),
    }
}

fn to_run_view(run: WorkflowRunRecord) -> WorkflowRunView {
    WorkflowRunView {
        id: run.id.to_string()[..8].to_string(),
        run_id: run.id.to_string(),
        trigger_type: run.trigger_type,
        status: run.status,
        started_at: format_ts(run.started_at),
    }
}

fn to_run_step_view(step: WorkflowRunStepRecord) -> WorkflowRunStepView {
    WorkflowRunStepView {
        step_index: step.step_index,
        node_id: step.node_id,
        node_kind: step.node_kind,
        note_text: step.note.unwrap_or_else(|| "-".to_string()),
        output_json: step
            .output
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| "{}".to_string()),
        occurred_at: format_ts(step.occurred_at),
    }
}

fn count_node_types(nodes: &[WorkflowNode]) -> (usize, usize) {
    let trigger_count = nodes
        .iter()
        .filter(|n| n.kind == WorkflowNodeKind::Trigger)
        .count();
    let action_count = nodes
        .iter()
        .filter(|n| n.kind == WorkflowNodeKind::Action)
        .count();
    (trigger_count, action_count)
}

fn format_ts(ts: chrono::DateTime<chrono::Utc>) -> String {
    ts.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

fn default_graph_json() -> String {
    serde_json::to_string_pretty(&WorkflowGraph {
        version: 1,
        nodes: vec![
            WorkflowNode {
                id: "trigger_manual".to_string(),
                kind: WorkflowNodeKind::Trigger,
                config: serde_json::json!({"type": "manual"}),
            },
            WorkflowNode {
                id: "action_assistant".to_string(),
                kind: WorkflowNodeKind::Action,
                config: serde_json::json!({"type": "assistant_turn", "prompt": "Summarize the incoming event"}),
            },
        ],
        edges: vec![assistant_storage::WorkflowEdge {
            from: "trigger_manual".to_string(),
            to: "action_assistant".to_string(),
            on: None,
            condition: None,
        }],
        execution: WorkflowExecutionLimits {
            max_steps: 200,
            max_visits_per_node: 25,
        },
    })
    .unwrap_or_else(|_| "{}".to_string())
}
