//! Tests for `resolve_home_channel_tools` and for the empty-tools fallback
//! path in `run_due_tasks` when no home channel is configured.

use assistant_core::{MessageBus, topic};
use chrono::{Duration, Utc};
use uuid::Uuid;
use wiremock::MockServer;

use super::{build, mount_answer};
use crate::scheduler::{resolve_home_channel_tools, run_due_tasks};

#[tokio::test]
async fn resolve_home_channel_tools_no_home_channel_returns_empty() {
    let server = MockServer::start().await;
    let (orch, storage) = build(&server.uri()).await;

    // Persona exists but has no home_channel set (default).
    storage
        .persona_store()
        .ensure_exists("default")
        .await
        .unwrap();

    let conv_id = Uuid::new_v4();
    let tools = resolve_home_channel_tools(&storage, &orch, conv_id).await;
    assert!(
        tools.is_empty(),
        "no home channel configured — should return empty tools"
    );
}

#[tokio::test]
async fn resolve_home_channel_tools_adapter_not_registered_returns_empty() {
    let server = MockServer::start().await;
    let (orch, storage) = build(&server.uri()).await;

    storage
        .persona_store()
        .ensure_exists("default")
        .await
        .unwrap();
    storage
        .persona_store()
        .set_home_channel("default", "slack", "#ops")
        .await
        .unwrap();

    // No adapter registered — should degrade gracefully.
    let conv_id = Uuid::new_v4();
    let tools = resolve_home_channel_tools(&storage, &orch, conv_id).await;
    assert!(
        tools.is_empty(),
        "no adapter running — should return empty tools"
    );
}

#[tokio::test]
async fn due_task_fires_without_output_tools_when_no_home_channel() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (orch, storage) = build(&server.uri()).await;

    let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
    let past = Utc::now() - Duration::seconds(60);
    store
        .insert("no-home-task", "0 0 * * *", "ping", false, Some(past))
        .await
        .unwrap();

    // Task fires normally even without home_channel.
    run_due_tasks(&storage, &orch).await.unwrap();

    let bus = storage.message_bus();
    let msg = bus
        .claim(topic::TURN_REQUEST, "test-consumer")
        .await
        .unwrap();
    assert!(msg.is_some(), "turn.request must be published");
}
