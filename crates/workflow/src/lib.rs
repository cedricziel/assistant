//! Workflow run engine and extensible action execution.

use std::collections::{HashMap, HashSet, VecDeque};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::{MessageBus, MessageStatus, bus_messages};
use assistant_storage::{
    StorageLayer, WorkflowGraph, WorkflowNode, WorkflowNodeKind, WorkflowRunRecord, WorkflowStore,
    WorkflowTriggerKind,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde_json::Value;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Input passed to action executors.
#[derive(Debug, Clone)]
pub struct WorkflowActionInput {
    pub run_id: Uuid,
    pub workflow_id: Uuid,
    pub node_id: String,
    pub config: Value,
    pub trigger_payload: Value,
}

/// Action execution output consumed by graph traversal.
#[derive(Debug, Clone)]
pub struct WorkflowActionResult {
    pub outcome_label: String,
    pub note: Option<String>,
    pub output: Option<Value>,
}

impl WorkflowActionResult {
    /// Success-style result.
    pub fn success(note: impl Into<String>) -> Self {
        Self {
            outcome_label: "success".to_string(),
            note: Some(note.into()),
            output: None,
        }
    }

    /// Failure-style result.
    pub fn failure(note: impl Into<String>) -> Self {
        Self {
            outcome_label: "failure".to_string(),
            note: Some(note.into()),
            output: None,
        }
    }

    /// Attach structured step output.
    pub fn with_output(mut self, output: Value) -> Self {
        self.output = Some(output);
        self
    }
}

/// Executor plugin interface for workflow action node types.
#[async_trait]
pub trait WorkflowActionExecutor: Send + Sync {
    /// Action type this executor handles, e.g. `assistant_turn`, `http_request`.
    fn action_type(&self) -> &'static str;

    /// Executes one action node.
    async fn execute(&self, input: WorkflowActionInput) -> Result<WorkflowActionResult>;
}

/// Client abstraction to submit assistant turns from workflows.
#[async_trait]
pub trait AssistantTurnClient: Send + Sync {
    /// Submit a prompt and return the assistant answer text.
    async fn submit_turn(&self, prompt: &str, conversation_id: Uuid) -> Result<String>;
}

/// Built-in action executor for `assistant_turn` nodes.
pub struct AssistantTurnActionExecutor {
    client: Arc<dyn AssistantTurnClient>,
}

impl AssistantTurnActionExecutor {
    /// Construct from any [`AssistantTurnClient`] implementation.
    pub fn new(client: Arc<dyn AssistantTurnClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl WorkflowActionExecutor for AssistantTurnActionExecutor {
    fn action_type(&self) -> &'static str {
        "assistant_turn"
    }

    async fn execute(&self, input: WorkflowActionInput) -> Result<WorkflowActionResult> {
        let prompt = input
            .config
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("Continue workflow action");

        let answer = self.client.submit_turn(prompt, input.run_id).await?;
        Ok(WorkflowActionResult::success(format!(
            "assistant_turn success: {} chars",
            answer.chars().count()
        )))
    }
}

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

/// Spawn schedule trigger adapter that creates runs for due schedule nodes.
pub fn spawn_schedule_trigger_adapter(
    storage: Arc<StorageLayer>,
    poll_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(?poll_interval, "Workflow schedule adapter started");
        loop {
            tokio::time::sleep(poll_interval).await;
            if let Err(err) = process_schedule_triggers(&storage).await {
                error!(error = %err, "Workflow schedule adapter iteration failed");
            }
        }
    })
}

