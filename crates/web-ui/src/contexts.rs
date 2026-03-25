//! Persona context management pages.

use askama::Template;
use assistant_core::{default_workspace_dir, validate_agent_id};
use assistant_storage::PersonaStore;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::common::{active_agent_id, internal_error, render_template, StaticUrls};
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
#[template(path = "personas/page.html")]
struct PersonasPageTemplate {
    active_page: &'static str,
    rows: Vec<AgentRowView>,
    current_agent: String,
    default_agent: String,
    show_updated: bool,
}

impl StaticUrls for PersonasPageTemplate {}

pub(crate) fn contexts_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/personas", axum::routing::get(show_contexts))
        .route("/personas/{id}/use", axum::routing::post(use_context))
}

async fn show_contexts(
    State(state): State<AppState>,
    Query(query): Query<ContextQuery>,
) -> Result<Response, (StatusCode, String)> {
    let current_agent = active_agent_id(&state.agent_id).await;
    let store = PersonaStore::new(state.pool.clone());
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

    let tmpl = PersonasPageTemplate {
        active_page: "personas",
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

    let store = PersonaStore::new(state.pool.clone());
    if let Err(e) = store.ensure_default().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if let Err(e) = store.ensure_exists(&id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    if let Err(e) = ensure_agent_dirs(&id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    *state.agent_id.write().await = id;

    Redirect::to("/personas?updated=1").into_response()
}

async fn ensure_agent_dirs(id: &str) -> std::io::Result<()> {
    if let Some(home) = dirs::home_dir() {
        let agent_root = home.join(".assistant").join("agents").join(id);
        tokio::fs::create_dir_all(agent_root).await?;
    }
    let workspace_dir = default_workspace_dir(id);
    tokio::fs::create_dir_all(workspace_dir).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use assistant_core::BusKind;
    use assistant_storage::{PersonaStore, StorageLayer};
    use axum::extract::{Path, State};
    use tokio::sync::RwLock;

    use super::use_context;
    use crate::AppState;

    fn test_state(pool: sqlx::SqlitePool) -> AppState {
        AppState {
            pool,
            agent_id: Arc::new(RwLock::new("default".to_string())),
            trace_limit: 10,
            log_limit: 10,
            bus_kind: BusKind::Sqlite,
            nats_url: None,
            nats_token: None,
        }
    }

    #[tokio::test]
    async fn use_context_updates_runtime_agent_without_changing_default() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let state = test_state(storage.pool.clone());

        let agents = PersonaStore::new(storage.pool.clone());
        agents.ensure_default().await.unwrap();
        agents.ensure_exists("marketing").await.unwrap();

        let response = use_context(
            State(state.clone()),
            Path::<String>("marketing".to_string()),
        )
        .await;

        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        let location = response.headers().get("location").unwrap().to_str().unwrap();
        assert_eq!(location, "/personas?updated=1");
        assert_eq!(*state.agent_id.read().await, "marketing");
        assert_eq!(agents.default_id().await.unwrap(), "default");
    }

    #[tokio::test]
    async fn use_context_rejects_invalid_agent_id() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let state = test_state(storage.pool.clone());

        let response = use_context(
            State(state.clone()),
            Path::<String>("../bad-id".to_string()),
        )
        .await;

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(*state.agent_id.read().await, "default");
    }
}
