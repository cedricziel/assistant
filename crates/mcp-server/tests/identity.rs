//! Tool calls run as the identity the request carries.

mod common;

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::ToolSpec;
use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
use assistant_runtime::StubOrchestrationEngine;
use assistant_tool_executor::StubToolDispatcher;
use common::*;
use rmcp::model::ErrorCode;
use serde_json::json;

fn user(role: Role) -> AuthContext {
    AuthContext {
        user_id: UserId::from("user-1"),
        org_id: OrgId::from("org-1"),
        email: "user@example.com".into(),
        space_roles: HashMap::from([(SpaceId::from("space-1"), role)]),
        // No scopes at all: the worst case for an `mcp:execute`-only token, which
        // carries none of the resource scopes the web API checks.
        scopes: Vec::new(),
        client_id: "mcp-client".into(),
    }
}

fn echo_stub() -> Arc<StubToolDispatcher> {
    Arc::new(StubToolDispatcher::new().with_specs(vec![ToolSpec {
        name: "echo".into(),
        description: "echo".into(),
        params_schema: json!({}),
        is_mutating: false,
        requires_confirmation: false,
    }]))
}

#[tokio::test]
async fn tool_call_runs_with_the_callers_user_org_and_space() {
    let stub = echo_stub();
    let client = connect_as(
        default_stubs()
            .executor(stub.clone())
            .server(allow_everything()),
        user(Role::Member),
    )
    .await;

    client
        .call_tool(call_params("echo", json!({})))
        .await
        .unwrap();

    let calls = stub.recorded_calls();
    assert_eq!(calls.len(), 1, "echo reached the dispatcher");
    let ctx = &calls[0].ctx;
    assert_eq!(ctx.user_id, Some(UserId::from("user-1")));
    assert_eq!(ctx.org_id, Some(OrgId::from("org-1")));
    assert_eq!(ctx.space_id, Some(SpaceId::from("space-1")));
}

#[tokio::test]
async fn member_without_resource_scopes_can_run_tools_and_prompts() {
    let stub = echo_stub();
    let orchestrator = Arc::new(StubOrchestrationEngine::new());
    let client = connect_as(
        default_stubs()
            .executor(stub.clone())
            .orchestrator(orchestrator.clone())
            .server(allow_everything()),
        user(Role::Member),
    )
    .await;

    let echoed = client
        .call_tool(call_params("echo", json!({})))
        .await
        .unwrap();
    let prompted = client
        .call_tool(call_params("run-prompt", json!({"prompt": "hi"})))
        .await
        .unwrap();

    assert_ne!(echoed.is_error, Some(true), "{echoed:?}");
    assert_ne!(prompted.is_error, Some(true), "{prompted:?}");
    assert_eq!(stub.recorded_calls().len(), 1);
    assert_eq!(orchestrator.submit_calls().len(), 1);
}

#[tokio::test]
async fn viewer_cannot_run_tools() {
    let stub = echo_stub();
    let client = connect_as(
        default_stubs()
            .executor(stub.clone())
            .server(allow_everything()),
        user(Role::Viewer),
    )
    .await;

    let result = client
        .call_tool(call_params("echo", json!({})))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(true), "{result:?}");
    assert!(
        stub.recorded_calls().is_empty(),
        "viewer reached the dispatcher"
    );
}

#[tokio::test]
async fn permission_policy_still_applies_to_a_member() {
    let stub = echo_stub();
    let client = connect_as(
        default_stubs()
            .executor(stub.clone())
            .server(policy(&[], &["echo"])),
        user(Role::Member),
    )
    .await;

    let result = client
        .call_tool(call_params("echo", json!({})))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(true), "{result:?}");
    assert!(stub.recorded_calls().is_empty());
}

#[tokio::test]
async fn run_prompt_submits_the_turn_as_the_caller() {
    let orchestrator = Arc::new(StubOrchestrationEngine::new());
    let client = connect_as(
        default_stubs()
            .orchestrator(orchestrator.clone())
            .server(allow_everything()),
        user(Role::Member),
    )
    .await;

    client
        .call_tool(call_params("run-prompt", json!({"prompt": "hi"})))
        .await
        .unwrap();

    let calls = orchestrator.submit_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].auth.user_id, UserId::from("user-1"));
    assert_eq!(calls[0].auth.org_id, OrgId::from("org-1"));
}

#[tokio::test]
async fn stdio_style_system_identity_runs_tools_in_the_default_space() {
    let stub = echo_stub();
    let client = default_stubs().executor(stub.clone()).connect().await;

    client
        .call_tool(call_params("echo", json!({})))
        .await
        .unwrap();

    let ctx = &stub.recorded_calls()[0].ctx;
    assert_eq!(ctx.user_id, Some(UserId::from("system")));
    assert_eq!(ctx.space_id, Some(SpaceId::from("default")));
}

#[tokio::test]
async fn call_without_identity_is_rejected_not_run_as_system() {
    let stub = echo_stub();
    let orchestrator = Arc::new(StubOrchestrationEngine::new());
    let client = connect_anonymous(
        default_stubs()
            .executor(stub.clone())
            .orchestrator(orchestrator.clone())
            .server(allow_everything()),
    )
    .await;

    let tool = client.call_tool(call_params("echo", json!({}))).await;
    let prompt = client
        .call_tool(call_params("run-prompt", json!({"prompt": "hi"})))
        .await;

    assert_eq!(code(tool.unwrap_err()), ErrorCode::INVALID_REQUEST);
    assert_eq!(code(prompt.unwrap_err()), ErrorCode::INVALID_REQUEST);
    assert!(stub.recorded_calls().is_empty());
    assert!(orchestrator.submit_calls().is_empty());
}
