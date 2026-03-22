//! Background workflow run processor.
//!
//! This worker consumes `workflow_runs` records in `running` state and executes
//! graph traversal with loop guardrails. Action side effects are intentionally
//! deferred to the next phase; this module focuses on deterministic traversal,
//! run status transitions, and step telemetry.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_storage::{StorageLayer, WorkflowGraph, WorkflowNode, WorkflowStore};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Spawn the workflow runner loop.
pub fn spawn_workflow_runner(
    storage: Arc<StorageLayer>,
    poll_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(?poll_interval, "Workflow runner started");
        loop {
            tokio::time::sleep(poll_interval).await;
            if let Err(err) = process_pending_runs(&storage).await {
                error!(error = %err, "Workflow runner iteration failed");
            }
        }
    })
}

async fn process_pending_runs(storage: &StorageLayer) -> Result<()> {
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

        let outcome = traverse_graph(run.id, &workflow.graph, &store).await?;
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

#[derive(Debug)]
enum RunOutcome {
    Completed,
    MaxStepsExceeded { max_steps: u32 },
    MaxVisitsExceeded { node_id: String, max_visits: u32 },
}

async fn traverse_graph(
    run_id: Uuid,
    graph: &WorkflowGraph,
    store: &WorkflowStore,
) -> Result<RunOutcome> {
    let mut nodes_by_id: HashMap<&str, &WorkflowNode> = HashMap::new();
    for node in &graph.nodes {
        nodes_by_id.insert(node.id.as_str(), node);
    }

    let mut queue: VecDeque<&str> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == assistant_storage::WorkflowNodeKind::Trigger)
        .map(|node| node.id.as_str())
        .collect();

    let mut visits: HashMap<&str, u32> = HashMap::new();
    let mut step_index: i64 = 0;

    while let Some(node_id) = queue.pop_front() {
        step_index += 1;
        if step_index as u32 > graph.execution.max_steps {
            return Ok(RunOutcome::MaxStepsExceeded {
                max_steps: graph.execution.max_steps,
            });
        }

        let visit_count = visits.entry(node_id).or_insert(0);
        *visit_count += 1;
        if *visit_count > graph.execution.max_visits_per_node {
            return Ok(RunOutcome::MaxVisitsExceeded {
                node_id: node_id.to_string(),
                max_visits: graph.execution.max_visits_per_node,
            });
        }

        let Some(node) = nodes_by_id.get(node_id) else {
            continue;
        };

        store
            .append_run_step(
                run_id,
                step_index,
                node.id.as_str(),
                match node.kind {
                    assistant_storage::WorkflowNodeKind::Trigger => "trigger",
                    assistant_storage::WorkflowNodeKind::Action => "action",
                    assistant_storage::WorkflowNodeKind::Condition => "condition",
                },
                Some("graph traversal"),
            )
            .await?;

        for edge in graph.edges.iter().filter(|edge| edge.from == node.id) {
            queue.push_back(edge.to.as_str());
        }
    }

    Ok(RunOutcome::Completed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_storage::{
        WorkflowEdge, WorkflowExecutionLimits, WorkflowNode, WorkflowNodeKind,
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
                    config: serde_json::json!({"type": "assistant_turn"}),
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
                assistant_storage::WorkflowTriggerKind::Manual,
                &serde_json::json!({}),
            )
            .await
            .expect("create run");

        process_pending_runs(&storage)
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
}
