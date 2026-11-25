//! Graph traversal: node execution, edge routing and condition evaluation.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use anyhow::Result;
use assistant_storage::{
    WorkflowGraph, WorkflowNode, WorkflowNodeKind, WorkflowRunRecord, WorkflowStore,
};
use serde_json::Value;

use crate::action::{WorkflowActionExecutor, WorkflowActionInput};

#[derive(Debug)]
pub(crate) enum RunOutcome {
    Completed,
    MaxStepsExceeded { max_steps: u32 },
    MaxVisitsExceeded { node_id: String, max_visits: u32 },
}

pub(crate) async fn traverse_graph(
    run: &WorkflowRunRecord,
    graph: &WorkflowGraph,
    store: &dyn WorkflowStore,
    action_executors: &[Arc<dyn WorkflowActionExecutor>],
) -> Result<RunOutcome> {
    let mut nodes_by_id: HashMap<&str, &WorkflowNode> = HashMap::new();
    for node in &graph.nodes {
        nodes_by_id.insert(node.id.as_str(), node);
    }

    let mut queue: VecDeque<&str> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == WorkflowNodeKind::Trigger)
        .filter(|node| run_starts_from_trigger(run, node))
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
                None,
            )
            .await?;

        let outcome_label = execute_node(run, node, store, step_index, action_executors).await?;

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

fn find_action_executor<'a>(
    action_type: &str,
    action_executors: &'a [Arc<dyn WorkflowActionExecutor>],
) -> Option<&'a Arc<dyn WorkflowActionExecutor>> {
    action_executors
        .iter()
        .find(|executor| executor.action_type() == action_type)
}

fn run_starts_from_trigger(run: &WorkflowRunRecord, trigger_node: &WorkflowNode) -> bool {
    if let Some(target_node) = run
        .trigger_payload
        .get("trigger_node_id")
        .and_then(|v| v.as_str())
    {
        return target_node == trigger_node.id;
    }

    let trigger_type = trigger_node
        .config
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("manual");
    match run.trigger_type.as_str() {
        "manual" => trigger_type == "manual",
        "webhook" => trigger_type == "webhook",
        "schedule" => trigger_type == "schedule",
        "event" => trigger_type == "event",
        _ => true,
    }
}

async fn execute_node(
    run: &WorkflowRunRecord,
    node: &WorkflowNode,
    store: &dyn WorkflowStore,
    step_index: i64,
    action_executors: &[Arc<dyn WorkflowActionExecutor>],
) -> Result<String> {
    // Skip disabled nodes — record a skipped step and return the primary
    // outcome so downstream nodes still execute.
    if node.config.get("disabled").and_then(|v| v.as_bool()) == Some(true) {
        let outcome = match node.kind {
            WorkflowNodeKind::Trigger => "trigger",
            WorkflowNodeKind::Action => "success",
            WorkflowNodeKind::Condition => "true",
        };
        store
            .append_run_step(
                run.id,
                step_index,
                node.id.as_str(),
                "skipped",
                Some("node disabled"),
                None,
            )
            .await?;
        return Ok(outcome.to_string());
    }

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
                    None,
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
            let Some(executor) = find_action_executor(action_type, action_executors) else {
                let note = format!("unsupported action type '{action_type}'");
                store
                    .append_run_step(
                        run.id,
                        step_index,
                        node.id.as_str(),
                        "action",
                        Some(&note),
                        None,
                    )
                    .await?;
                return Ok("failure".to_string());
            };

            let input = WorkflowActionInput {
                run_id: run.id,
                workflow_id: run.workflow_id,
                node_id: node.id.clone(),
                config: node.config.clone(),
                trigger_payload: run.trigger_payload.clone(),
            };

            match executor.execute(input).await {
                Ok(result) => {
                    store
                        .append_run_step(
                            run.id,
                            step_index,
                            node.id.as_str(),
                            "action",
                            result.note.as_deref(),
                            result.output.as_ref(),
                        )
                        .await?;
                    Ok(result.outcome_label)
                }
                Err(err) => {
                    let note = format!("action execution failure: {err}");
                    store
                        .append_run_step(
                            run.id,
                            step_index,
                            node.id.as_str(),
                            "action",
                            Some(&note),
                            None,
                        )
                        .await?;
                    Ok("failure".to_string())
                }
            }
        }
    }
}

fn evaluate_condition(node: &WorkflowNode, trigger_payload: &Value) -> String {
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
