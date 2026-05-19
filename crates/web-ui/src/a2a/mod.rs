//! A2A protocol HTTP handler layer.
//!
//! Exposes the Agent-to-Agent protocol endpoints as axum routes, backed by
//! an in-memory task store. The handler layer is transport-agnostic -- it
//! accepts and returns the canonical JSON types from `assistant_a2a_json_schema`.

pub mod agent_store;
pub mod handlers;
#[cfg(test)]
mod handlers_tests;
pub mod task_store;

use axum::Router;
use axum::routing::{get, post};

use crate::a2a::handlers::A2AState;

/// Public A2A routes that do **not** require authentication.
///
/// Per the A2A spec the agent card must be publicly discoverable so that
/// callers can learn the auth requirements before making authenticated calls.
pub fn public_router() -> Router<A2AState> {
    Router::new().route(
        "/.well-known/agent.json",
        get(handlers::get_agent_card_well_known),
    )
}

/// Protected A2A routes that **require** authentication.
///
/// Includes the authenticated extended card, message operations, task
/// operations, and push notification configuration.
pub fn protected_router() -> Router<A2AState> {
    Router::new()
        // -- Agent Card (authenticated) --
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

#[cfg(test)]
mod router_tests {
    use super::*;
    use crate::a2a::handlers::build_default_agent_card;
    use crate::a2a::task_store::TaskStore;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn make_state() -> A2AState {
        A2AState {
            task_store: TaskStore::new(),
            agent_card: build_default_agent_card("https://example.com"),
        }
    }

    #[tokio::test]
    async fn public_router_serves_well_known_agent_card() {
        let app = public_router().with_state(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/agent.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protected_router_exposes_authenticated_extended_card() {
        let app = protected_router().with_state(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/agent/authenticatedExtendedCard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protected_router_exposes_tasks_endpoint() {
        let app = protected_router().with_state(make_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/tasks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn public_router_does_not_expose_protected_paths() {
        let app = public_router().with_state(make_state());
        // /tasks is not on the public router; should 404 (no matching route).
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/tasks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
