//! `run-prompt` runs as a task for clients that declare the Tasks extension.

mod common;

use std::sync::Arc;
use std::time::Duration;

use assistant_core::auth::AuthContext;
use assistant_core::identity::UserId;
use assistant_mcp_server::{AssistantMcpServer, TaskStore};
use assistant_runtime::StubOrchestrationEngine;
use common::*;
use rmcp::ServiceExt;
use rmcp::model::{
    CallToolResponse, CancelTaskParams, ErrorCode, GetTaskParams, TASKS_EXTENSION_ID, TaskPayload,
    TaskStatus,
};
use serde_json::json;

async fn task_client_as(server: AssistantMcpServer, auth: AuthContext) -> TaskClient {
    TaskAwareClient
        .serve(spawn_server_as(server, auth))
        .await
        .expect("client handshake")
}

fn server_with(stub: Arc<StubOrchestrationEngine>, tasks: TaskStore) -> AssistantMcpServer {
    default_stubs()
        .orchestrator(stub)
        .tasks(tasks)
        .server(allow_everything())
}

fn other_user() -> AuthContext {
    let mut auth = AuthContext::system();
    auth.user_id = UserId::from("someone-else");
    auth
}

/// Polls `task_id` until it leaves `working`, failing after five seconds.
async fn settled(client: &TaskClient, task_id: &str) -> TaskPayload {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let payload = task_payload(client, task_id).await;
            if payload.status() != TaskStatus::Working {
                return payload;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("task settles within five seconds")
}

#[tokio::test]
async fn server_advertises_the_tasks_extension() {
    let client = connect(default_stubs().server(allow_everything())).await;

    let info = client.peer_info().expect("server info");
    let extensions = info.capabilities.extensions.clone().unwrap_or_default();
    assert!(
        extensions.contains_key(TASKS_EXTENSION_ID),
        "extensions: {extensions:?}"
    );
}

#[tokio::test]
async fn task_aware_client_gets_a_working_task_for_run_prompt() {
    let stub = Arc::new(StubOrchestrationEngine::new().hold_turns());
    let client = task_client_as(
        server_with(stub, TaskStore::default()),
        AuthContext::system(),
    )
    .await;

    let task = start_task(&client).await;

    assert_eq!(task.task.status, TaskStatus::Working, "{task:?}");
    assert!(!task.task.task_id.is_empty(), "{task:?}");
    assert_eq!(
        task_payload(&client, &task.task.task_id).await.status(),
        TaskStatus::Working
    );
}

#[tokio::test]
async fn client_without_the_extension_gets_a_blocking_result() {
    let stub = StubOrchestrationEngine::new().queue_turn(Ok(turn("done")));
    let client = connect(server_with(Arc::new(stub), TaskStore::default())).await;

    let response = client
        .call_tool_once(call_params("run-prompt", json!({"prompt": "hi"})))
        .await
        .expect("run-prompt");

    match response {
        CallToolResponse::Complete(result) => assert_eq!(text_of(&result), "done"),
        other => panic!("expected a completed result, got {other:?}"),
    }
}

#[tokio::test]
async fn polling_reaches_completed_with_the_tool_result() {
    let stub = StubOrchestrationEngine::new().queue_turn(Ok(turn("done")));
    let client = task_client_as(
        server_with(Arc::new(stub), TaskStore::default()),
        AuthContext::system(),
    )
    .await;

    let task = start_task(&client).await;

    match settled(&client, &task.task.task_id).await {
        TaskPayload::Completed { result } => {
            assert_eq!(
                result["content"][0]["text"],
                json!("done"),
                "result: {result:?}"
            );
        }
        other => panic!("expected completed, got {other:?}"),
    }
}

#[tokio::test]
async fn failed_turn_settles_as_failed() {
    let stub = StubOrchestrationEngine::new().queue_turn(Err(anyhow::anyhow!("model down")));
    let client = task_client_as(
        server_with(Arc::new(stub), TaskStore::default()),
        AuthContext::system(),
    )
    .await;

    let task = start_task(&client).await;

    let payload = settled(&client, &task.task.task_id).await;
    assert_eq!(payload.status(), TaskStatus::Failed, "{payload:?}");
}

#[tokio::test]
async fn cancelling_a_task_cancels_the_turn() {
    let stub = Arc::new(StubOrchestrationEngine::new().hold_turns());
    let client = task_client_as(
        server_with(stub.clone(), TaskStore::default()),
        AuthContext::system(),
    )
    .await;
    let task = start_task(&client).await;

    client
        .cancel_task(CancelTaskParams::new(task.task.task_id.clone()))
        .await
        .expect("tasks/cancel");

    let payload = settled(&client, &task.task.task_id).await;
    assert_eq!(payload.status(), TaskStatus::Cancelled, "{payload:?}");
    let submitted = stub.submit_calls()[0].request_id;
    assert_eq!(submitted, stub.cancel_calls().first().copied());
}

#[tokio::test]
async fn another_users_task_is_not_found() {
    let stub = Arc::new(StubOrchestrationEngine::new().hold_turns());
    let tasks = TaskStore::default();
    let owner = task_client_as(
        server_with(stub.clone(), tasks.clone()),
        AuthContext::system(),
    )
    .await;
    let intruder = task_client_as(server_with(stub.clone(), tasks), other_user()).await;
    let task = start_task(&owner).await;
    let task_id = task.task.task_id;

    let get = intruder
        .get_task(GetTaskParams::new(task_id.clone()))
        .await
        .expect_err("another user's task");
    let cancel = intruder
        .cancel_task(CancelTaskParams::new(task_id.clone()))
        .await
        .expect_err("another user's task");

    assert_eq!(code(get), ErrorCode::INVALID_PARAMS);
    assert_eq!(code(cancel), ErrorCode::INVALID_PARAMS);
    assert!(stub.cancel_calls().is_empty(), "the owner's turn runs on");
    assert_eq!(
        task_payload(&owner, &task_id).await.status(),
        TaskStatus::Working
    );
}

#[tokio::test]
async fn unknown_task_is_invalid_params() {
    let client = task_client_as(
        default_stubs().server(allow_everything()),
        AuthContext::system(),
    )
    .await;

    let error = client
        .get_task(GetTaskParams::new("no-such-task"))
        .await
        .expect_err("unknown task");

    assert_eq!(code(error), ErrorCode::INVALID_PARAMS);
}
