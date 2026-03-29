//! Skill management UI — list, view, create, edit, and delete skills.

pub mod pages;

use axum::routing::{get, post};
use axum::Router;

use pages::SkillsPagesState;

/// Builds the axum [`Router`] for skill management HTML pages.
pub fn skills_router() -> Router<SkillsPagesState> {
    Router::new()
        .route("/skills", get(pages::list_skills).post(pages::create_skill))
        .route("/skills/new", get(pages::new_skill_form))
        .route("/skills/{name}", get(pages::show_skill))
        .route(
            "/skills/{name}/edit",
            get(pages::edit_skill_form).post(pages::update_skill),
        )
        .route("/skills/{name}/delete", post(pages::delete_skill))
}
