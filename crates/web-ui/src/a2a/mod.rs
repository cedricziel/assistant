//! A2A protocol HTTP handler layer.
//!
//! Exposes the Agent-to-Agent protocol endpoints as axum routes, backed by
//! an in-memory task store. The handler layer is transport-agnostic -- it
//! accepts and returns the canonical JSON types from `assistant_a2a_json_schema`.

pub mod agent_store;
pub mod handlers;
pub mod pages;
pub mod task_store;

use axum::routing::{get, post};
use axum::Router;

use crate::a2a::handlers::A2AState;
use crate::a2a::pages::AgentPagesState;

/// Builds the axum [`Router`] for all A2A protocol endpoints.
///
/// Mount this under a prefix (e.g., `/a2a`) or at the root, depending on your
/// deployment topology.
pub fn router() -> Router<A2AState> {
    Router::new()
        // -- Agent Card discovery --
        .route(
            "/.well-known/agent.json",
            get(handlers::get_agent_card_well_known),
        )
        .route(
            "/agent/authenticatedExtendedCard",
            get(handlers::get_extended_agent_card),
        )
        // -- Message operations --
        .route("/message/send", post(handlers::send_message))
        .route("/message/stream", post(handlers::send_message_streaming))
        // -- Task operations --
        .route("/tasks", get(handlers::list_tasks))
        .route("/tasks/{id}", get(handlers::get_task))
        .route("/tasks/{id}/cancel", post(handlers::cancel_task))
        .route("/tasks/{id}/subscribe", get(handlers::subscribe_to_task))
        // -- Push notification config operations --
        .route(
            "/tasks/{task_id}/pushNotificationConfigs",
            get(handlers::list_push_notification_configs)
                .post(handlers::create_push_notification_config),
        )
        .route(
            "/tasks/{task_id}/pushNotificationConfigs/{config_id}",
            get(handlers::get_push_notification_config)
                .delete(handlers::delete_push_notification_config),
        )
}

/// Builds the axum [`Router`] for agent management HTML pages.
pub fn agent_pages_router() -> Router<AgentPagesState> {
    Router::new()
        .route("/agents", get(pages::list_agents).post(pages::create_agent))
        .route("/agents/new", get(pages::new_agent_form))
        .route("/agents/{id}", get(pages::show_agent))
        .route(
            "/agents/{id}/edit",
            get(pages::edit_agent_form).post(pages::update_agent),
        )
        .route("/agents/{id}/delete", post(pages::delete_agent))
        .route("/agents/{id}/set-default", post(pages::set_default_agent))
        .route("/agents/{id}/card.json", get(pages::show_agent_card_json))
}
