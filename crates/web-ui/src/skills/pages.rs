//! Server-side rendered HTML pages for skill management.
//!
//! All HTML is rendered via Askama templates under `templates/skills/`.

use std::sync::Arc;

use askama::Template;
use assistant_core::Interface;
use assistant_runtime::Orchestrator;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Redirect, Response};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use assistant_skills::{parse_skill_content, SkillSource};
use assistant_storage::registry::SkillRegistry;

use crate::common::{internal_error, render_template, StaticUrls};

// -- Shared state -----------------------------------------------------------

/// State required by the skill management pages.
#[derive(Clone)]
pub struct SkillsPagesState {
    pub registry: Arc<SkillRegistry>,
    pub orchestrator: Arc<Orchestrator>,
}

// -- View models ------------------------------------------------------------

/// A row in the skill list table.
struct SkillRowView {
    name: String,
    description: String,
    source_label: String,
    is_builtin: bool,
    is_project: bool,
}

/// Full skill detail for show/edit pages.
struct SkillDetailView {
    name: String,
    description: String,
    body: String,
    source_label: String,
    is_builtin: bool,
    is_project: bool,
    license: String,
}

// -- Templates --------------------------------------------------------------

#[derive(Template)]
#[template(path = "skills/list.html")]
struct SkillListTemplate {
    active_page: &'static str,
    count: usize,
    rows: Vec<SkillRowView>,
}

impl StaticUrls for SkillListTemplate {}

#[derive(Template)]
#[template(path = "skills/show.html")]
struct SkillShowTemplate {
    active_page: &'static str,
    skill: SkillDetailView,
}

impl StaticUrls for SkillShowTemplate {}

#[derive(Template)]
#[template(path = "skills/new.html")]
struct SkillNewTemplate {
    active_page: &'static str,
    error: Option<String>,
    name: String,
    description: String,
    body: String,
}

impl StaticUrls for SkillNewTemplate {}

#[derive(Template)]
#[template(path = "skills/edit.html")]
struct SkillEditTemplate {
    active_page: &'static str,
    error: Option<String>,
    name: String,
    description: String,
    body: String,
}

impl StaticUrls for SkillEditTemplate {}

// -- Helpers ----------------------------------------------------------------

fn source_label(source: &SkillSource) -> String {
    match source {
        SkillSource::Builtin => "builtin".to_string(),
        SkillSource::User => "user".to_string(),
        SkillSource::Project => "project".to_string(),
        SkillSource::Installed => "installed".to_string(),
    }
}

// -- Handlers ---------------------------------------------------------------

pub async fn list_skills(State(state): State<SkillsPagesState>) -> Response {
    let skills = state.registry.list().await;
    let count = skills.len();
    let rows = skills
        .into_iter()
        .map(|s| SkillRowView {
            is_builtin: matches!(s.source, SkillSource::Builtin),
            is_project: matches!(s.source, SkillSource::Project),
            source_label: source_label(&s.source),
            name: s.name,
            description: s.description,
        })
        .collect();

    render_template(SkillListTemplate {
        active_page: "skills",
        count,
        rows,
    })
}

pub async fn show_skill(
    State(state): State<SkillsPagesState>,
    Path(name): Path<String>,
) -> Response {
    let skill = match state.registry.get(&name).await {
        Some(s) => s,
        None => {
            return (StatusCode::NOT_FOUND, format!("Skill '{}' not found", name)).into_response()
        }
    };

    render_template(SkillShowTemplate {
        active_page: "skills",
        skill: SkillDetailView {
            is_builtin: matches!(skill.source, SkillSource::Builtin),
            is_project: matches!(skill.source, SkillSource::Project),
            source_label: source_label(&skill.source),
            license: skill.license.unwrap_or_default(),
            name: skill.name,
            description: skill.description,
            body: skill.body,
        },
    })
}

pub async fn new_skill_form(State(_state): State<SkillsPagesState>) -> Response {
    render_template(SkillNewTemplate {
        active_page: "skills",
        error: None,
        name: String::new(),
        description: String::new(),
        body: String::new(),
    })
}

#[derive(Deserialize)]
pub struct SkillForm {
    pub name: String,
    pub description: String,
    pub body: String,
}

pub async fn create_skill(
    State(state): State<SkillsPagesState>,
    Form(form): Form<SkillForm>,
) -> Response {
    match state
        .registry
        .create_user_skill(&form.name, &form.description, &form.body)
        .await
    {
        Ok(def) => Redirect::to(&format!("/skills/{}", def.name)).into_response(),
        Err(e) => render_template(SkillNewTemplate {
            active_page: "skills",
            error: Some(e.to_string()),
            name: form.name,
            description: form.description,
            body: form.body.trim_start().to_string(),
        }),
    }
}

