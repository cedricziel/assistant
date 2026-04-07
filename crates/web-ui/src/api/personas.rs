//! REST JSON API for persona management.
//!
//! ## Routes
//!
//! | Method | Path                                          | Description                      |
//! |--------|-----------------------------------------------|----------------------------------|
//! | GET    | `/api/personas`                               | List all personas                |
//! | POST   | `/api/personas`                               | Create a persona                 |
//! | POST   | `/api/personas/active`                        | Switch active persona            |
//! | GET    | `/api/personas/{id}`                          | Get persona detail               |
//! | GET    | `/api/personas/{id}/files/{filename}`         | Read a persona file slot         |
//! | PUT    | `/api/personas/{id}/files/{filename}`         | Write a persona file slot        |
//! | GET    | `/api/personas/{id}/skill-access`             | Get skill access config          |
//! | PATCH  | `/api/personas/{id}/skill-access`             | Set skill access mode            |
//! | POST   | `/api/personas/{id}/skill-access/skills`      | Add skill to access list         |
//! | DELETE | `/api/personas/{id}/skill-access/skills/{sk}` | Remove skill from access list    |

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tracing::warn;

use assistant_core::validate_agent_id;

// -- Constants ---------------------------------------------------------------

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

const MAX_FILE_BYTES: usize = 2 * 1024 * 1024; // 2 MB

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct PersonaApiState {
    pub pool: SqlitePool,
    /// Shared live agent ID — updated when the user switches personas.
    pub agent_id: Arc<RwLock<String>>,
}

// -- Helpers -----------------------------------------------------------------

fn persona_agent_dir(id: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".assistant").join("agents").join(id))
}

fn is_valid_filename(filename: &str) -> bool {
    PERSONA_FILE_SLOTS.iter().any(|(f, _, _)| *f == filename)
}

async fn build_file_slots(id: &str) -> Vec<PersonaFileSlot> {
    let mut slots = Vec::with_capacity(PERSONA_FILE_SLOTS.len());
    let base = persona_agent_dir(id);
    for (filename, display_name, description) in PERSONA_FILE_SLOTS {
        let exists = if let Some(ref dir) = base {
            tokio::fs::try_exists(dir.join(filename))
                .await
                .unwrap_or(false)
        } else {
            false
        };
        slots.push(PersonaFileSlot {
            filename: filename.to_string(),
            display_name: display_name.to_string(),
            description: description.to_string(),
            exists,
        });
    }
    slots
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

/// Full persona detail including file slot inventory.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PersonaDetail {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub skill_access_mode: String,
    pub turn_timeout_secs: Option<u64>,
    pub created_at: String,
    pub updated_at: String,
    pub files: Vec<PersonaFileSlot>,
}

/// A single persona file slot.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PersonaFileSlot {
    pub filename: String,
    pub display_name: String,
    pub description: String,
    pub exists: bool,
}

/// Content of a persona file slot.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PersonaFileContent {
    pub filename: String,
    pub content: String,
}

/// Skill access configuration for a persona.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PersonaSkillAccess {
    pub mode: String,
    pub skills: Vec<String>,
}

// -- Request types -----------------------------------------------------------

/// Body for `POST /api/personas/active`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SetActivePersonaRequest {
    pub id: String,
}

/// Body for `POST /api/personas`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreatePersonaRequest {
    pub id: String,
    pub name: String,
}

/// Body for `PUT /api/personas/{id}/files/{filename}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct WritePersonaFileRequest {
    pub content: String,
}

/// Body for `PATCH /api/personas/{id}/skill-access`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SetSkillAccessModeRequest {
    /// One of "all", "whitelist", or "blacklist".
    pub mode: String,
}

/// Body for `POST /api/personas/{id}/skill-access/skills`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AddSkillAccessRequest {
    pub skill_name: String,
}

// -- Router ------------------------------------------------------------------

