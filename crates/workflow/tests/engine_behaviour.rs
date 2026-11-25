//! Behaviour of the graph engine, observed through the public runner API and
//! the persisted run + step records.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, bail};
use assistant_storage::{
    StorageLayer, WorkflowEdge, WorkflowExecutionLimits, WorkflowGraph, WorkflowNode,
    WorkflowNodeKind, WorkflowRunRecord, WorkflowRunStepRecord, WorkflowStore as _,
    WorkflowTriggerKind,
};
use assistant_workflow::{
    WorkflowActionExecutor, WorkflowActionInput, WorkflowActionResult, spawn_workflow_runner,
};
use async_trait::async_trait;
use serde_json::{Value, json};

/// Executor that records each node it runs and replies with a canned result.
struct ScriptedExecutor {
    action_type: &'static str,
    calls: Arc<Mutex<Vec<String>>>,
    fail_nodes: Vec<&'static str>,
    error_nodes: Vec<&'static str>,
}

#[async_trait]
impl WorkflowActionExecutor for ScriptedExecutor {
    fn action_type(&self) -> &'static str {
        self.action_type
    }

    async fn execute(&self, input: WorkflowActionInput) -> Result<WorkflowActionResult> {
        self.calls.lock().unwrap().push(input.node_id.clone());
        if self.error_nodes.contains(&input.node_id.as_str()) {
            bail!("boom in {}", input.node_id);
        }
        if self.fail_nodes.contains(&input.node_id.as_str()) {
            return Ok(WorkflowActionResult::failure("scripted failure"));
        }
        Ok(WorkflowActionResult::success("ok").with_output(json!({"node": input.node_id})))
    }
}

fn node(id: &str, kind: WorkflowNodeKind, config: Value) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind,
        config,
    }
}

fn trigger(id: &str) -> WorkflowNode {
    node(id, WorkflowNodeKind::Trigger, json!({"type": "manual"}))
}

fn action(id: &str) -> WorkflowNode {
    node(
        id,
        WorkflowNodeKind::Action,
        json!({"type": "http_request", "url": "https://example.com"}),
    )
}

fn condition(id: &str, config: Value) -> WorkflowNode {
    node(id, WorkflowNodeKind::Condition, config)
}

fn edge(from: &str, to: &str, on: Option<&str>) -> WorkflowEdge {
    WorkflowEdge {
        from: from.to_string(),
        to: to.to_string(),
        on: on.map(str::to_string),
        condition: None,
    }
}

fn graph(nodes: Vec<WorkflowNode>, edges: Vec<WorkflowEdge>) -> WorkflowGraph {
    WorkflowGraph {
        version: 1,
        nodes,
        edges,
        execution: WorkflowExecutionLimits {
            max_steps: 50,
            max_visits_per_node: 5,
        },
    }
}

struct Outcome {
    run: WorkflowRunRecord,
    steps: Vec<WorkflowRunStepRecord>,
    calls: Vec<String>,
}

impl Outcome {
    fn notes_for(&self, node_id: &str) -> Vec<&str> {
        self.steps
            .iter()
            .filter(|s| s.node_id == node_id)
            .filter_map(|s| s.note.as_deref())
            .collect()
    }
}

async fn run_graph(graph: WorkflowGraph, payload: Value) -> Outcome {
    run_graph_with(graph, WorkflowTriggerKind::Manual, payload, vec![], vec![]).await
}

async fn run_graph_with(
    graph: WorkflowGraph,
    kind: WorkflowTriggerKind,
    payload: Value,
    fail_nodes: Vec<&'static str>,
    error_nodes: Vec<&'static str>,
) -> Outcome {
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let store = storage.workflow_store();
    let workflow_id = store.create("wf", "", &graph, true).await.unwrap();
    let run_id = store.create_run(workflow_id, kind, &payload).await.unwrap();

    let calls = Arc::new(Mutex::new(Vec::new()));
    let executors: Vec<Arc<dyn WorkflowActionExecutor>> = vec![Arc::new(ScriptedExecutor {
        action_type: "http_request",
        calls: calls.clone(),
        fail_nodes,
        error_nodes,
    })];
    let handle = spawn_workflow_runner(storage.clone(), Duration::from_millis(5), executors);

    let mut run = store.get_run(workflow_id, run_id).await.unwrap().unwrap();
    for _ in 0..400 {
        if run.finished_at.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
        run = store.get_run(workflow_id, run_id).await.unwrap().unwrap();
    }
    handle.abort();
    let _ = handle.await;

    let steps = store.list_run_steps(run_id, 500).await.unwrap();
    let calls = calls.lock().unwrap().clone();
    Outcome { run, steps, calls }
}

#[tokio::test]
async fn nodes_execute_in_edge_order() {
    let g = graph(
        vec![trigger("t"), action("a"), action("b"), action("c")],
        vec![
            edge("t", "a", None),
            edge("a", "b", None),
            edge("b", "c", None),
        ],
    );

    let out = run_graph(g, json!({})).await;

    assert_eq!(out.run.status, "completed");
    assert_eq!(out.calls, ["a", "b", "c"]);
}

