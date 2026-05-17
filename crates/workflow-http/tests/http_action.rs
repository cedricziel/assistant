//! Integration tests for `HttpRequestActionExecutor`.
//!
//! Uses `wiremock::MockServer` + an injected `reqwest::Client` to exercise
//! the action against deterministic HTTP responses.

use std::time::Duration;

use assistant_workflow::{WorkflowActionExecutor, WorkflowActionInput};
use assistant_workflow_http::HttpRequestActionExecutor;
use reqwest::Client;
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_input(config: Value, trigger_payload: Value) -> WorkflowActionInput {
    WorkflowActionInput {
        run_id: uuid::Uuid::new_v4(),
        workflow_id: uuid::Uuid::new_v4(),
        node_id: "http_action".into(),
        config,
        trigger_payload,
    }
}

fn is_success(result: &assistant_workflow::WorkflowActionResult) -> bool {
    result.outcome_label == "success"
}

#[tokio::test]
async fn happy_path_post_returns_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/hook"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;

    let executor = HttpRequestActionExecutor::with_client(Client::new(), Duration::from_secs(5));
    let input = make_input(
        json!({
            "url": format!("{}/hook", server.uri()),
            "method": "POST",
            "body": {"hello": "world"},
        }),
        json!({}),
    );

    let result = executor.execute(input).await.unwrap();
    assert!(is_success(&result), "expected success outcome");
    let output = result.output.expect("output should be present");
    assert_eq!(output.get("status").and_then(Value::as_u64), Some(200));
    assert_eq!(output.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        output
            .get("body_json")
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[tokio::test]
async fn non_success_status_returns_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/fail"))
        .respond_with(ResponseTemplate::new(503).set_body_string("nope"))
        .mount(&server)
        .await;

    let executor = HttpRequestActionExecutor::with_client(Client::new(), Duration::from_secs(2));
    let input = make_input(json!({"url": format!("{}/fail", server.uri())}), json!({}));

    let result = executor.execute(input).await.unwrap();
    assert!(!is_success(&result), "expected failure outcome");
    let output = result.output.expect("output should be present");
    assert_eq!(output.get("status").and_then(Value::as_u64), Some(503));
    assert_eq!(output.get("ok").and_then(Value::as_bool), Some(false));
}

#[tokio::test]
async fn body_from_trigger_uses_trigger_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/echo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let executor = HttpRequestActionExecutor::with_client(Client::new(), Duration::from_secs(2));
    let input = make_input(
        json!({
            "url": format!("{}/echo", server.uri()),
            "body_from_trigger": true,
        }),
        json!({"from_trigger": true}),
    );

    let result = executor.execute(input).await.unwrap();
    assert!(is_success(&result), "should succeed");
}

#[tokio::test]
async fn host_blocklist_rejects_request() {
    let server = MockServer::start().await;
    let host = url::Url::parse(&server.uri())
        .unwrap()
        .host_str()
        .unwrap()
        .to_string();

    let executor = HttpRequestActionExecutor::with_client(Client::new(), Duration::from_secs(2))
        .with_blocked_hosts([host]);

    let input = make_input(
        json!({"url": format!("{}/anything", server.uri())}),
        json!({}),
    );

    let err = executor.execute(input).await.unwrap_err();
    assert!(
        err.to_string().contains("blocked by workflow policy"),
        "expected blocklist error, got: {err}"
    );
}

#[tokio::test]
async fn host_allowlist_rejects_unlisted() {
    let server = MockServer::start().await;
    let executor = HttpRequestActionExecutor::with_client(Client::new(), Duration::from_secs(2))
        .with_allowed_hosts(["example.com".to_string()]);

    let input = make_input(
        json!({"url": format!("{}/anything", server.uri())}),
        json!({}),
    );

    let err = executor.execute(input).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("not in allowed workflow host list"),
        "expected allowlist rejection, got: {err}"
    );
}

#[tokio::test]
async fn json_pointer_extracts_field() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/data"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data": {"nested": "value"}})),
        )
        .mount(&server)
        .await;

    let executor = HttpRequestActionExecutor::with_client(Client::new(), Duration::from_secs(2));
    let input = make_input(
        json!({
            "url": format!("{}/data", server.uri()),
            "response": {"json_pointer": "/data/nested"},
        }),
        json!({}),
    );

    let result = executor.execute(input).await.unwrap();
    let output = result.output.expect("output should be present");
    assert_eq!(
        output.get("mapped").and_then(Value::as_str),
        Some("value"),
        "mapped should hold the value at /data/nested"
    );
}

#[tokio::test]
async fn invalid_url_fails() {
    let executor = HttpRequestActionExecutor::with_client(Client::new(), Duration::from_secs(2));
    let input = make_input(json!({"url": "not-a-url"}), json!({}));

    let err = executor.execute(input).await.unwrap_err();
    assert!(
        err.to_string().contains("invalid url"),
        "expected URL parse error, got: {err}"
    );
}

#[tokio::test]
async fn missing_url_fails() {
    let executor = HttpRequestActionExecutor::with_client(Client::new(), Duration::from_secs(2));
    let input = make_input(json!({}), json!({}));

    let err = executor.execute(input).await.unwrap_err();
    assert!(
        err.to_string().contains("missing 'url'"),
        "expected missing-url error, got: {err}"
    );
}
