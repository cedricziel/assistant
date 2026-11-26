//! Tests for `reap_stale_and_recover` — crash recovery for orphaned SSE
//! runs and stale bus messages.

use assistant_core::{MessageBus, topic};
use assistant_storage::ConversationEventStore as _;
use uuid::Uuid;
use wiremock::MockServer;

use super::{build, mount_answer};
use crate::scheduler::reap_stale_and_recover;

#[tokio::test]
async fn reap_stale_closes_orphaned_run() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (_orch, storage) = build(&server.uri()).await;

    let event_store = storage.conversation_event_store();
    let run_id = "run-orphan-001";
    let conv_id = "conv-orphan-001";

    // Simulate an orphaned run: run_started but no terminal event.
    event_store
        .append_event(
            run_id,
            conv_id,
            0,
            "run_started",
            &serde_json::json!({"run_id": run_id}),
        )
        .await
        .unwrap();
    event_store
        .append_event(
            run_id,
            conv_id,
            1,
            "token",
            &serde_json::json!({"token": "partial"}),
        )
        .await
        .unwrap();

    // Backdate so it's older than the stale threshold.
    sqlx::query(
        "UPDATE conversation_events SET created_at = datetime('now', '-10 minutes') \
         WHERE run_id = ?1",
    )
    .bind(run_id)
    .execute(&storage.pool)
    .await
    .unwrap();

    let bus = storage.message_bus();
    reap_stale_and_recover(&storage, &bus).await;

    // The orphaned run should now have a synthetic terminal event.
    assert!(
        event_store.is_run_complete(run_id).await.unwrap(),
        "orphaned run should be marked complete after recovery"
    );

    let events = event_store.list_events_since(run_id, 2).await.unwrap();
    assert_eq!(events.len(), 1, "should have exactly one synthetic event");
    assert_eq!(events[0].event_type, "agent_error");
    assert_eq!(events[0].payload["synthetic"], true);
}

#[tokio::test]
async fn reap_stale_skips_run_with_active_bus_message() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (_orch, storage) = build(&server.uri()).await;

    let event_store = storage.conversation_event_store();
    let run_id = "run-active-001";
    let conv_id = Uuid::new_v4();

    // Create an orphaned run.
    event_store
        .append_event(
            run_id,
            &conv_id.to_string(),
            0,
            "run_started",
            &serde_json::json!({"run_id": run_id}),
        )
        .await
        .unwrap();

    // Backdate.
    sqlx::query(
        "UPDATE conversation_events SET created_at = datetime('now', '-10 minutes') \
         WHERE run_id = ?1",
    )
    .bind(run_id)
    .execute(&storage.pool)
    .await
    .unwrap();

    // Publish an active bus message for the same conversation.
    let bus = storage.message_bus();
    use assistant_core::PublishRequest;
    bus.publish(
        PublishRequest::new(topic::TURN_REQUEST, serde_json::json!({"prompt": "retry"}))
            .with_conversation_id(conv_id),
    )
    .await
    .unwrap();

    reap_stale_and_recover(&storage, &bus).await;

    // The run should NOT be closed — bus message is still active.
    assert!(
        !event_store.is_run_complete(run_id).await.unwrap(),
        "run with active bus message should not be closed"
    );
}

#[tokio::test]
async fn reap_stale_skips_fresh_runs() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (_orch, storage) = build(&server.uri()).await;

    let event_store = storage.conversation_event_store();
    let run_id = "run-fresh-001";
    let conv_id = "conv-fresh-001";

    // Create a run that just started (no backdating).
    event_store
        .append_event(
            run_id,
            conv_id,
            0,
            "run_started",
            &serde_json::json!({"run_id": run_id}),
        )
        .await
        .unwrap();

    let bus = storage.message_bus();
    reap_stale_and_recover(&storage, &bus).await;

    // Fresh run should not be touched.
    assert!(
        !event_store.is_run_complete(run_id).await.unwrap(),
        "fresh run should not be closed"
    );
}

#[tokio::test]
async fn reap_stale_recovers_despite_non_turn_request_bus_message() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (_orch, storage) = build(&server.uri()).await;

    let event_store = storage.conversation_event_store();
    let run_id = "run-nonturn-001";
    let conv_id = Uuid::new_v4();

    // Create an orphaned run.
    event_store
        .append_event(
            run_id,
            &conv_id.to_string(),
            0,
            "run_started",
            &serde_json::json!({"run_id": run_id}),
        )
        .await
        .unwrap();

    // Backdate.
    sqlx::query(
        "UPDATE conversation_events SET created_at = datetime('now', '-10 minutes') \
         WHERE run_id = ?1",
    )
    .bind(run_id)
    .execute(&storage.pool)
    .await
    .unwrap();

    // Publish a non-turn.request bus message on the same conversation.
    // This should NOT block recovery — only turn.request matters.
    let bus = storage.message_bus();
    use assistant_core::PublishRequest;
    bus.publish(
        PublishRequest::new(
            topic::SCHEDULE_TRIGGER,
            serde_json::json!({"task_name": "irrelevant"}),
        )
        .with_conversation_id(conv_id),
    )
    .await
    .unwrap();

    reap_stale_and_recover(&storage, &bus).await;

    // The orphan should be recovered despite the schedule.trigger message.
    assert!(
        event_store.is_run_complete(run_id).await.unwrap(),
        "non-turn.request bus message should not block orphan recovery"
    );
}