#[tokio::test]
async fn each_node_visit_gets_the_next_step_index() {
    let g = graph(vec![trigger("t"), action("a")], vec![edge("t", "a", None)]);

    let out = run_graph(g, json!({})).await;

    let mut indexes: Vec<i64> = out.steps.iter().map(|s| s.step_index).collect();
    indexes.dedup();
    assert_eq!(indexes, [1, 2]);
}

#[tokio::test]
async fn fan_out_runs_every_matching_branch_breadth_first() {
    let g = graph(
        vec![
            trigger("t"),
            action("a"),
            action("b"),
            action("a2"),
            action("b2"),
        ],
        vec![
            edge("t", "a", None),
            edge("t", "b", None),
            edge("a", "a2", None),
            edge("b", "b2", None),
        ],
    );

    let out = run_graph(g, json!({})).await;

    assert_eq!(out.calls, ["a", "b", "a2", "b2"]);
}

#[tokio::test]
async fn failed_action_routes_only_along_failure_edge() {
    let g = graph(
        vec![
            trigger("t"),
            action("a"),
            action("on_ok"),
            action("on_fail"),
        ],
        vec![
            edge("t", "a", None),
            edge("a", "on_ok", Some("success")),
            edge("a", "on_fail", Some("failure")),
        ],
    );

    let out = run_graph_with(g, WorkflowTriggerKind::Manual, json!({}), vec!["a"], vec![]).await;

    assert_eq!(out.calls, ["a", "on_fail"]);
}

#[tokio::test]
async fn executor_error_is_recorded_and_routes_as_failure() {
    let g = graph(
        vec![
            trigger("t"),
            action("a"),
            action("on_ok"),
            action("on_fail"),
        ],
        vec![
            edge("t", "a", None),
            edge("a", "on_ok", Some("success")),
            edge("a", "on_fail", Some("failure")),
        ],
    );

    let out = run_graph_with(g, WorkflowTriggerKind::Manual, json!({}), vec![], vec!["a"]).await;

    assert_eq!(
        (
            out.run.status.as_str(),
            out.calls.as_slice(),
            out.notes_for("a")
        ),
        (
            "completed",
            ["a".to_string(), "on_fail".to_string()].as_slice(),
            vec!["node started", "action execution failure: boom in a"]
        )
    );
}

#[tokio::test]
async fn unknown_action_type_fails_the_node_without_aborting_the_run() {
    let g = graph(
        vec![
            trigger("t"),
            node(
                "a",
                WorkflowNodeKind::Action,
                json!({"type": "assistant_turn", "prompt": "p"}),
            ),
            action("on_fail"),
        ],
        vec![edge("t", "a", None), edge("a", "on_fail", Some("failure"))],
    );

    let out = run_graph(g, json!({})).await;

    assert_eq!(
        (
            out.run.status.as_str(),
            out.calls.as_slice(),
            out.notes_for("a")
        ),
        (
            "completed",
            ["on_fail".to_string()].as_slice(),
            vec!["node started", "unsupported action type 'assistant_turn'"]
        )
    );
}

#[tokio::test]
async fn edge_label_matching_ignores_case() {
    let g = graph(
        vec![trigger("t"), action("a"), action("b")],
        vec![edge("t", "a", None), edge("a", "b", Some("SUCCESS"))],
    );

    let out = run_graph(g, json!({})).await;

    assert_eq!(out.calls, ["a", "b"]);
}

#[tokio::test]
async fn always_true_condition_takes_true_branch() {
    let g = graph(
        vec![
            trigger("t"),
            condition("c", json!({"type": "always_true"})),
            action("yes"),
            action("no"),
        ],
        vec![
            edge("t", "c", None),
            edge("c", "yes", Some("true")),
            edge("c", "no", Some("false")),
        ],
    );

    let out = run_graph(g, json!({})).await;

    assert_eq!(out.calls, ["yes"]);
}

#[tokio::test]
async fn always_false_condition_takes_false_branch() {
    let g = graph(
        vec![
            trigger("t"),
            condition("c", json!({"type": "always_false"})),
            action("yes"),
            action("no"),
        ],
        vec![
            edge("t", "c", None),
            edge("c", "yes", Some("true")),
            edge("c", "no", Some("false")),
        ],
    );

    let out = run_graph(g, json!({})).await;

    assert_eq!(out.calls, ["no"]);
}

#[tokio::test]
async fn payload_key_condition_branches_on_key_presence() {
    let g = || {
        graph(
            vec![
                trigger("t"),
                condition(
                    "c",
                    json!({"type": "has_trigger_payload_key", "key": "ticket"}),
                ),
                action("yes"),
                action("no"),
            ],
            vec![
                edge("t", "c", None),
                edge("c", "yes", Some("true")),
                edge("c", "no", Some("false")),
            ],
        )
    };

    let with_key = run_graph(g(), json!({"ticket": 1})).await;
    let without_key = run_graph(g(), json!({"other": 1})).await;

    assert_eq!(
        (with_key.calls, without_key.calls),
        (vec!["yes".to_string()], vec!["no".to_string()])
    );
}

