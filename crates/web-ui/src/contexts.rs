//! Persona context management pages.

use std::path::PathBuf;

use askama::Template;
use assistant_core::{default_workspace_dir, validate_agent_id};
use assistant_storage::PersonaStore;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::common::{active_agent_id, internal_error, render_template, url_encode, StaticUrls};
use crate::AppState;

/// The 8 canonical persona file slots.
///
/// Each entry is `(filename, display_name, description)`.
const PERSONA_FILE_SLOTS: [(&str, &str, &str); 8] = [
    ("SOUL.md", "Soul", "Personality, values, and core truths"),
    (
        "IDENTITY.md",
        "Identity",
        "Name, role, and structured identity profile",
    ),
    ("USER.md", "User", "User profile, preferences, and timezone"),
    ("MEMORY.md", "Memory", "Curated long-term memory"),
    (
        "AGENTS.md",
        "Agents",
        "Workspace rules and session startup ritual",
    ),
    ("TOOLS.md", "Tools", "Environment-specific tool notes"),
    ("BOOTSTRAP.md", "Bootstrap", "First-run onboarding ritual"),
    (
        "HEARTBEAT.md",
        "Heartbeat",
        "Periodic task checklist for the scheduler",
    ),
];

// -- Helper functions --------------------------------------------------------

/// Returns `Some(canonical_filename)` if `s` matches one of the 8 allowed
/// filenames, `None` otherwise.
fn persona_filename(s: &str) -> Option<&'static str> {
    PERSONA_FILE_SLOTS
        .iter()
        .find(|(filename, _, _)| *filename == s)
        .map(|(filename, _, _)| *filename)
}

/// Returns `Some(~/.assistant/agents/{id}/)` — does NOT create the directory.
fn persona_agent_dir(id: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".assistant").join("agents").join(id))
}

// -- Templates ---------------------------------------------------------------

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

// -- Persona detail ----------------------------------------------------------

struct PersonaFileSlotView {
    filename: &'static str,
    display_name: &'static str,
    description: &'static str,
    exists: bool,
}

#[derive(Template)]
#[template(path = "personas/detail.html")]
struct PersonaDetailTemplate {
    active_page: &'static str,
    persona_id: String,
    persona_name: String,
    slots: Vec<PersonaFileSlotView>,
}

impl StaticUrls for PersonaDetailTemplate {}

// -- File editor -------------------------------------------------------------

#[derive(Deserialize)]
struct FileEditorQuery {
    saved: Option<u8>,
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "personas/file_editor.html")]
struct PersonaFileEditorTemplate {
    active_page: &'static str,
    persona_id: String,
    persona_name: String,
    filename: &'static str,
    display_name: &'static str,
    content: String,
    is_new: bool,
    show_saved: bool,
    error_msg: Option<String>,
}

impl StaticUrls for PersonaFileEditorTemplate {}

#[derive(Deserialize)]
struct SaveFileForm {
    content: String,
}

// -- New persona form --------------------------------------------------------

#[derive(Deserialize)]
struct NewPersonaQuery {
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "personas/new.html")]
struct PersonaNewFormTemplate {
    active_page: &'static str,
    error_msg: Option<String>,
}

impl StaticUrls for PersonaNewFormTemplate {}

#[derive(Deserialize)]
struct CreatePersonaForm {
    id: String,
    name: String,
}

// -- Router ------------------------------------------------------------------

pub(crate) fn contexts_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/personas", axum::routing::get(show_contexts))
        .route("/personas/new", axum::routing::get(show_new_persona_form)) // MUST be before /{id}
        .route("/personas", axum::routing::post(create_persona))
        .route("/personas/{id}", axum::routing::get(show_persona_detail))
        .route("/personas/{id}/use", axum::routing::post(use_context))
        .route(
            "/personas/{id}/files/{filename}",
            axum::routing::get(show_file_editor).post(save_file),
        )
}

// -- Handlers ----------------------------------------------------------------

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

async fn show_persona_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    if !validate_agent_id(&id) {
        return Err((StatusCode::BAD_REQUEST, "Invalid agent ID".to_string()));
    }

    let store = PersonaStore::new(state.pool.clone());
    let persona = store
        .get(&id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Persona '{}' not found", id)))?;

    let mut slots = Vec::with_capacity(PERSONA_FILE_SLOTS.len());
    for (filename, display_name, description) in &PERSONA_FILE_SLOTS {
        let exists = if let Some(dir) = persona_agent_dir(&id) {
            tokio::fs::try_exists(dir.join(filename))
                .await
                .unwrap_or(false)
        } else {
            false
        };
        slots.push(PersonaFileSlotView {
            filename,
            display_name,
            description,
            exists,
        });
    }

    let tmpl = PersonaDetailTemplate {
        active_page: "personas",
        persona_id: persona.id,
        persona_name: persona.name,
        slots,
    };

    Ok(render_template(tmpl))
}

