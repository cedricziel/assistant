use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::{bus_messages, MessageBus, PublishRequest};
use assistant_storage::{
    StorageLayer, WorkflowEdge, WorkflowExecutionLimits, WorkflowGraph, WorkflowNode,
    WorkflowNodeKind, WorkflowTriggerKind,
};
use assistant_workflow::{
    spawn_event_trigger_adapter, spawn_schedule_trigger_adapter, spawn_workflow_runner,
    WorkflowActionExecutor, WorkflowActionInput, WorkflowActionResult,
};
use async_trait::async_trait;

struct MockAssistantTurnExecutor;

#[async_trait]
impl WorkflowActionExecutor for MockAssistantTurnExecutor {
    fn action_type(&self) -> &'static str {
        "assistant_turn"
    }

    async fn execute(&self, input: WorkflowActionInput) -> Result<WorkflowActionResult> {
        Ok(WorkflowActionResult::success("mock assistant execution").with_output(
            serde_json::json!({
                "run_id": input.run_id,
                "node_id": input.node_id,
                "ok": true,
            }),
        ))
    }
}

fn graph_with_trigger(trigger_config: serde_json::Value) -> WorkflowGraph {
    WorkflowGraph {
        version: 1,
        nodes: vec![
            WorkflowNode {
                id: "trigger_1".to_string(),
                kind: WorkflowNodeKind::Trigger,
                config: trigger_config,
            },
            WorkflowNode {
                id: "action_1".to_string(),
                kind: WorkflowNodeKind::Action,
                config: serde_json::json!({"type": "assistant_turn", "prompt": "Run action"}),
            },
        ],
        edges: vec![WorkflowEdge {
            from: "trigger_1".to_string(),
            to: "action_1".to_string(),
            on: None,
            condition: None,
        }],
        execution: WorkflowExecutionLimits {
            max_steps: 20,
            max_visits_per_node: 5,
        },
    }
}

#[tokio::test]
async fn runner_executes_action_and_persists_step_output() {
    let storage = Arc::new(StorageLayer::new_in_memory().await.expect("in-memory storage"));
    let store = storage.workflow_store();
    let workflow_id = store
        .create(
            "runner-manual",
            "manual runner test",
            &graph_with_trigger(serde_json::json!({"type": "manual"}),),
            true,
        )
        .await
        .expect("create workflow");

    let run_id = store
        .create_run(workflow_id, WorkflowTriggerKind::Manual, &serde_json::json!({}))
        .await
        .expect("create run");

    let executors: Vec<Arc<dyn WorkflowActionExecutor>> = vec![Arc::new(MockAssistantTurnExecutor)];
    let handle = spawn_workflow_runner(storage.clone(), Duration::from_millis(20), executors);
    tokio::time::sleep(Duration::from_millis(150)).await;
    handle.abort();

    let run = store
        .get_run(workflow_id, run_id)
        .await
        .expect("load run")
        .expect("run exists");
    assert_eq!(run.status, "completed");

    let steps = store
        .list_run_steps(run_id, 20)
        .await
        .expect("load run steps");
    assert!(!steps.is_empty());
    let action_step_with_output = steps
        .iter()
        .find(|step| step.node_kind == "action" && step.output.is_some())
        .expect("action step with output exists");
    assert!(action_step_with_output.output.is_some());
}

#[tokio::test]
async fn schedule_adapter_creates_schedule_run() {
    let storage = Arc::new(StorageLayer::new_in_memory().await.expect("in-memory storage"));
    let store = storage.workflow_store();
    let workflow_id = store
        .create(
            "schedule-trigger",
            "schedule adapter test",
            &graph_with_trigger(serde_json::json!({"type": "schedule", "cron": "*/15 * * * *"})),
            true,
        )
        .await
        .expect("create workflow");

    let handle = spawn_schedule_trigger_adapter(storage.clone(), Duration::from_millis(20));
    tokio::time::sleep(Duration::from_millis(120)).await;
    handle.abort();

    let runs = store
        .list_runs(workflow_id, 10)
        .await
        .expect("list runs after schedule adapter");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].trigger_type, "schedule");
}

#[tokio::test]
async fn event_adapter_creates_run_from_done_bus_message() {
    let storage = Arc::new(StorageLayer::new_in_memory().await.expect("in-memory storage"));
    let store = storage.workflow_store();
    let workflow_id = store
        .create(
            "event-trigger",
            "event adapter test",
            &graph_with_trigger(serde_json::json!({"type": "event", "event": bus_messages::topic::TURN_RESULT})),
            true,
        )
        .await
        .expect("create workflow");

    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let handle =
        spawn_event_trigger_adapter(storage.clone(), bus.clone(), Duration::from_millis(20));

    tokio::time::sleep(Duration::from_millis(40)).await;
    let message_id = bus
        .publish(PublishRequest::new(
            bus_messages::topic::TURN_RESULT,
            serde_json::json!({"conversation_id": uuid::Uuid::new_v4(), "content": "ok", "turn": 1, "attachments": []}),
        ))
        .await
        .expect("publish turn.result message");

    let claimed = bus
        .claim(bus_messages::topic::TURN_RESULT, "workflow-event-test")
        .await
        .expect("claim published message")
        .expect("claimed message exists");
    assert_eq!(claimed.id, message_id);
    bus.ack(claimed.id).await.expect("ack message as done");

    tokio::time::sleep(Duration::from_millis(150)).await;
    handle.abort();

    let runs = store
        .list_runs(workflow_id, 10)
        .await
        .expect("list runs after event adapter");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].trigger_type, "event");
}