pub async fn edit_skill_form(
    State(state): State<SkillsPagesState>,
    Path(name): Path<String>,
) -> Response {
    let skill = match state.registry.get(&name).await {
        Some(s) => s,
        None => {
            return (StatusCode::NOT_FOUND, format!("Skill '{}' not found", name)).into_response()
        }
    };

    if matches!(skill.source, SkillSource::Builtin | SkillSource::Project) {
        return (
            StatusCode::FORBIDDEN,
            format!("Skill '{}' cannot be edited", name),
        )
            .into_response();
    }

    render_template(SkillEditTemplate {
        active_page: "skills",
        error: None,
        name: skill.name,
        description: skill.description,
        // Trim leading whitespace so the HTML formatter's newline before
        // `{{ body }}` inside the <textarea> doesn't corrupt the saved value.
        body: skill.body.trim_start().to_string(),
    })
}

#[derive(Deserialize)]
pub struct SkillUpdateForm {
    pub description: String,
    pub body: String,
}

pub async fn update_skill(
    State(state): State<SkillsPagesState>,
    Path(name): Path<String>,
    Form(form): Form<SkillUpdateForm>,
) -> Response {
    match state
        .registry
        .update_user_skill(&name, &form.description, &form.body)
        .await
    {
        Ok(()) => Redirect::to(&format!("/skills/{}", name)).into_response(),
        Err(e) => {
            if e.to_string().contains("Cannot edit builtin")
                || e.to_string().contains("Cannot edit project")
            {
                return (StatusCode::FORBIDDEN, e.to_string()).into_response();
            }
            render_template(SkillEditTemplate {
                active_page: "skills",
                error: Some(e.to_string()),
                name,
                description: form.description,
                body: form.body,
            })
        }
    }
}

pub async fn delete_skill(
    State(state): State<SkillsPagesState>,
    Path(name): Path<String>,
) -> Response {
    match state.registry.delete_user_skill(&name).await {
        Ok(()) => Redirect::to("/skills").into_response(),
        Err(e) => {
            if e.to_string().contains("Cannot delete builtin") {
                return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
            }
            warn!("Failed to delete skill '{}': {}", name, e);
            internal_error(e).into_response()
        }
    }
}

// -- AI generation ----------------------------------------------------------

#[derive(Deserialize)]
pub struct GenerateRequest {
    pub description: String,
}

#[derive(Serialize)]
struct GenerateOk {
    /// Parsed skill name from generated frontmatter (empty string if unparseable).
    name: String,
    /// Parsed description from generated frontmatter.
    description: String,
    /// Body-only content (everything after the frontmatter `---`).
    body: String,
}

#[derive(Serialize)]
struct GenerateErr {
    error: String,
}

/// `POST /skills/generate` — ask the Orchestrator to produce a SKILL.md draft.
///
/// Returns `{ "body": "<content>" }` on success or `{ "error": "..." }` on
/// failure.  Times out at 30 s and returns 504 if the Orchestrator does not
/// respond in time.
pub async fn generate_skill(
    State(state): State<SkillsPagesState>,
    Json(req): Json<GenerateRequest>,
) -> Response {
    if req.description.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenerateErr {
                error: "description is required".to_string(),
            }),
        )
            .into_response();
    }

    let prompt = format!(
        "Using the agentskills-spec builtin skill as your authoritative specification, \
         generate a complete and valid SKILL.md for the following description:\n\n{}\n\n\
         Output ONLY the raw SKILL.md content — no explanation, no markdown fences.",
        req.description.trim()
    );

    let conversation_id = Uuid::new_v4();
    let orch = state.orchestrator.clone();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        orch.submit_turn(&prompt, conversation_id, Interface::Web, None),
    )
    .await;

    match result {
        Ok(Ok(turn_result)) => {
            // Attempt to parse the LLM output as a valid SKILL.md so we can
            // return name/description/body separately.  On parse failure we
            // return the raw answer as the body so the user can still see it.
            let raw = turn_result.answer;
            let tmp_dir = std::path::PathBuf::from("/tmp");
            let (name, description, body) =
                match parse_skill_content(&raw, &tmp_dir, SkillSource::User) {
                    Ok(def) => (def.name, def.description, def.body.trim_start().to_string()),
                    Err(_) => (String::new(), String::new(), raw.trim_start().to_string()),
                };
            Json(GenerateOk {
                name,
                description,
                body,
            })
            .into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenerateErr {
                error: e.to_string(),
            }),
        )
            .into_response(),
        Err(_timeout) => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(GenerateErr {
                error: "Generation timed out after 30 seconds".to_string(),
            }),
        )
            .into_response(),
    }
}