// -- Turn interactions (ADR-0017) --

/// Start a run in the event log and age it past the stale threshold.
async fn seed_orphan(storage: &assistant_storage::StorageLayer, run_id: &str, conv_id: &str) {
    storage
        .conversation_event_store()
        .append_event(
            run_id,
            conv_id,
            0,
            "run_started",
            &serde_json::json!({"run_id": run_id}),
        )
        .await
        .unwrap();
    sqlx::query(
        "UPDATE conversation_events SET created_at = datetime('now', '-10 minutes') \
         WHERE run_id = ?1",
    )
    .bind(run_id)
    .execute(&storage.pool)
    .await
    .unwrap();
}

/// A `pending` approval of `run_id` whose waiter last reported ten minutes ago.
async fn seed_dead_interaction(
    storage: &assistant_storage::StorageLayer,
    run_id: &str,
    conv_id: &str,
) -> String {
    use assistant_storage::{InteractionStore as _, NewInteraction};
    let id = Uuid::new_v4().to_string();
    storage
        .interaction_store()
        .create(NewInteraction {
            id: id.clone(),
            conversation_id: conv_id.to_string(),
            run_id: run_id.to_string(),
            user_id: "alice".into(),
            kind: "tool_approval".into(),
            tool_call_id: None,
            request: serde_json::json!({"tool_name": "bash"}),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
        })
        .await
        .unwrap();
    sqlx::query("UPDATE interactions SET waiter_seen_at = ?1 WHERE id = ?2")
        .bind(
            (chrono::Utc::now() - chrono::Duration::minutes(10))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        )
        .bind(&id)
        .execute(&storage.pool)
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn reap_expires_dead_interactions_even_when_the_run_has_an_active_bus_message() {
    use assistant_storage::{InteractionStatus, InteractionStore as _};
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (_orch, storage) = build(&server.uri()).await;
    let (run_id, conv_id) = ("run-dead-ia-001", Uuid::new_v4());
    seed_orphan(&storage, run_id, &conv_id.to_string()).await;
    let id = seed_dead_interaction(&storage, run_id, &conv_id.to_string()).await;
    let bus = storage.message_bus();
    use assistant_core::PublishRequest;
    bus.publish(
        PublishRequest::new(topic::TURN_REQUEST, serde_json::json!({"prompt": "retry"}))
            .with_conversation_id(conv_id),
    )
    .await
    .unwrap();

    reap_stale_and_recover(&storage, &bus).await;

    let row = storage.interaction_store().get(&id).await.unwrap().unwrap();
    assert_eq!(row.status, InteractionStatus::Expired);
    assert!(
        !storage
            .conversation_event_store()
            .is_run_complete(run_id)
            .await
            .unwrap(),
        "the run may still recover, so it gets no terminal"
    );
}

#[tokio::test]
async fn reap_logs_each_unemitted_resolution_before_the_synthetic_terminal() {
    use assistant_storage::{InteractionStatus, InteractionStore as _};
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (_orch, storage) = build(&server.uri()).await;
    let (run_id, conv_id) = ("run-dead-ia-002", Uuid::new_v4().to_string());
    seed_orphan(&storage, run_id, &conv_id).await;
    let dead = seed_dead_interaction(&storage, run_id, &conv_id).await;
    let answered = seed_dead_interaction(&storage, run_id, &conv_id).await;
    let response = serde_json::json!({"decision": "allow_once"});
    storage
        .interaction_store()
        .resolve_if_pending(
            &answered,
            InteractionStatus::Resolved,
            Some(&response),
            None,
        )
        .await
        .unwrap();
    let bus = storage.message_bus();

    reap_stale_and_recover(&storage, &bus).await;

    let events = storage
        .conversation_event_store()
        .list_events_since(run_id, 0)
        .await
        .unwrap();
    let shape: Vec<(i64, &str)> = events
        .iter()
        .map(|e| (e.sequence, e.event_type.as_str()))
        .collect();
    assert_eq!(
        shape,
        [
            (0, "run_started"),
            (1, "interaction_resolved"),
            (2, "interaction_resolved"),
            (3, "agent_error"),
        ]
    );
    assert_eq!(events[1].payload["interaction_id"], dead);
    assert_eq!(events[1].payload["outcome"], "expired");
    assert_eq!(events[2].payload["interaction_id"], answered);
    assert_eq!(events[2].payload["outcome"], "resolved");
    assert_eq!(events[2].payload["response"], response);
    for id in [&dead, &answered] {
        let row = storage.interaction_store().get(id).await.unwrap().unwrap();
        assert!(row.resolution_emitted, "{id} is marked emitted");
    }
}
