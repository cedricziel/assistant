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

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use assistant_storage::{InMemoryWorkflowStore, WorkflowEdge, WorkflowExecutionLimits};

    use super::*;

    fn node(id: &str, kind: WorkflowNodeKind, config: Value) -> WorkflowNode {
        WorkflowNode {
            id: id.to_string(),
            kind,
            config,
        }
    }

    fn edge(from: &str, to: &str) -> WorkflowEdge {
        WorkflowEdge {
            from: from.to_string(),
            to: to.to_string(),
            on: None,
            condition: None,
        }
    }

    fn valid_graph() -> WorkflowGraph {
        WorkflowGraph {
            version: 1,
            nodes: vec![
                node("t", WorkflowNodeKind::Trigger, json!({"type": "manual"})),
                node(
                    "a",
                    WorkflowNodeKind::Action,
                    json!({"type": "http_request", "url": "https://example.com"}),
                ),
            ],
            edges: vec![edge("t", "a")],
            execution: WorkflowExecutionLimits {
                max_steps: 10,
                max_visits_per_node: 3,
            },
        }
    }

    /// Run `graph` (which may be invalid for the store) against a run that was
    /// created from a valid workflow, returning the outcome and recorded node ids.
    async fn traverse(graph: &WorkflowGraph) -> (RunOutcome, Vec<String>) {
        let store = InMemoryWorkflowStore::new();
        let workflow_id = store.create("wf", "", &valid_graph(), true).await.unwrap();
        let run_id = store
            .create_run(
                workflow_id,
                assistant_storage::WorkflowTriggerKind::Manual,
                &json!({}),
            )
            .await
            .unwrap();
        let run = store
            .list_runs(workflow_id, 1)
            .await
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let outcome = traverse_graph(&run, graph, &store, &[]).await.unwrap();
        let steps = store.list_run_steps(run_id, 100).await.unwrap();
        (outcome, steps.into_iter().map(|s| s.node_id).collect())
    }

    #[tokio::test]
    async fn graph_without_trigger_completes_without_executing_anything() {
        let mut graph = valid_graph();
        graph.nodes.remove(0);
        graph.edges.clear();

        let (outcome, visited) = traverse(&graph).await;

        assert!(matches!(outcome, RunOutcome::Completed));
        assert!(visited.is_empty(), "visited: {visited:?}");
    }

    #[tokio::test]
    async fn edge_to_missing_node_is_ignored() {
        let mut graph = valid_graph();
        graph.edges.insert(0, edge("t", "ghost"));

        let (outcome, visited) = traverse(&graph).await;

        assert!(matches!(outcome, RunOutcome::Completed));
        assert_eq!(visited, ["t", "a", "a"]);
    }

    fn payload_key_condition(config: Value) -> WorkflowNode {
        node("c", WorkflowNodeKind::Condition, config)
    }

    #[test]
    fn payload_key_condition_without_key_is_false() {
        let cond = payload_key_condition(json!({"type": "has_trigger_payload_key"}));

        assert_eq!(evaluate_condition(&cond, &json!({"": 1})), "false");
    }

    #[test]
    fn payload_key_condition_with_empty_key_is_false() {
        let cond = payload_key_condition(json!({"type": "has_trigger_payload_key", "key": ""}));

        assert_eq!(evaluate_condition(&cond, &json!({"": 1})), "false");
    }

    #[test]
    fn condition_without_type_is_true() {
        let cond = payload_key_condition(json!({}));

        assert_eq!(evaluate_condition(&cond, &json!({})), "true");
    }

    #[test]
    fn condition_with_unknown_type_is_true() {
        let cond = payload_key_condition(json!({"type": "mystery"}));

        assert_eq!(evaluate_condition(&cond, &json!({})), "true");
    }

    #[test]
    fn unlabelled_edge_matches_any_outcome() {
        assert!(edge_matches_label(None, "failure"));
    }

    #[test]
    fn labelled_edge_only_matches_its_own_outcome() {
        assert!(!edge_matches_label(Some("success"), "failure"));
    }

    fn run_with(trigger_type: &str, payload: Value) -> WorkflowRunRecord {
        WorkflowRunRecord {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            agent_id: "default".to_string(),
            trigger_type: trigger_type.to_string(),
            trigger_payload: payload,
            status: "running".to_string(),
            started_at: chrono::Utc::now(),
            finished_at: None,
            error_message: None,
        }
    }

    #[test]
    fn untyped_trigger_node_counts_as_manual() {
        let trigger = node("t", WorkflowNodeKind::Trigger, json!({}));

        assert!(run_starts_from_trigger(
            &run_with("manual", json!({})),
            &trigger
        ));
        assert!(!run_starts_from_trigger(
            &run_with("webhook", json!({})),
            &trigger
        ));
    }

    #[test]
    fn unrecognised_run_trigger_type_starts_from_every_trigger() {
        let trigger = node("t", WorkflowNodeKind::Trigger, json!({"type": "webhook"}));

        assert!(run_starts_from_trigger(
            &run_with("custom", json!({})),
            &trigger
        ));
    }
}