#[tokio::test]
async fn self_loop_stops_at_max_visits_per_node() {
    let mut g = graph(
        vec![trigger("t"), action("a")],
        vec![edge("t", "a", None), edge("a", "a", None)],
    );
    g.execution.max_visits_per_node = 3;

    let out = run_graph(g, json!({})).await;

    assert_eq!(
        (
            out.run.status.as_str(),
            out.run.error_message.as_deref(),
            out.calls.len()
        ),
        (
            "max_visits_exceeded",
            Some("node 'a' exceeded max_visits_per_node=3"),
            3
        )
    );
}

#[tokio::test]
async fn two_node_cycle_stops_at_max_steps() {
    let mut g = graph(
        vec![trigger("t"), action("a"), action("b")],
        vec![
            edge("t", "a", None),
            edge("a", "b", None),
            edge("b", "a", None),
        ],
    );
    g.execution.max_visits_per_node = 100;
    g.execution.max_steps = 6;

    let out = run_graph(g, json!({})).await;

    assert_eq!(
        (
            out.run.status.as_str(),
            out.run.error_message.as_deref(),
            out.calls.len()
        ),
        (
            "max_steps_exceeded",
            Some("execution exceeded max_steps=6"),
            5
        )
    );
}

#[tokio::test]
async fn run_starts_only_from_the_trigger_named_in_its_payload() {
    let g = graph(
        vec![
            trigger("t1"),
            trigger("t2"),
            action("from_t1"),
            action("from_t2"),
        ],
        vec![edge("t1", "from_t1", None), edge("t2", "from_t2", None)],
    );

    let out = run_graph(g, json!({"trigger_node_id": "t2"})).await;

    assert_eq!(out.calls, ["from_t2"]);
}

#[tokio::test]
async fn run_starts_only_from_triggers_matching_its_trigger_type() {
    let g = graph(
        vec![
            node(
                "manual_t",
                WorkflowNodeKind::Trigger,
                json!({"type": "manual"}),
            ),
            node(
                "hook_t",
                WorkflowNodeKind::Trigger,
                json!({"type": "webhook"}),
            ),
            action("from_manual"),
            action("from_hook"),
        ],
        vec![
            edge("manual_t", "from_manual", None),
            edge("hook_t", "from_hook", None),
        ],
    );

    let out = run_graph_with(g, WorkflowTriggerKind::Webhook, json!({}), vec![], vec![]).await;

    assert_eq!(out.calls, ["from_hook"]);
}

#[tokio::test]
async fn disabled_trigger_still_starts_the_run() {
    let g = graph(
        vec![
            node(
                "t",
                WorkflowNodeKind::Trigger,
                json!({"type": "manual", "disabled": true}),
            ),
            action("a"),
        ],
        vec![edge("t", "a", Some("trigger"))],
    );

    let out = run_graph(g, json!({})).await;

    assert_eq!(out.calls, ["a"]);
}

#[tokio::test]
async fn disabled_action_is_not_executed() {
    let g = graph(
        vec![
            trigger("t"),
            node(
                "a",
                WorkflowNodeKind::Action,
                json!({"type": "http_request", "url": "https://example.com", "disabled": true}),
            ),
            action("b"),
        ],
        vec![edge("t", "a", None), edge("a", "b", Some("success"))],
    );

    let out = run_graph(g, json!({})).await;

    assert_eq!(out.calls, ["b"]);
}

#[tokio::test]
async fn action_receives_node_config_and_trigger_payload() {
    struct Capture(Arc<Mutex<Option<WorkflowActionInput>>>);
    #[async_trait]
    impl WorkflowActionExecutor for Capture {
        fn action_type(&self) -> &'static str {
            "assistant_turn"
        }
        async fn execute(&self, input: WorkflowActionInput) -> Result<WorkflowActionResult> {
            *self.0.lock().unwrap() = Some(input);
            Ok(WorkflowActionResult::success("captured"))
        }
    }

    let g = graph(
        vec![
            trigger("t"),
            node(
                "a",
                WorkflowNodeKind::Action,
                json!({"type": "assistant_turn", "prompt": "hi"}),
            ),
        ],
        vec![edge("t", "a", None)],
    );
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let store = storage.workflow_store();
    let workflow_id = store.create("wf", "", &g, true).await.unwrap();
    let run_id = store
        .create_run(workflow_id, WorkflowTriggerKind::Manual, &json!({"k": "v"}))
        .await
        .unwrap();
    let seen = Arc::new(Mutex::new(None));
    let handle = spawn_workflow_runner(
        storage.clone(),
        Duration::from_millis(5),
        vec![Arc::new(Capture(seen.clone()))],
    );
    for _ in 0..400 {
        if seen.lock().unwrap().is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    handle.abort();
    let _ = handle.await;

    let input = seen.lock().unwrap().clone().expect("executor invoked");
    assert_eq!(
        (
            input.run_id,
            input.workflow_id,
            input.node_id.as_str(),
            input.config["prompt"].clone(),
            input.trigger_payload
        ),
        (run_id, workflow_id, "a", json!("hi"), json!({"k": "v"}))
    );
}
