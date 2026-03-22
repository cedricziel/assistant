//! Background workflow run processor.
//!
//! This worker consumes `workflow_runs` records in `running` state and executes
//! graph traversal with loop guardrails. It currently supports `assistant_turn`
//! action nodes through the orchestrator while keeping run/step telemetry and
//! deterministic transition routing.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::Interface;
use assistant_storage::{
    StorageLayer, WorkflowGraph, WorkflowNode, WorkflowNodeKind, WorkflowRunRecord, WorkflowStore,
};
use tracing::{error, info, warn};

use crate::Orchestrator;

/// Spawn the workflow runner loop.
pub fn spawn_workflow_runner(
    storage: Arc<StorageLayer>,
    orchestrator: Option<Arc<Orchestrator>>,
    poll_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(?poll_interval, "Workflow runner started");
        loop {
            tokio::time::sleep(poll_interval).await;
            if let Err(err) = process_pending_runs(&storage, orchestrator.as_deref()).await {
                error!(error = %err, "Workflow runner iteration failed");
            }
        }
    })
}

async fn process_pending_runs(
    storage: &StorageLayer,
    orchestrator: Option<&Orchestrator>,
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

        let outcome = traverse_graph(&run, &workflow.graph, &store, orchestrator).await?;
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
    run: &WorkflowRunRecord,
    graph: &WorkflowGraph,
    store: &WorkflowStore,
    orchestrator: Option<&Orchestrator>,
) -> Result<RunOutcome> {
    let mut nodes_by_id: HashMap<&str, &WorkflowNode> = HashMap::new();
    for node in &graph.nodes {
        nodes_by_id.insert(node.id.as_str(), node);
    }

    let mut queue: VecDeque<&str> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == WorkflowNodeKind::Trigger)
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
                run.id,
                step_index,
                node.id.as_str(),
                node_kind_label(node.kind.clone()),
                Some("node started"),
            )
            .await?;

        let outcome_label = execute_node(run, node, store, step_index, orchestrator).await?;

        for edge in graph.edges.iter().filter(|edge| edge.from == node.id) {
            if edge_matches_label(edge.on.as_deref(), &outcome_label) {
                queue.push_back(edge.to.as_str());
            }
        }
    }

    Ok(RunOutcome::Completed)
}

fn node_kind_label(kind: WorkflowNodeKind) -> &'static str {
    match kind {
        WorkflowNodeKind::Trigger => "trigger",
        WorkflowNodeKind::Action => "action",
        WorkflowNodeKind::Condition => "condition",
    }
}

fn edge_matches_label(edge_label: Option<&str>, outcome_label: &str) -> bool {
    match edge_label {
        Some(label) => label.eq_ignore_ascii_case(outcome_label),
        None => true,
    }
}

async fn execute_node(
    run: &WorkflowRunRecord,
    node: &WorkflowNode,
    store: &WorkflowStore,
    step_index: i64,
    orchestrator: Option<&Orchestrator>,
) -> Result<String> {
    match node.kind {
        WorkflowNodeKind::Trigger => Ok("trigger".to_string()),
        WorkflowNodeKind::Condition => {
            let label = evaluate_condition(node, &run.trigger_payload);
            store
                .append_run_step(
                    run.id,
                    step_index,
                    node.id.as_str(),
                    "condition",
                    Some(&format!("condition -> {label}")),
                )
                .await?;
            Ok(label)
        }
        WorkflowNodeKind::Action => {
            let action_type = node
                .config
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            match action_type {
                "assistant_turn" => {
                    let Some(orch) = orchestrator else {
                        store
                            .append_run_step(
                                run.id,
                                step_index,
                                node.id.as_str(),
                                "action",
                                Some("assistant_turn skipped: orchestrator unavailable"),
                            )
                            .await?;
                        return Ok("failure".to_string());
                    };

                    let prompt = node
                        .config
                        .get("prompt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Continue workflow action");
                    match orch
                        .submit_turn(prompt, run.id, Interface::Scheduler, None)
                        .await
                    {
                        Ok(result) => {
                            store
                                .append_run_step(
                                    run.id,
                                    step_index,
                                    node.id.as_str(),
                                    "action",
                                    Some(&format!(
                                        "assistant_turn success: {} chars",
                                        result.answer.chars().count()
                                    )),
                                )
                                .await?;
                            Ok("success".to_string())
                        }
                        Err(err) => {
                            store
                                .append_run_step(
                                    run.id,
                                    step_index,
                                    node.id.as_str(),
                                    "action",
                                    Some(&format!("assistant_turn failure: {err}")),
                                )
                                .await?;
                            Ok("failure".to_string())
                        }
                    }
                }
                _ => {
                    store
                        .append_run_step(
                            run.id,
                            step_index,
                            node.id.as_str(),
                            "action",
                            Some(&format!("unsupported action type '{action_type}'")),
                        )
                        .await?;
                    Ok("failure".to_string())
                }
            }
        }
    }
}

fn evaluate_condition(node: &WorkflowNode, trigger_payload: &serde_json::Value) -> String {
    let condition_type = node
        .config
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("always_true");
    match condition_type {
        "always_false" => "false".to_string(),
        "has_trigger_payload_key" => {
            let key = node
                .config
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if key.is_empty() {
                return "false".to_string();
            }
            if trigger_payload.get(key).is_some() {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        _ => "true".to_string(),
    }
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

        process_pending_runs(&storage, None)
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
