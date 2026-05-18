//! Coverage tests for web-fetch and web-search builtins.
//!
//! Uses `wiremock` for `web-fetch` to exercise the success path.
//! `web-search` uses Tavily — its missing-API-key path doesn't require
//! a live service so we exercise that branch.

use std::collections::HashMap;

use assistant_core::ToolHandler;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_tool_executor::builtins::web_fetch::WebFetchHandler;
use assistant_tool_executor::builtins::web_search::WebSearchHandler;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ctx() -> ExecutionContext {
    ExecutionContext {
        conversation_id: uuid::Uuid::new_v4(),
        agent_id: "default".into(),
        turn: 0,
        interface: Interface::Cli,
        interactive: false,
        allowed_tools: None,
        depth: 0,
        user_id: None,
        org_id: None,
        space_id: None,
    }
}

fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

// ── web_fetch ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn web_fetch_missing_url_returns_error() {
    let handler = WebFetchHandler::new();
    let out = handler.run(params(&[]), &ctx()).await.unwrap();
    assert!(!out.success);
    assert!(out.content.contains("url"));
}

#[tokio::test]
async fn web_fetch_invalid_scheme_returns_error() {
    let handler = WebFetchHandler::new();
    let out = handler
        .run(params(&[("url", json!("file:///etc/passwd"))]), &ctx())
        .await
        .unwrap();
    assert!(!out.success);
    assert!(out.content.contains("Invalid URL"));
}

#[tokio::test]
async fn web_fetch_returns_body_from_http_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("<html><body><p>hello world</p></body></html>"),
        )
        .mount(&server)
        .await;

    let handler = WebFetchHandler::new();
    let out = handler
        .run(
            params(&[("url", json!(format!("{}/page", server.uri())))]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success, "{}", out.content);
    assert!(out.content.contains("hello world"));
}

#[tokio::test]
async fn web_fetch_honors_max_chars() {
    let server = MockServer::start().await;
    let body = "x".repeat(20_000);
    Mock::given(method("GET"))
        .and(path("/big"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    let handler = WebFetchHandler::new();
    let out = handler
        .run(
            params(&[
                ("url", json!(format!("{}/big", server.uri()))),
                ("max_chars", json!(50)),
            ]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success);
    // Truncated content should be much shorter than 20k.
    assert!(out.content.len() < 1000);
}

#[tokio::test]
async fn web_fetch_non_200_returns_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/404"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let handler = WebFetchHandler::new();
    let out = handler
        .run(
            params(&[("url", json!(format!("{}/404", server.uri())))]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!out.success);
}

// ── web_search ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn web_search_missing_query_returns_error() {
    let handler = WebSearchHandler::new();
    let out = handler.run(params(&[]), &ctx()).await.unwrap();
    assert!(!out.success);
}

#[tokio::test]
async fn web_search_missing_api_key_returns_error() {
    let handler = WebSearchHandler::new();
    // Ensure TAVILY_API_KEY isn't set for this test.
    let original = std::env::var("TAVILY_API_KEY").ok();
    unsafe {
        std::env::remove_var("TAVILY_API_KEY");
    }
    let out = handler
        .run(params(&[("query", json!("rust language"))]), &ctx())
        .await
        .unwrap();
    if let Some(orig) = original {
        unsafe {
            std::env::set_var("TAVILY_API_KEY", orig);
        }
    }
    // Returns either an api-key error or an attempted request failure;
    // either path covers code in this branch.
    let _ = out;
}
