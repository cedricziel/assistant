//! Workflow pages and API routes.

pub mod pages;

use axum::routing::{get, post};
use axum::Router;

use pages::WorkflowPagesState;

/// Router for auth-protected workflow HTML pages (kept for reference; no longer mounted).
///
/// All JSON API routes have moved to `crate::api::workflows::workflows_api_router`.
#[allow(dead_code)]
pub fn workflow_pages_router() -> Router<WorkflowPagesState> {
    Router::new()
        .route(
            "/workflows",
            get(pages::list_workflows).post(pages::create_workflow),
        )
        .route("/workflows/new", get(pages::new_workflow_form))
        .route("/workflows/{id}", get(pages::show_workflow))
        .route(
            "/workflows/{id}/runs/{run_id}",
            get(pages::show_workflow_run),
        )
        .route(
            "/workflows/{id}/edit",
            get(pages::edit_workflow_form).post(pages::update_workflow),
        )
        .route("/workflows/{id}/delete", post(pages::delete_workflow))
        .route("/workflows/{id}/toggle", post(pages::toggle_workflow))
        .route(
            "/workflows/{id}/webhook/rotate",
            post(pages::rotate_workflow_webhook),
        )
        .route("/workflows/{id}/editor", get(pages::editor_page))
}

/// Router for public inbound workflow webhook trigger endpoints.
pub fn workflow_public_router() -> Router<WorkflowPagesState> {
    Router::new().route(
        "/workflow-hooks/{id}/{token}",
        post(pages::public_webhook_trigger),
    )
}
