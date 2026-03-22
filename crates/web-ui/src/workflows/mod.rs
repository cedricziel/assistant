//! Workflow pages and API routes.

pub mod pages;

use axum::routing::{get, post};
use axum::Router;

use pages::WorkflowPagesState;

/// Router for workflow UI pages and JSON API.
pub fn workflow_pages_router() -> Router<WorkflowPagesState> {
    Router::new()
        .route(
            "/workflows",
            get(pages::list_workflows).post(pages::create_workflow),
        )
        .route("/workflows/new", get(pages::new_workflow_form))
        .route("/workflows/{id}", get(pages::show_workflow))
        .route(
            "/workflows/{id}/edit",
            get(pages::edit_workflow_form).post(pages::update_workflow),
        )
        .route("/workflows/{id}/delete", post(pages::delete_workflow))
        .route("/workflows/{id}/toggle", post(pages::toggle_workflow))
        .route("/workflows/{id}/editor", get(pages::editor_page))
        .route(
            "/api/workflows",
            get(pages::api_list_workflows).post(pages::api_create_workflow),
        )
        .route(
            "/api/workflows/{id}",
            get(pages::api_get_workflow).put(pages::api_update_workflow),
        )
        .route(
            "/api/workflows/{id}/activate",
            post(pages::api_activate_workflow),
        )
        .route(
            "/api/workflows/{id}/deactivate",
            post(pages::api_deactivate_workflow),
        )
        .route(
            "/api/workflows/{id}/test-run",
            post(pages::api_test_run_workflow),
        )
}