/// Spawn event trigger adapter from completed bus events.
pub fn spawn_event_trigger_adapter(
    storage: Arc<StorageLayer>,
    bus: Arc<dyn MessageBus>,
    poll_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(?poll_interval, "Workflow event adapter started");
        let mut seen: HashSet<Uuid> = HashSet::new();
        let mut seen_order: VecDeque<Uuid> = VecDeque::new();
        let started_at = SystemClock.now();
        let mut warned_done_filter_fallback = false;

        loop {
            tokio::time::sleep(poll_interval).await;
            if let Err(err) = process_event_triggers(
                &storage,
                bus.as_ref(),
                started_at,
                &mut seen,
                &mut seen_order,
                &mut warned_done_filter_fallback,
            )
            .await
            {
                error!(error = %err, "Workflow event adapter iteration failed");
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

async fn process_schedule_triggers(storage: &StorageLayer) -> Result<()> {
    let store = storage.workflow_store();
    let workflows = store.list().await?;
    let now = SystemClock.now();

    for workflow in workflows.into_iter().filter(|w| w.active) {
        for node in workflow
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == WorkflowNodeKind::Trigger)
        {
            let trigger_type = node
                .config
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("manual");
            if trigger_type != "schedule" {
                continue;
            }

            let cron_expr = node
                .config
                .get("cron")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if cron_expr.is_empty() {
                warn!(workflow_id = %workflow.id, node_id = %node.id, "schedule trigger missing cron");
                continue;
            }

            if !schedule_trigger_due(&store, workflow.id, node.id.as_str(), cron_expr, now).await? {
                continue;
            }

            let payload = serde_json::json!({
                "trigger_node_id": node.id,
                "cron": cron_expr,
                "scheduled_at": now.to_rfc3339(),
            });
            let run_id = store
                .create_run(workflow.id, WorkflowTriggerKind::Schedule, &payload)
                .await?;
            info!(workflow_id = %workflow.id, node_id = %node.id, run_id = %run_id, "Scheduled trigger queued workflow run");
        }
    }

    Ok(())
}

async fn schedule_trigger_due(
    store: &WorkflowStore,
    workflow_id: Uuid,
    trigger_node_id: &str,
    cron_expr: &str,
    now: DateTime<Utc>,
) -> Result<bool> {
    let runs = store.list_runs(workflow_id, 200).await?;
    let latest_for_node = runs.into_iter().find(|run| {
        run.trigger_type == "schedule"
            && run
                .trigger_payload
                .get("trigger_node_id")
                .and_then(|v| v.as_str())
                == Some(trigger_node_id)
    });

    let Some(schedule) = parse_schedule(cron_expr) else {
        warn!(workflow_id = %workflow_id, trigger_node_id, cron_expr, "Invalid schedule cron expression");
        return Ok(false);
    };

    if let Some(last_run) = latest_for_node {
        let next = schedule.after(&last_run.started_at).next();
        return Ok(next.map(|t| t <= now).unwrap_or(false));
    }

    Ok(true)
}

async fn process_event_triggers(
    storage: &StorageLayer,
    bus: &dyn MessageBus,
    started_at: DateTime<Utc>,
    seen: &mut HashSet<Uuid>,
    seen_order: &mut VecDeque<Uuid>,
    warned_done_filter_fallback: &mut bool,
) -> Result<()> {
    let store = storage.workflow_store();
    let workflows = store.list().await?;

    for topic in [
        bus_messages::topic::TURN_RESULT,
        bus_messages::topic::TOOL_RESULT,
        bus_messages::topic::AGENT_REPORT,
    ] {
        let mut messages = bus.list(topic, Some(MessageStatus::Done), 50).await?;
        if messages.is_empty() {
            let fallback = bus.list(topic, None, 50).await?;
            if !fallback.is_empty() {
                if !*warned_done_filter_fallback {
                    warn!(
                        "MessageBus did not return done-filtered events; using unfiltered fallback for workflow event triggers"
                    );
                    *warned_done_filter_fallback = true;
                }
                messages = fallback;
            }
        }
        for msg in messages {
            if msg.created_at < started_at || seen.contains(&msg.id) {
                continue;
            }

            seen.insert(msg.id);
            seen_order.push_back(msg.id);
            while seen_order.len() > 4096 {
                if let Some(old) = seen_order.pop_front() {
                    seen.remove(&old);
                }
            }

            for workflow in workflows.iter().filter(|w| w.active) {
                for node in workflow
                    .graph
                    .nodes
                    .iter()
                    .filter(|n| n.kind == WorkflowNodeKind::Trigger)
                {
                    let trigger_type = node
                        .config
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("manual");
                    if trigger_type != "event" {
                        continue;
                    }

                    let expected_topic = node
                        .config
                        .get("event")
                        .or_else(|| node.config.get("topic"))
                        .and_then(|v| v.as_str());
                    if let Some(expected_topic) = expected_topic
                        && expected_topic != msg.topic
                    {
                        continue;
                    }

                    let payload = serde_json::json!({
                        "trigger_node_id": node.id,
                        "event_topic": msg.topic,
                        "message_id": msg.id,
                        "conversation_id": msg.conversation_id,
                        "payload": msg.payload,
                    });
                    let run_id = store
                        .create_run(workflow.id, WorkflowTriggerKind::Event, &payload)
                        .await?;
                    info!(workflow_id = %workflow.id, node_id = %node.id, run_id = %run_id, event_topic = %topic, "Event trigger queued workflow run");
                }
            }
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
    store: &WorkflowStore,
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

fn parse_schedule(expr: &str) -> Option<Schedule> {
    Schedule::from_str(expr)
        .or_else(|_| Schedule::from_str(&format!("0 {expr}")))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_storage::{
        WorkflowEdge, WorkflowExecutionLimits, WorkflowNode, WorkflowNodeKind, WorkflowTriggerKind,
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