/// Build the personas API sub-router.  Mounted under `/api`.
pub fn personas_router() -> Router<PersonaApiState> {
    Router::new()
        .route("/personas", get(list_personas).post(create_persona))
        .route("/personas/active", post(set_active_persona))
        .route("/personas/{id}", get(get_persona))
        .route(
            "/personas/{id}/files/{filename}",
            get(get_persona_file).put(put_persona_file),
        )
        .route(
            "/personas/{id}/skill-access",
            get(get_skill_access).patch(patch_skill_access_mode),
        )
        .route("/personas/{id}/skill-access/skills", post(add_skill_access))
        .route(
            "/personas/{id}/skill-access/skills/{skill_name}",
            delete(delete_skill_access),
        )
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

/// `POST /api/personas` — create a new persona.
#[utoipa::path(
    post,
    path = "/api/personas",
    tag = "personas",
    request_body = CreatePersonaRequest,
    responses(
        (status = 201, description = "Persona created", body = PersonaDetail),
        (status = 400, description = "Invalid ID or missing name"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Persona ID already exists"),
    ),
    security(("bearer_token" = []))
)]
pub async fn create_persona(
    State(state): State<PersonaApiState>,
    Json(body): Json<CreatePersonaRequest>,
) -> Response {
    let id = body.id.trim().to_string();
    let name = body.name.trim().to_string();

    if !validate_agent_id(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid persona ID"})),
        )
            .into_response();
    }
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Name is required"})),
        )
            .into_response();
    }

    let store = assistant_storage::personas::PersonaStore::new(state.pool.clone());
    match store.create(&id, &name).await {
        Ok(p) => {
            let files: Vec<PersonaFileSlot> = PERSONA_FILE_SLOTS
                .iter()
                .map(|(filename, display_name, description)| PersonaFileSlot {
                    filename: filename.to_string(),
                    display_name: display_name.to_string(),
                    description: description.to_string(),
                    exists: false,
                })
                .collect();
            let detail = PersonaDetail {
                id: p.id,
                name: p.name,
                is_default: p.is_default,
                skill_access_mode: p.skill_access_mode,
                turn_timeout_secs: p.turn_timeout_secs,
                created_at: p.created_at.to_rfc3339(),
                updated_at: p.updated_at.to_rfc3339(),
                files,
            };
            (StatusCode::CREATED, Json(detail)).into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("UNIQUE") || msg.contains("unique") || msg.contains("already exists") {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({"error": "Persona ID already exists"})),
                )
                    .into_response();
            }
            warn!("Failed to create persona {id}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
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

/// `GET /api/personas/{id}` — get full persona detail.
#[utoipa::path(
    get,
    path = "/api/personas/{id}",
    tag = "personas",
    params(("id" = String, Path, description = "Persona ID")),
    responses(
        (status = 200, description = "Persona detail", body = PersonaDetail),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Persona not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_persona(State(state): State<PersonaApiState>, Path(id): Path<String>) -> Response {
    if !validate_agent_id(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid persona ID"})),
        )
            .into_response();
    }

    let store = assistant_storage::personas::PersonaStore::new(state.pool.clone());
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

    let files = build_file_slots(&id).await;
    let detail = PersonaDetail {
        id: persona.id,
        name: persona.name,
        is_default: persona.is_default,
        skill_access_mode: persona.skill_access_mode,
        turn_timeout_secs: persona.turn_timeout_secs,
        created_at: persona.created_at.to_rfc3339(),
        updated_at: persona.updated_at.to_rfc3339(),
        files,
    };
    Json(detail).into_response()
}

/// `GET /api/personas/{id}/files/{filename}` — read a persona file slot.
#[utoipa::path(
    get,
    path = "/api/personas/{id}/files/{filename}",
    tag = "personas",
    params(
        ("id" = String, Path, description = "Persona ID"),
        ("filename" = String, Path, description = "File slot name (e.g. SOUL.md)"),
    ),
    responses(
        (status = 200, description = "File content (empty string if not yet written)", body = PersonaFileContent),
        (status = 400, description = "Invalid ID or filename"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Persona not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_persona_file(
    State(state): State<PersonaApiState>,
    Path((id, filename)): Path<(String, String)>,
) -> Response {
    if !validate_agent_id(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid persona ID"})),
        )
            .into_response();
    }
    if !is_valid_filename(&filename) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid filename"})),
        )
            .into_response();
    }

    let store = assistant_storage::personas::PersonaStore::new(state.pool.clone());
    match store.get(&id).await {
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
        _ => {}
    }

    let content = if let Some(dir) = persona_agent_dir(&id) {
        let path = dir.join(&filename);
        tokio::fs::read_to_string(&path).await.unwrap_or_default()
    } else {
        String::new()
    };

    Json(PersonaFileContent { filename, content }).into_response()
}

