//! Runner loop that drains runnable runs through the graph traversal.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_storage::{StorageLayer, WorkflowStore};
use tracing::{error, info, warn};

use crate::action::WorkflowActionExecutor;
use crate::traverse::{RunOutcome, traverse_graph};

/// Spawn the workflow runner loop.
pub fn spawn_workflow_runner(
    storage: Arc<StorageLayer>,
    poll_interval: Duration,
    action_executors: Vec<Arc<dyn WorkflowActionExecutor>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(?poll_interval, "Workflow runner started");
        loop {
            tokio::time::sleep(poll_interval).await;
            if let Err(err) = process_pending_runs(&storage, &action_executors).await {
                error!(error = %err, "Workflow runner iteration failed");
            }
        }
    })
}

async fn process_pending_runs(
    storage: &StorageLayer,
    action_executors: &[Arc<dyn WorkflowActionExecutor>],
) -> Result<()> {
    let store = storage.workflow_store();
    let runs = store.list_runnable_runs(25).await?;
    if runs.is_empty() {
        return Ok(());
    }

    for run in runs {
        let workflow = match store.workflow_for_run(run.id).await? {
            Some(workflow) => workflow,
            None => {
                warn!(run_id = %run.id, "Run references missing workflow; failing run");
                let _ = store
                    .finish_run(run.id, "failed", Some("workflow missing for run"))
                    .await;
                continue;
            }
        };

        let outcome = traverse_graph(&run, &workflow.graph, &store, action_executors).await?;
        let (status, error_message) = match outcome {
            RunOutcome::Completed => ("completed", None),
            RunOutcome::MaxStepsExceeded { max_steps } => (
                "max_steps_exceeded",
                Some(format!("execution exceeded max_steps={max_steps}")),
            ),
            RunOutcome::MaxVisitsExceeded {
                node_id,
                max_visits,
            } => (
                "max_visits_exceeded",
                Some(format!(
                    "node '{node_id}' exceeded max_visits_per_node={max_visits}"
                )),
            ),
        };

        let finished = store
            .finish_run(run.id, status, error_message.as_deref())
            .await?;
        if finished {
            info!(run_id = %run.id, workflow_id = %workflow.id, status, "Workflow run finished");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_storage::{
        WorkflowEdge, WorkflowExecutionLimits, WorkflowGraph, WorkflowNode, WorkflowNodeKind,
        WorkflowTriggerKind,
    };

    fn looping_graph() -> WorkflowGraph {
        WorkflowGraph {
            version: 1,
            nodes: vec![
                WorkflowNode {
                    id: "t1".to_string(),
                    kind: WorkflowNodeKind::Trigger,
                    config: serde_json::json!({"type": "manual"}),
                },
                WorkflowNode {
                    id: "a1".to_string(),
                    kind: WorkflowNodeKind::Action,
                    config: serde_json::json!({"type": "assistant_turn", "prompt": "Loop"}),
                },
            ],
            edges: vec![
                WorkflowEdge {
                    from: "t1".to_string(),
                    to: "a1".to_string(),
                    on: None,
                    condition: None,
                },
                WorkflowEdge {
                    from: "a1".to_string(),
                    to: "a1".to_string(),
                    on: None,
                    condition: None,
                },
            ],
            execution: WorkflowExecutionLimits {
                max_steps: 20,
                max_visits_per_node: 4,
            },
        }
    }

    #[tokio::test]
    async fn runner_marks_loop_exceeded_run() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .expect("in-memory storage");
        let store = storage.workflow_store();
        let workflow_id = store
            .create("loop", "loop", &looping_graph(), true)
            .await
            .expect("create workflow");
        let run_id = store
            .create_run(
                workflow_id,
                WorkflowTriggerKind::Manual,
                &serde_json::json!({}),
            )
            .await
            .expect("create run");

        process_pending_runs(&storage, &[])
            .await
            .expect("runner process");

        let runs = store.list_runs(workflow_id, 5).await.expect("list runs");
        let run = runs
            .into_iter()
            .find(|r| r.id == run_id)
            .expect("run present");
        assert_eq!(run.status, "max_visits_exceeded");
        let steps = store.list_run_steps(run_id, 100).await.expect("steps");
        assert!(!steps.is_empty());
    }

    /// A disabled action node should be skipped (recorded as "skipped") and
    /// still route downstream via its primary outcome ("success").
    #[tokio::test]
    async fn disabled_node_is_skipped_during_execution() {
        let graph = WorkflowGraph {
            version: 1,
            nodes: vec![
                WorkflowNode {
                    id: "t1".to_string(),
                    kind: WorkflowNodeKind::Trigger,
                    config: serde_json::json!({"type": "manual"}),
                },
                WorkflowNode {
                    id: "a1".to_string(),
                    kind: WorkflowNodeKind::Action,
                    config: serde_json::json!({"type": "http_request", "url": "https://example.com", "disabled": true}),
                },
                WorkflowNode {
                    id: "c1".to_string(),
                    kind: WorkflowNodeKind::Condition,
                    config: serde_json::json!({"type": "always_true"}),
                },
            ],
            edges: vec![
                WorkflowEdge {
                    from: "t1".to_string(),
                    to: "a1".to_string(),
                    on: Some("trigger".to_string()),
                    condition: None,
                },
                WorkflowEdge {
                    from: "a1".to_string(),
                    to: "c1".to_string(),
                    on: Some("success".to_string()),
                    condition: None,
                },
            ],
            execution: WorkflowExecutionLimits {
                max_steps: 50,
                max_visits_per_node: 3,
            },
        };

        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .expect("in-memory storage");
        let store = storage.workflow_store();
        let workflow_id = store
            .create("disabled-test", "", &graph, true)
            .await
            .expect("create workflow");
        let run_id = store
            .create_run(
                workflow_id,
                WorkflowTriggerKind::Manual,
                &serde_json::json!({}),
            )
            .await
            .expect("create run");

        process_pending_runs(&storage, &[])
            .await
            .expect("runner process");

        let runs = store.list_runs(workflow_id, 5).await.expect("list runs");
        let run = runs
            .into_iter()
            .find(|r| r.id == run_id)
            .expect("run present");
        assert_eq!(
            run.status, "completed",
            "run should complete even with a disabled node"
        );

        let steps = store.list_run_steps(run_id, 100).await.expect("steps");

        // Disabled action node should have a "skipped" step.
        let skipped = steps
            .iter()
            .find(|s| s.node_id == "a1" && s.node_kind == "skipped");
        assert!(
            skipped.is_some(),
            "disabled action node should produce a skipped step"
        );
        assert_eq!(
            skipped.unwrap().note.as_deref(),
            Some("node disabled"),
            "skipped step note should explain why"
        );

        // Downstream condition node should still have been reached.
        let condition_step = steps.iter().find(|s| s.node_id == "c1");
        assert!(
            condition_step.is_some(),
            "downstream condition should execute after disabled node"
        );
    }

    /// A disabled condition node should return "true" as its primary outcome.
    #[tokio::test]
    async fn disabled_condition_routes_via_true_branch() {
        let graph = WorkflowGraph {
            version: 1,
            nodes: vec![
                WorkflowNode {
                    id: "t1".to_string(),
                    kind: WorkflowNodeKind::Trigger,
                    config: serde_json::json!({"type": "manual"}),
                },
                WorkflowNode {
                    id: "c1".to_string(),
                    kind: WorkflowNodeKind::Condition,
                    config: serde_json::json!({"type": "always_false", "disabled": true}),
                },
                WorkflowNode {
                    id: "a_true".to_string(),
                    kind: WorkflowNodeKind::Action,
                    config: serde_json::json!({"type": "http_request", "url": "https://true.example.com"}),
                },
                WorkflowNode {
                    id: "a_false".to_string(),
                    kind: WorkflowNodeKind::Action,
                    config: serde_json::json!({"type": "http_request", "url": "https://false.example.com"}),
                },
            ],
            edges: vec![
                WorkflowEdge {
                    from: "t1".to_string(),
                    to: "c1".to_string(),
                    on: Some("trigger".to_string()),
                    condition: None,
                },
                WorkflowEdge {
                    from: "c1".to_string(),
                    to: "a_true".to_string(),
                    on: Some("true".to_string()),
                    condition: None,
                },
                WorkflowEdge {
                    from: "c1".to_string(),
                    to: "a_false".to_string(),
                    on: Some("false".to_string()),
                    condition: None,
                },
            ],
            execution: WorkflowExecutionLimits {
                max_steps: 50,
                max_visits_per_node: 3,
            },
        };

        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .expect("in-memory storage");
        let store = storage.workflow_store();
        let workflow_id = store
            .create("disabled-condition", "", &graph, true)
            .await
            .expect("create workflow");
        let run_id = store
            .create_run(
                workflow_id,
                WorkflowTriggerKind::Manual,
                &serde_json::json!({}),
            )
            .await
            .expect("create run");

        process_pending_runs(&storage, &[])
            .await
            .expect("runner process");

        let runs = store.list_runs(workflow_id, 5).await.expect("list runs");
        let run = runs
            .into_iter()
            .find(|r| r.id == run_id)
            .expect("run present");
        assert_eq!(run.status, "completed");

        let steps = store.list_run_steps(run_id, 100).await.expect("steps");

        // The condition was disabled (always_false) but should route via "true"
        // because disabled nodes use the primary outcome.
        let true_branch = steps.iter().any(|s| s.node_id == "a_true");
        let false_branch = steps.iter().any(|s| s.node_id == "a_false");
        assert!(
            true_branch,
            "disabled always_false condition should route via true branch"
        );
        assert!(
            !false_branch,
            "false branch should NOT be reached when condition is disabled"
        );
    }
}
