//! Tests for `run_due_tasks` — cron-driven dispatch into the bus.

use assistant_core::{MessageBus, topic};
use chrono::{Duration, Utc};
use wiremock::MockServer;

use super::{build, mount_answer};
use crate::scheduler::SCHEDULER_USER_ID;
use crate::scheduler::run_due_tasks;

#[tokio::test]
async fn due_task_is_published_to_bus() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (orch, storage) = build(&server.uri()).await;

    let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
    let past = Utc::now() - Duration::seconds(60);
    store
        .insert("test-task", "0 0 * * *", "say hello", false, Some(past))
        .await
        .unwrap();

    run_due_tasks(&storage, &orch).await.unwrap();

    let bus = storage.message_bus();
    let msg = bus
        .claim(topic::TURN_REQUEST, "test-consumer")
        .await
        .unwrap();
    assert!(msg.is_some(), "turn.request should be on the bus");
    let msg = msg.unwrap();
    assert_eq!(
        msg.interface.as_deref(),
        Some("Scheduler"),
        "interface must be Scheduler"
    );
    assert_eq!(
        msg.user_id.as_deref(),
        Some(SCHEDULER_USER_ID),
        "user_id must be the scheduler constant"
    );
}

#[tokio::test]
async fn once_task_is_disabled_after_dispatch() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (orch, storage) = build(&server.uri()).await;

    let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
    let past = Utc::now() - Duration::seconds(60);
    let id = store
        .insert("once-task", "", "ping", true, Some(past))
        .await
        .unwrap();

    run_due_tasks(&storage, &orch).await.unwrap();

    let tasks = store.list_all().await.unwrap();
    let task = tasks.iter().find(|t| t.id == id).unwrap();
    assert!(
        !task.enabled,
        "one-shot task must be disabled after dispatch"
    );
    assert!(task.last_run.is_some(), "last_run must be recorded");
}

#[tokio::test]
async fn recurring_task_next_run_is_advanced() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (orch, storage) = build(&server.uri()).await;

    let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
    let past = Utc::now() - Duration::seconds(60);
    let id = store
        .insert("cron-task", "0 0 9 * * *", "morning", false, Some(past))
        .await
        .unwrap();

    run_due_tasks(&storage, &orch).await.unwrap();

    let tasks = store.list_all().await.unwrap();
    let task = tasks.iter().find(|t| t.id == id).unwrap();
    assert!(task.enabled, "recurring task must stay enabled");
    assert!(
        task.next_run.is_some_and(|nr| nr > Utc::now()),
        "next_run must be advanced into the future"
    );
}

#[tokio::test]
async fn not_due_task_is_not_dispatched() {
    let server = MockServer::start().await;
    let (orch, storage) = build(&server.uri()).await;

    let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
    let future = Utc::now() + Duration::hours(1);
    store
        .insert("future-task", "0 0 9 * * *", "later", false, Some(future))
        .await
        .unwrap();

    run_due_tasks(&storage, &orch).await.unwrap();

    let bus = storage.message_bus();
    let msg = bus
        .claim(topic::TURN_REQUEST, "test-consumer")
        .await
        .unwrap();
    assert!(
        msg.is_none(),
        "no turn.request should be published for a future task"
    );
}
