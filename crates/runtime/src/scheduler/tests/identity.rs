//! Persona-owner identity threading: scheduled tasks must carry the
//! persona's `owner_user_id` on the resulting `TurnRequest`.

use assistant_storage::PersonaStore as _;
use assistant_storage::ScheduledTaskStore as _;
use std::sync::Arc;

use assistant_core::{MessageBus, bus_messages, topic};
use chrono::{Duration, Utc};
use wiremock::MockServer;

use super::{build, mount_answer};
use crate::orchestrator::Orchestrator;
use crate::scheduler::run_due_tasks;

#[tokio::test]
async fn scheduled_task_carries_persona_owner_identity() {
    let server = MockServer::start().await;
    mount_answer(&server, "done").await;
    let (orch, storage) = build(&server.uri()).await;

    // Create a persona with an owner and set it as the orchestrator's agent.
    let persona_store = storage.persona_store();
    persona_store
        .create_owned("test-persona", "Test Persona", "usr_alice")
        .await
        .unwrap();
    // Override the orchestrator's agent_id to match the persona.
    // Safety: we need interior mutability for this test — use unsafe to
    // mutate through the Arc since we hold the only reference.
    unsafe {
        let orch_mut = Arc::as_ptr(&orch) as *mut Orchestrator;
        (*orch_mut).agent_id = "test-persona".to_string();
    }

    let store = storage.scheduled_task_store_for_agent("test-persona");
    let past = Utc::now() - Duration::seconds(60);
    store
        .insert(
            "identity-task",
            "0 0 * * *",
            "probe identity",
            false,
            Some(past),
        )
        .await
        .unwrap();

    run_due_tasks(&storage, &orch).await.unwrap();

    let bus = storage.message_bus();
    let msg = bus
        .claim(topic::TURN_REQUEST, "test-consumer")
        .await
        .unwrap()
        .expect("turn.request should be on the bus");

    let turn_req: bus_messages::TurnRequest = serde_json::from_value(msg.payload).unwrap();

    assert_eq!(
        turn_req.user_id.as_deref(),
        Some("usr_alice"),
        "TurnRequest should carry the persona owner's user_id"
    );
}
