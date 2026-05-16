//! Tests for `run_heartbeat` — reads HEARTBEAT.md and publishes a TurnRequest.

use assistant_core::{MessageBus, bus_messages, topic};
use wiremock::MockServer;

use super::{build_with_agent_dir, mount_answer};
use crate::scheduler::SCHEDULER_USER_ID;
use crate::scheduler::run_heartbeat;

#[tokio::test]
async fn heartbeat_skipped_when_file_missing() {
    let server = MockServer::start().await;
    // agent dir is created but we deliberately don't write HEARTBEAT.md
    let (orch, storage, _agent_dir, _agent_id) = build_with_agent_dir(&server.uri()).await;

    run_heartbeat(&orch).await.unwrap();

    let msg = storage
        .message_bus()
        .claim(topic::TURN_REQUEST, "test-consumer")
        .await
        .unwrap();
    assert!(
        msg.is_none(),
        "no message should be published when HEARTBEAT.md is absent"
    );
}

#[tokio::test]
async fn heartbeat_skipped_when_file_empty_after_comment_strip() {
    let server = MockServer::start().await;
    let (orch, storage, agent_dir, _agent_id) = build_with_agent_dir(&server.uri()).await;

    std::fs::write(agent_dir.join("HEARTBEAT.md"), "<!-- just a comment -->").unwrap();

    run_heartbeat(&orch).await.unwrap();

    let msg = storage
        .message_bus()
        .claim(topic::TURN_REQUEST, "test-consumer")
        .await
        .unwrap();
    assert!(
        msg.is_none(),
        "comment-only HEARTBEAT.md must produce no message"
    );
}

#[tokio::test]
async fn heartbeat_publishes_turn_request_with_scheduler_user_id() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;

    let (orch, storage, agent_dir, _agent_id) = build_with_agent_dir(&server.uri()).await;

    std::fs::write(agent_dir.join("HEARTBEAT.md"), "Check system health.").unwrap();

    run_heartbeat(&orch).await.unwrap();

    let msg = storage
        .message_bus()
        .claim(topic::TURN_REQUEST, "test-consumer")
        .await
        .unwrap();
    assert!(
        msg.is_some(),
        "HEARTBEAT.md with content must publish a turn.request"
    );
    let msg = msg.unwrap();
    assert_eq!(
        msg.interface.as_deref(),
        Some("Scheduler"),
        "heartbeat must use Scheduler interface"
    );
    assert_eq!(
        msg.user_id.as_deref(),
        Some(SCHEDULER_USER_ID),
        "heartbeat must use SCHEDULER_USER_ID constant"
    );

    // Verify the prompt text was preserved
    let payload: bus_messages::TurnRequest = serde_json::from_value(msg.payload).unwrap();
    assert_eq!(payload.prompt.trim(), "Check system health.");
}
