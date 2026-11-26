//! The permission pipeline gates every tool call dispatched by the
//! orchestrator, including non-interactive turns.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};

use crate::orchestrator::{ConfirmationAnswer, ConfirmationCallback, ConfirmationRequest};

use super::super::OrchestratorEvent;
use super::{build_customized, build_with_config, ollama_answer, ollama_tool_calls_with_args};

/// Mount: first chat → a `bash` tool call, afterwards → a final answer.
async fn mount_bash_call(server: &MockServer, command: &str) {
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[(
                "bash",
                json!({ "command": command }),
            )])),
        )
        .up_to_n_times(1)
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(server)
        .await;
}

fn config(allow: &[&str]) -> OrgConfig {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.permissions.allow = allow.iter().map(|r| r.parse().unwrap()).collect();
    config
}

/// Run one non-interactive (Slack) turn and return the `ToolResult` events.
async fn run_turn(config: OrgConfig, command: &str) -> Vec<(String, String)> {
    let server = MockServer::start().await;
    mount_bash_call(&server, command).await;
    let (orch, _) = build_with_config(&server.uri(), config).await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);
    orch.run_turn_with_tools_streaming(
        "do it",
        Uuid::new_v4(),
        Interface::Slack,
        vec![],
        None,
        vec![],
        tx,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();
    let mut results = Vec::new();
    while let Ok(event) = rx.try_recv() {
        if let OrchestratorEvent::ToolResult { status, result, .. } = event {
            results.push((status, result.unwrap_or_default()));
        }
    }
    results
}

#[tokio::test]
async fn non_interactive_turn_denies_undecided_bash_and_tells_the_llm() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marker");
    let results = run_turn(config(&[]), &format!("touch {}", marker.display())).await;

    assert!(!marker.exists(), "denied command must not run");
    let (status, message) = results.first().expect("a tool result event");
    assert_eq!(status, "denied");
    assert!(message.contains("Permission denied"), "{message}");
    assert!(message.contains("bash"), "{message}");
}

#[tokio::test]
async fn static_allow_rule_lets_the_command_run() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marker");
    let results = run_turn(
        config(&["bash(touch *)"]),
        &format!("touch {}", marker.display()),
    )
    .await;

    assert!(
        marker.exists(),
        "allowed command must run; events: {results:?}"
    );
}

#[tokio::test]
async fn hard_deny_beats_a_configured_allow() {
    let results = run_turn(config(&["bash(*)"]), "sudo touch /tmp/never").await;

    let (status, message) = results.first().expect("a tool result event");
    assert_eq!(status, "denied");
    assert!(message.contains("bash(sudo *)"), "{message}");
}

/// Callback that returns a scripted answer and records what it was asked.
struct Scripted {
    answer: ConfirmationAnswer,
    seen: std::sync::Mutex<Vec<(String, Option<String>, String, bool)>>,
}

impl Scripted {
    fn new(answer: ConfirmationAnswer) -> Arc<Self> {
        Arc::new(Self {
            answer,
            seen: std::sync::Mutex::new(Vec::new()),
        })
    }
}

#[async_trait]
impl ConfirmationCallback for Scripted {
    async fn confirm(&self, req: ConfirmationRequest<'_>) -> ConfirmationAnswer {
        // Yield so the test proves the call is genuinely awaited.
        tokio::task::yield_now().await;
        self.seen.lock().unwrap().push((
            req.tool_name.to_string(),
            req.tool_call_id.map(str::to_string),
            req.reason.to_string(),
            req.options.is_once_only(),
        ));
        self.answer
    }
}

fn ctx(interactive: bool) -> ExecutionContext {
    ExecutionContext {
        conversation_id: Uuid::new_v4(),
        agent_id: "perm-test".into(),
        turn: 0,
        interface: if interactive {
            Interface::Cli
        } else {
            Interface::Slack
        },
        interactive,
        run_id: None,
        allowed_tools: None,
        depth: 0,
        user_id: None,
        org_id: None,
        space_id: None,
    }
}

async fn authorize_with(callback: Option<Arc<Scripted>>, interactive: bool) -> Result<(), String> {
    let (orch, _) = build_customized("http://127.0.0.1:1", config(&[]), |o| match callback {
        Some(cb) => o.with_confirmation_callback(cb),
        None => o,
    })
    .await;
    let params = json!({ "command": "make build" });
    orch.authorize_tool_call(
        "bash",
        &params,
        &ctx(interactive),
        false,
        None,
        None,
        Some("call-1"),
    )
    .await
}

async fn authorize(answer: ConfirmationAnswer) -> Result<(), String> {
    authorize_with(Some(Scripted::new(answer)), true).await
}

#[tokio::test]
async fn allow_answers_run_the_tool() {
    assert_eq!(authorize(ConfirmationAnswer::AllowOnce).await, Ok(()));
    assert_eq!(authorize(ConfirmationAnswer::AllowAlways).await, Ok(()));
}