/// `PUT /api/personas/{id}/files/{filename}` — write a persona file slot.
#[utoipa::path(
    put,
    path = "/api/personas/{id}/files/{filename}",
    tag = "personas",
    params(
        ("id" = String, Path, description = "Persona ID"),
        ("filename" = String, Path, description = "File slot name (e.g. SOUL.md)"),
    ),
    request_body = WritePersonaFileRequest,
    responses(
        (status = 200, description = "File written", body = PersonaFileContent),
        (status = 400, description = "Invalid ID or filename"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Persona not found"),
        (status = 413, description = "Content too large (max 2 MB)"),
    ),
    security(("bearer_token" = []))
)]
pub async fn put_persona_file(
    State(state): State<PersonaApiState>,
    Path((id, filename)): Path<(String, String)>,
    Json(body): Json<WritePersonaFileRequest>,
) -> Response {
    if !validate_agent_id(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid persona ID"})),
        )
            .into_response();
    }
    if !is_valid_filename(&filename) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid filename"})),
        )
            .into_response();
    }
    if body.content.len() > MAX_FILE_BYTES {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "Content too large (max 2 MB)"})),
        )
            .into_response();
    }

    let store = assistant_storage::personas::PersonaStore::new(state.pool.clone());
    match store.get(&id).await {
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
        _ => {}
    }

    let Some(dir) = persona_agent_dir(&id) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not determine home directory",
        )
            .into_response();
    };

    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        warn!("Failed to create persona dir {}: {e}", dir.display());
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create directory",
        )
            .into_response();
    }

    let path = dir.join(&filename);
    if let Err(e) = tokio::fs::write(&path, body.content.as_bytes()).await {
        warn!("Failed to write persona file {}: {e}", path.display());
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to write file").into_response();
    }

    Json(PersonaFileContent {
        filename,
        content: body.content,
    })
    .into_response()
}

/// `GET /api/personas/{id}/skill-access` — get skill access config.
#[utoipa::path(
    get,
    path = "/api/personas/{id}/skill-access",
    tag = "personas",
    params(("id" = String, Path, description = "Persona ID")),
    responses(
        (status = 200, description = "Skill access configuration", body = PersonaSkillAccess),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Persona not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_skill_access(
    State(state): State<PersonaApiState>,
    Path(id): Path<String>,
) -> Response {
    if !validate_agent_id(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid persona ID"})),
        )
            .into_response();
    }

    let persona_store = assistant_storage::personas::PersonaStore::new(state.pool.clone());
    match persona_store.get(&id).await {
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
        _ => {}
    }

    let access_store =
        assistant_storage::persona_skill_access::PersonaSkillAccessStore::new(state.pool.clone());
    let mode = access_store
        .get_mode(&id)
        .await
        .unwrap_or_else(|_| "all".to_string());
    let skills = access_store.list_skill_names(&id).await.unwrap_or_default();

    Json(PersonaSkillAccess { mode, skills }).into_response()
}

