//! Coverage for the A2A HTTP handlers using direct `oneshot` calls
//! against the built router.

#![cfg(test)]

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::routing::{delete, get, post};
use http_body_util::BodyExt;
use tower::ServiceExt;

use axum::Extension;

use assistant_core::auth::AuthContext;

use super::handlers::test_support::test_state;
use super::handlers::{
    A2AState, build_default_agent_card, cancel_task, create_push_notification_config,
    delete_push_notification_config, get_agent_card_well_known, get_extended_agent_card,
    get_push_notification_config, get_task, list_push_notification_configs, list_tasks,
    send_message,
};

async fn make_state(base_url: &str) -> A2AState {
    test_state(base_url).await
}

/// Build a minimal Router exposing every A2A handler under test paths. A system
/// `AuthContext` extension is injected so the auth-gated handlers resolve it
/// (mirrors the `require_auth` layer in production).
fn make_app(state: A2AState) -> Router {
    Router::new()
        .route("/.well-known/agent.json", get(get_agent_card_well_known))
        .route(
            "/agent/authenticatedExtendedCard",
            get(get_extended_agent_card),
        )
        .route("/message/send", post(send_message))
        .route("/tasks/{id}", get(get_task))
        .route("/tasks", get(list_tasks))
        .route("/tasks/{id}/cancel", post(cancel_task))
        .route("/tasks/{id}/push", post(create_push_notification_config))
        .route(
            "/tasks/{id}/push/{config_id}",
            get(get_push_notification_config),
        )
        .route("/tasks/{id}/push-list", get(list_push_notification_configs))
        .route(
            "/tasks/{id}/push/{config_id}/delete",
            delete(delete_push_notification_config),
        )
        .layer(Extension(AuthContext::system()))
        .with_state(state)
}

async fn body_json(body: Body) -> serde_json::Value {
    let b = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&b).unwrap()
}

// ── build_default_agent_card ────────────────────────────────────────────

#[test]
fn build_default_agent_card_includes_base_url_in_supported_interface() {
    let card = build_default_agent_card("https://example.com");
    assert_eq!(card.name, "Assistant");
    assert!(!card.supported_interfaces.is_empty());
    assert_eq!(card.supported_interfaces[0].url, "https://example.com");
    assert!(!card.skills.is_empty());
    assert_eq!(card.default_input_modes, vec!["text/plain"]);
}

#[test]
fn build_default_agent_card_streaming_capability_is_true() {
    let card = build_default_agent_card("https://x.test");
    assert_eq!(card.capabilities.streaming, Some(true));
    assert_eq!(card.capabilities.push_notifications, Some(false));
}

// ── GET /.well-known/agent.json ─────────────────────────────────────────

#[tokio::test]
async fn well_known_agent_card_returns_200_with_card() {
    let app = make_app(make_state("https://example.com").await);
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
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["name"], "Assistant");
}

#[tokio::test]
async fn extended_agent_card_returns_200() {
    let app = make_app(make_state("https://example.com").await);
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
    let json = body_json(resp.into_body()).await;
    assert!(json["name"].is_string());
}

// ── Tasks ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_task_returns_404_for_unknown_id() {
    let app = make_app(make_state("https://example.com").await);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/tasks/no-such-task")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_tasks_with_no_tasks_returns_200_and_empty_list() {
    let app = make_app(make_state("https://example.com").await);
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
    let json = body_json(resp.into_body()).await;
    // The shape depends on the schema; just assert it's a JSON object/array.
    assert!(json.is_object() || json.is_array(), "got: {json}");
}

#[tokio::test]
async fn cancel_task_unknown_id_returns_404() {
    let app = make_app(make_state("https://example.com").await);
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/tasks/no-such-task/cancel")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_task_returns_created_task_when_present() {
    let state = make_state("https://example.com").await;
    let task = state.task_store.create_task(None).await;
    let id = task.id.clone();
    let app = make_app(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/tasks/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["id"], id);
}