#[tokio::test]
async fn reject_and_cancel_answers_report_a_user_denial() {
    for answer in [
        ConfirmationAnswer::RejectOnce,
        ConfirmationAnswer::RejectAlways,
        ConfirmationAnswer::Cancelled,
    ] {
        let denied = authorize(answer).await.unwrap_err();
        assert!(
            denied.contains("User denied execution of 'bash'"),
            "{answer:?}: {denied}"
        );
    }
}

#[tokio::test]
async fn expired_answer_tells_the_llm_the_request_expired() {
    let message = authorize(ConfirmationAnswer::Expired).await.unwrap_err();
    assert!(message.contains("expired"), "{message}");
    assert!(message.contains("bash"), "{message}");
    assert!(!message.contains("User denied"), "{message}");
}

#[tokio::test]
async fn the_request_carries_the_call_id_reason_and_once_only_options() {
    let cb = Scripted::new(ConfirmationAnswer::AllowOnce);
    authorize_with(Some(cb.clone()), true).await.unwrap();
    let seen = cb.seen.lock().unwrap();
    let (tool, call_id, reason, once_only) = seen.first().expect("callback consulted");
    assert_eq!(tool, "bash");
    assert_eq!(call_id.as_deref(), Some("call-1"));
    assert!(!reason.is_empty());
    assert!(*once_only, "no suggested rule yet, so only once answers");
}

#[tokio::test]
async fn interactive_turn_defers_undecided_calls_to_the_human() {
    assert_eq!(authorize(ConfirmationAnswer::AllowOnce).await, Ok(()));
    let denied = authorize(ConfirmationAnswer::RejectOnce).await.unwrap_err();
    assert!(denied.contains("User denied"), "{denied}");
}

#[tokio::test]
async fn callback_is_never_consulted_on_non_interactive_turns() {
    let cb = Scripted::new(ConfirmationAnswer::AllowOnce);
    let err = authorize_with(Some(cb.clone()), false).await.unwrap_err();
    assert!(cb.seen.lock().unwrap().is_empty(), "callback was consulted");
    assert!(err.contains("cannot prompt a user"), "{err}");
}

#[tokio::test]
async fn interactive_turn_without_a_callback_is_denied() {
    assert!(authorize_with(None, true).await.is_err());
}

#[tokio::test]
async fn tools_requiring_confirmation_are_denied_when_nobody_can_be_asked() {
    let (orch, _) = build_with_config("http://127.0.0.1:1", config(&[])).await;
    let params = json!({});
    let err = orch
        .authorize_tool_call("mcp__x__do", &params, &ctx(false), true, None, None, None)
        .await
        .unwrap_err();
    assert!(err.contains("requires confirmation"), "{err}");
}

// -- decision model end to end --

use assistant_core::permission::PermissionPolicy;
use assistant_core::types::permissions::DecisionModelConfig;

use crate::decision_model::JevDecider;

async fn jev_server(reversible: f64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/decisions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {
                "reversible": { "noul": reversible },
                "serves_task": { "noul": reversible },
            }
        })))
        .mount(&server)
        .await;
    server
}

/// Run a turn whose `bash` call is undecided by static rules, with a Jev
/// decider pointed at `jev`.
async fn run_with_model(jev: &MockServer, command: &str) -> Vec<(String, String)> {
    let llm = MockServer::start().await;
    mount_bash_call(&llm, command).await;
    let policy = PermissionPolicy::from_config(&config(&[]).permissions).with_model(
        std::sync::Arc::new(JevDecider::new(
            DecisionModelConfig {
                enabled: true,
                base_url: format!("{}/decisions", jev.uri()),
                ..Default::default()
            },
            "k".into(),
        )),
    );
    let (orch, _) = build_customized(&llm.uri(), config(&[]), |o| {
        o.with_permission_policy(policy)
    })
    .await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);
    orch.run_turn_with_tools_streaming(
        "do it",
        Uuid::new_v4(),
        Interface::Slack,
        vec![],
        None,
        vec![],
        tx,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();
    let mut results = Vec::new();
    while let Ok(event) = rx.try_recv() {
        if let OrchestratorEvent::ToolResult { status, result, .. } = event {
            results.push((status, result.unwrap_or_default()));
        }
    }
    results
}

#[tokio::test]
async fn model_approval_lets_an_undecided_call_run_on_a_non_interactive_turn() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marker");
    let jev = jev_server(0.97).await;
    run_with_model(&jev, &format!("touch {}", marker.display())).await;
    assert!(marker.exists());
    assert_eq!(jev.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn model_low_confidence_means_denied_on_a_non_interactive_turn() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marker");
    let jev = jev_server(0.4).await;
    let results = run_with_model(&jev, &format!("touch {}", marker.display())).await;
    assert!(!marker.exists());
    assert_eq!(results[0].0, "denied");
    assert!(results[0].1.contains("threshold"), "{}", results[0].1);
}

#[tokio::test]
async fn hard_denied_calls_never_reach_the_model() {
    let jev = jev_server(0.99).await;
    let results = run_with_model(&jev, "sudo touch /tmp/never").await;
    assert_eq!(results[0].0, "denied");
    assert!(jev.received_requests().await.unwrap().is_empty());
}