/// `PATCH /api/personas/{id}/skill-access` — set skill access mode.
#[utoipa::path(
    patch,
    path = "/api/personas/{id}/skill-access",
    tag = "personas",
    params(("id" = String, Path, description = "Persona ID")),
    request_body = SetSkillAccessModeRequest,
    responses(
        (status = 200, description = "Updated skill access configuration", body = PersonaSkillAccess),
        (status = 400, description = "Invalid ID or mode"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Persona not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn patch_skill_access_mode(
    State(state): State<PersonaApiState>,
    Path(id): Path<String>,
    Json(body): Json<SetSkillAccessModeRequest>,
) -> Response {
    if !validate_agent_id(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid persona ID"})),
        )
            .into_response();
    }

    let mode = body.mode.trim().to_string();
    if !matches!(mode.as_str(), "all" | "whitelist" | "blacklist") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Mode must be one of: all, whitelist, blacklist"})),
        )
            .into_response();
    }

    let persona_store = assistant_storage::personas::PersonaStore::new(state.pool.clone());
    match persona_store.get(&id).await {
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
        _ => {}
    }

    let access_store =
        assistant_storage::persona_skill_access::PersonaSkillAccessStore::new(state.pool.clone());

    if let Err(e) = access_store.set_mode(&id, &mode).await {
        warn!("Failed to set skill access mode for {id}: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
    }

    let skills = access_store.list_skill_names(&id).await.unwrap_or_default();

    Json(PersonaSkillAccess { mode, skills }).into_response()
}

/// `POST /api/personas/{id}/skill-access/skills` — add a skill to the access list.
#[utoipa::path(
    post,
    path = "/api/personas/{id}/skill-access/skills",
    tag = "personas",
    params(("id" = String, Path, description = "Persona ID")),
    request_body = AddSkillAccessRequest,
    responses(
        (status = 200, description = "Updated skill access configuration", body = PersonaSkillAccess),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Persona not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn add_skill_access(
    State(state): State<PersonaApiState>,
    Path(id): Path<String>,
    Json(body): Json<AddSkillAccessRequest>,
) -> Response {
    if !validate_agent_id(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid persona ID"})),
        )
            .into_response();
    }

    let persona_store = assistant_storage::personas::PersonaStore::new(state.pool.clone());
    match persona_store.get(&id).await {
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
        _ => {}
    }

    let access_store =
        assistant_storage::persona_skill_access::PersonaSkillAccessStore::new(state.pool.clone());

    if let Err(e) = access_store.add_skill(&id, &body.skill_name).await {
        warn!("Failed to add skill {} for {id}: {e}", body.skill_name);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
    }

    let mode = access_store
        .get_mode(&id)
        .await
        .unwrap_or_else(|_| "all".to_string());
    let skills = access_store.list_skill_names(&id).await.unwrap_or_default();

    Json(PersonaSkillAccess { mode, skills }).into_response()
}