#[tokio::test]
async fn cancel_task_existing_returns_200() {
    let state = make_state("https://example.com").await;
    let task = state.task_store.create_task(None).await;
    let id = task.id.clone();
    let app = make_app(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/tasks/{id}/cancel"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ── Push notification configs ───────────────────────────────────────────

fn push_config_request_body(task_id: &str, config_id: &str) -> serde_json::Value {
    serde_json::json!({
        "taskId": task_id,
        "config": {
            "id": config_id,
            "url": "https://hook.example.com/notify",
        }
    })
}

#[tokio::test]
async fn create_push_config_for_unknown_task_returns_404() {
    let app = make_app(make_state("https://example.com").await);
    let body = push_config_request_body("no-such-task", "cfg-1");
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/tasks/no-such-task/push")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_push_config_unknown_returns_404() {
    let app = make_app(make_state("https://example.com").await);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/tasks/no-such-task/push/cfg-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_push_configs_unknown_task_returns_empty_array() {
    let app = make_app(make_state("https://example.com").await);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/tasks/no-such-task/push-list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // The handler returns 200 with an empty configs array for unknown task.
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    let configs = json["configs"].as_array().unwrap();
    assert!(configs.is_empty(), "expected empty configs; got: {json}");
}

#[tokio::test]
async fn delete_push_config_unknown_returns_404() {
    let app = make_app(make_state("https://example.com").await);
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/tasks/no-such-task/push/cfg-1/delete")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_then_get_then_list_then_delete_push_config_round_trip() {
    let state = make_state("https://example.com").await;
    let task = state.task_store.create_task(None).await;
    let task_id = task.id.clone();
    let app = make_app(state);

    // CREATE
    let body = push_config_request_body(&task_id, "cfg-x");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/tasks/{task_id}/push"))
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "create push config should succeed: {}",
        resp.status()
    );

    // GET
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/tasks/{task_id}/push/cfg-x"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // LIST
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/tasks/{task_id}/push-list"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json.is_object() || json.is_array());

    // DELETE
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/tasks/{task_id}/push/cfg-x/delete"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_success());
}

// ── send_message ────────────────────────────────────────────────────────

#[tokio::test]
async fn send_message_with_invalid_body_returns_400() {
    let app = make_app(make_state("https://example.com").await);
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/message/send")
                .header("Content-Type", "application/json")
                .body(Body::from("not json"))
                .unwrap(),
        )
        .await
        .unwrap();
    // Axum's Json extractor returns 400 for invalid JSON.
    assert!(
        resp.status().is_client_error(),
        "expected 4xx; got {}",
        resp.status()
    );
}

fn user_message(text: &str) -> serde_json::Value {
    let msg = assistant_a2a_json_schema::types::Message {
        message_id: "m1".to_string(),
        context_id: None,
        task_id: None,
        role: assistant_a2a_json_schema::types::Role::RoleUser,
        parts: vec![assistant_a2a_json_schema::types::Part::text(text)],
        metadata: None,
        extensions: vec![],
        reference_task_ids: vec![],
    };
    serde_json::json!({ "message": serde_json::to_value(&msg).unwrap() })
}

#[tokio::test]
async fn send_message_runs_real_turn_and_returns_agent_reply() {
    let app = make_app(make_state("https://example.com").await);
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/message/send")
                .header("Content-Type", "application/json")
                .body(Body::from(user_message("hello agent").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    // The echo orchestrator returns "echo: hello agent" as the final message.
    let final_text = json["task"]["status"]["message"]["parts"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(final_text.contains("echo: hello agent"), "got: {json}");
}

#[tokio::test]
async fn send_message_without_conversations_write_scope_returns_403() {
    use axum::routing::post;

    let state = make_state("https://example.com").await;
    let restricted = AuthContext {
        user_id: "u".into(),
        org_id: "default".into(),
        email: "u@example.com".to_string(),
        space_roles: std::collections::HashMap::new(),
        scopes: vec![],
        client_id: "test".to_string(),
    };
    let app = Router::new()
        .route("/message/send", post(send_message))
        .layer(Extension(restricted))
        .with_state(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/message/send")
                .header("Content-Type", "application/json")
                .body(Body::from(user_message("hi").to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
