//! Assistant context (agent) management pages.

use askama::Template;
use assistant_core::{default_workspace_dir, validate_agent_id};
use assistant_storage::AssistantAgentStore;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::common::{internal_error, render_template, StaticUrls};
use crate::AppState;

#[derive(Debug)]
struct AgentRowView {
    id: String,
    name: String,
    is_default: bool,
    is_current: bool,
}

#[derive(Debug, Deserialize)]
struct ContextQuery {
    updated: Option<u8>,
}

#[derive(Template)]
#[template(path = "contexts/page.html")]
struct ContextsPageTemplate {
    active_page: &'static str,
    rows: Vec<AgentRowView>,
    current_agent: String,
    default_agent: String,
    show_updated: bool,
}

impl StaticUrls for ContextsPageTemplate {}

pub(crate) fn contexts_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/contexts", axum::routing::get(show_contexts))
        .route("/contexts/{id}/use", axum::routing::post(use_context))
}

async fn show_contexts(
    State(state): State<AppState>,
    Query(query): Query<ContextQuery>,
) -> Result<Response, (StatusCode, String)> {
    let current_agent = state.agent_id.read().await.clone();
    let store = AssistantAgentStore::new(state.pool.clone());
    store.ensure_default().await.map_err(internal_error)?;
    let rows_raw = store.list().await.map_err(internal_error)?;
    let default_agent = store.default_id().await.map_err(internal_error)?;

    let rows = rows_raw
        .into_iter()
        .map(|row| AgentRowView {
            is_current: row.id == current_agent,
            is_default: row.is_default,
            id: row.id,
            name: row.name,
        })
        .collect();

    let tmpl = ContextsPageTemplate {
        active_page: "contexts",
        rows,
        current_agent,
        default_agent,
        show_updated: query.updated.unwrap_or(0) != 0,
    };

    Ok(render_template(tmpl))
}

async fn use_context(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    if !validate_agent_id(&id) {
        return (StatusCode::BAD_REQUEST, "Invalid agent ID".to_string()).into_response();
    }

    let store = AssistantAgentStore::new(state.pool.clone());
    if let Err(e) = store.ensure_default().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if let Err(e) = store.ensure_exists(&id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if let Some(home) = dirs::home_dir() {
        let agent_root = home.join(".assistant").join("agents").join(&id);
        if let Err(e) = tokio::fs::create_dir_all(&agent_root).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }
    let workspace_dir = default_workspace_dir(&id);
    if let Err(e) = tokio::fs::create_dir_all(&workspace_dir).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    *state.agent_id.write().await = id;

    Redirect::to("/contexts?updated=1").into_response()
}