async fn show_file_editor(
    State(state): State<AppState>,
    Path((id, filename)): Path<(String, String)>,
    Query(query): Query<FileEditorQuery>,
) -> Result<Response, (StatusCode, String)> {
    if !validate_agent_id(&id) {
        return Err((StatusCode::BAD_REQUEST, "Invalid agent ID".to_string()));
    }

    let canonical = persona_filename(&filename).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("Unknown file: {}", filename),
        )
    })?;

    let store = PersonaStore::new(state.pool.clone());
    let persona = store
        .get(&id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Persona '{}' not found", id)))?;

    // is_new: file did not exist before opening — used to show "Create File" vs "Save Changes" in the editor
    let (content, is_new) = match persona_agent_dir(&id) {
        Some(dir) => match tokio::fs::read_to_string(dir.join(canonical)).await {
            Ok(c) => (c, false),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (String::new(), true),
            Err(e) => return Err(internal_error(e)),
        },
        None => (String::new(), true),
    };

    let display_name = PERSONA_FILE_SLOTS
        .iter()
        .find(|(f, _, _)| *f == canonical)
        .map(|(_, d, _)| *d)
        .unwrap_or(canonical);

    let tmpl = PersonaFileEditorTemplate {
        active_page: "personas",
        persona_id: persona.id,
        persona_name: persona.name,
        filename: canonical,
        display_name,
        content,
        is_new,
        show_saved: query.saved.unwrap_or(0) != 0,
        error_msg: query.error,
    };

    Ok(render_template(tmpl))
}

async fn save_file(
    State(state): State<AppState>,
    Path((id, filename)): Path<(String, String)>,
    axum::Form(form): axum::Form<SaveFileForm>,
) -> Response {
    if !validate_agent_id(&id) {
        return (StatusCode::BAD_REQUEST, "Invalid agent ID".to_string()).into_response();
    }

    let canonical = match persona_filename(&filename) {
        Some(f) => f,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Unknown file: {}", filename),
            )
                .into_response()
        }
    };

    let store = PersonaStore::new(state.pool.clone());
    if store.get(&id).await.unwrap_or(None).is_none() {
        return (StatusCode::NOT_FOUND, format!("Persona '{}' not found", id)).into_response();
    }

    if form.content.len() > 2 * 1024 * 1024 {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            "File content too large".to_string(),
        )
            .into_response();
    }

    let dir = match persona_agent_dir(&id) {
        Some(d) => d,
        None => {
            return Redirect::to(&format!(
                "/personas/{}/files/{}?error={}",
                id,
                canonical,
                url_encode("Cannot determine home directory")
            ))
            .into_response()
        }
    };

    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        return Redirect::to(&format!(
            "/personas/{}/files/{}?error={}",
            id,
            canonical,
            url_encode(&e.to_string())
        ))
        .into_response();
    }

    match tokio::fs::write(dir.join(canonical), form.content.as_bytes()).await {
        Ok(()) => {
            Redirect::to(&format!("/personas/{}/files/{}?saved=1", id, canonical)).into_response()
        }
        Err(e) => Redirect::to(&format!(
            "/personas/{}/files/{}?error={}",
            id,
            canonical,
            url_encode(&e.to_string())
        ))
        .into_response(),
    }
}

async fn show_new_persona_form(
    Query(query): Query<NewPersonaQuery>,
) -> Result<Response, (StatusCode, String)> {
    let tmpl = PersonaNewFormTemplate {
        active_page: "personas",
        error_msg: query.error,
    };
    Ok(render_template(tmpl))
}

async fn create_persona(
    State(state): State<AppState>,
    axum::Form(form): axum::Form<CreatePersonaForm>,
) -> Response {
    let id = form.id.trim().to_string();
    let name = form.name.trim().to_string();

    if id.is_empty() {
        return Redirect::to(&format!(
            "/personas/new?error={}",
            url_encode("ID is required")
        ))
        .into_response();
    }
    if !validate_agent_id(&id) {
        return Redirect::to(&format!(
            "/personas/new?error={}",
            url_encode("Invalid ID: use letters, numbers, - and _ only")
        ))
        .into_response();
    }
    if name.is_empty() {
        return Redirect::to(&format!(
            "/personas/new?error={}",
            url_encode("Name is required")
        ))
        .into_response();
    }

    let store = PersonaStore::new(state.pool.clone());
    match store.create(&id, &name).await {
        Ok(_) => Redirect::to(&format!("/personas/{}", id)).into_response(),
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") {
                format!("A persona with ID '{}' already exists", id)
            } else {
                e.to_string()
            };
            Redirect::to(&format!("/personas/new?error={}", url_encode(&msg))).into_response()
        }
    }
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

    async fn test_state(pool: sqlx::SqlitePool) -> AppState {
        use assistant_storage::registry::SkillRegistry;
        let registry = SkillRegistry::new(pool.clone())
            .await
            .expect("test registry");
        AppState {
            pool,
            agent_id: Arc::new(RwLock::new("default".to_string())),
            registry: Arc::new(registry),
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
        let state = test_state(storage.pool.clone()).await;

        let agents = PersonaStore::new(storage.pool.clone());
        agents.ensure_default().await.unwrap();
        agents.ensure_exists("marketing").await.unwrap();

        let response = use_context(
            State(state.clone()),
            Path::<String>("marketing".to_string()),
        )
        .await;

        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(location, "/personas?updated=1");
        assert_eq!(*state.agent_id.read().await, "marketing");
        assert_eq!(agents.default_id().await.unwrap(), "default");
    }

    #[tokio::test]
    async fn use_context_rejects_invalid_agent_id() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let state = test_state(storage.pool.clone()).await;

        let response = use_context(
            State(state.clone()),
            Path::<String>("../bad-id".to_string()),
        )
        .await;

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(*state.agent_id.read().await, "default");
    }
}