/// `DELETE /api/personas/{id}/skill-access/skills/{skill_name}` — remove a skill from the access list.
#[utoipa::path(
    delete,
    path = "/api/personas/{id}/skill-access/skills/{skill_name}",
    tag = "personas",
    params(
        ("id" = String, Path, description = "Persona ID"),
        ("skill_name" = String, Path, description = "Skill name to remove"),
    ),
    responses(
        (status = 204, description = "Skill removed"),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Persona not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn delete_skill_access(
    State(state): State<PersonaApiState>,
    Path((id, skill_name)): Path<(String, String)>,
) -> Response {
    if !validate_agent_id(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid persona ID"})),
        )
            .into_response();
    }

    let persona_store = assistant_storage::personas::PersonaStore::new(state.pool.clone());
    match persona_store.get(&id).await {
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
        _ => {}
    }

    let access_store =
        assistant_storage::persona_skill_access::PersonaSkillAccessStore::new(state.pool.clone());

    if let Err(e) = access_store.remove_skill(&id, &skill_name).await {
        warn!("Failed to remove skill {skill_name} for {id}: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
    }

    StatusCode::NO_CONTENT.into_response()
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

    // -- existing tests --

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

    // -- new tests --

    #[tokio::test]
    async fn get_persona_returns_default_detail() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/default")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["id"], "default");
        assert!(json["files"].is_array(), "files should be an array");
        assert_eq!(
            json["files"].as_array().unwrap().len(),
            8,
            "should have 8 file slots"
        );
    }

    #[tokio::test]
    async fn get_persona_invalid_id_returns_400() {
        let (state, _) = test_state().await;

        // "invalid.id" contains a '.' which fails validate_agent_id but is URI-safe.
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/invalid.id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_persona_unknown_returns_404() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_persona_returns_201() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/personas")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"id":"new-persona","name":"New Persona"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["id"], "new-persona");
        assert_eq!(json["name"], "New Persona");
        assert_eq!(
            json["files"].as_array().unwrap().len(),
            8,
            "should have 8 file slots"
        );
    }

    #[tokio::test]
    async fn create_persona_invalid_id_returns_400() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/personas")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"id":"bad id!","name":"Test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_persona_duplicate_returns_409() {
        let (state, _) = test_state().await;

        let body = r#"{"id":"default","name":"Duplicate"}"#;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/personas")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn get_persona_file_nonexistent_returns_empty_content() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/default/files/SOUL.md")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["filename"], "SOUL.md");
        // content may be empty string or actual content depending on filesystem
        assert!(json["content"].is_string());
    }

    #[tokio::test]
    async fn get_persona_file_invalid_filename_returns_400() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/default/files/INVALID.md")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_persona_file_unknown_persona_returns_404() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/nonexistent/files/SOUL.md")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn put_persona_file_oversized_returns_413() {
        let (state, _) = test_state().await;

        let big_content = "x".repeat(3 * 1024 * 1024);
        let body = serde_json::json!({"content": big_content}).to_string();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/personas/default/files/SOUL.md")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn put_and_get_persona_file_roundtrip() {
        let (state, storage) = test_state().await;

        // Create a dedicated test persona to avoid polluting "default".
        storage
            .persona_store()
            .create("filetest", "File Test")
            .await
            .unwrap();

        let state2 = state.clone();

        let write_body = r#"{"content":"Hello, world!"}"#;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/personas/filetest/files/SOUL.md")
                    .header("Content-Type", "application/json")
                    .body(Body::from(write_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let resp2 = app(state2)
            .oneshot(
                Request::builder()
                    .uri("/personas/filetest/files/SOUL.md")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp2.status(), StatusCode::OK);
        let json = body_json(resp2.into_body()).await;
        assert_eq!(json["content"], "Hello, world!");

        // Cleanup.
        let dir = dirs::home_dir()
            .unwrap()
            .join(".assistant")
            .join("agents")
            .join("filetest");
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn get_skill_access_returns_defaults() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/default/skill-access")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert!(json["mode"].is_string());
        assert!(json["skills"].is_array());
    }

    #[tokio::test]
    async fn get_skill_access_unknown_persona_returns_404() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/personas/nonexistent/skill-access")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn patch_skill_access_mode_updates_mode() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/personas/default/skill-access")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"mode":"whitelist"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["mode"], "whitelist");
    }

    #[tokio::test]
    async fn patch_skill_access_mode_invalid_mode_returns_400() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/personas/default/skill-access")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"mode":"invalid"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn add_and_delete_skill_access() {
        let (state, _) = test_state().await;
        let state2 = state.clone();
        let state3 = state.clone();

        // Add a skill.
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/personas/default/skill-access/skills")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"skill_name":"my-skill"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert!(
            json["skills"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("my-skill")),
            "skill should be in the list"
        );

        // Delete the skill.
        let resp2 = app(state2)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/personas/default/skill-access/skills/my-skill")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp2.status(), StatusCode::NO_CONTENT);

        // Verify removed.
        let resp3 = app(state3)
            .oneshot(
                Request::builder()
                    .uri("/personas/default/skill-access")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json3 = body_json(resp3.into_body()).await;
        assert!(
            !json3["skills"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("my-skill")),
            "skill should be removed"
        );
    }

    #[tokio::test]
    async fn delete_skill_access_unknown_persona_returns_404() {
        let (state, _) = test_state().await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/personas/nonexistent/skill-access/skills/foo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
