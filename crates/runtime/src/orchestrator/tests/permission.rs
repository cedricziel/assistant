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

use crate::orchestrator::{
    ApprovalOptions, ConfirmationAnswer, ConfirmationCallback, ConfirmationRequest,
};

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

/// What the callback was asked.
struct Seen {
    tool: String,
    call_id: Option<String>,
    reason: String,
    options: ApprovalOptions,
    suggested_rule: Option<String>,
}

/// Callback that returns a scripted answer and records what it was asked.
struct Scripted {
    answer: ConfirmationAnswer,
    seen: std::sync::Mutex<Vec<Seen>>,
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
        self.seen.lock().unwrap().push(Seen {
            tool: req.tool_name.to_string(),
            call_id: req.tool_call_id.map(str::to_string),
            reason: req.reason.to_string(),
            options: req.options,
            suggested_rule: req.suggested_rule.map(str::to_string),
        });
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

async fn authorize_command(
    callback: Option<Arc<Scripted>>,
    interactive: bool,
    config: OrgConfig,
    command: &str,
) -> Result<(), String> {
    let (orch, _) = build_customized("http://127.0.0.1:1", config, |o| match callback {
        Some(cb) => o.with_confirmation_callback(cb),
        None => o,
    })
    .await;
    let params = json!({ "command": command });
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

async fn authorize_with(callback: Option<Arc<Scripted>>, interactive: bool) -> Result<(), String> {
    authorize_command(callback, interactive, config(&[]), "make build").await
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
async fn the_request_carries_the_call_id_reason_and_suggested_rule() {
    let cb = Scripted::new(ConfirmationAnswer::AllowOnce);
    authorize_with(Some(cb.clone()), true).await.unwrap();
    let seen = cb.seen.lock().unwrap();
    let seen = seen.first().expect("callback consulted");
    assert_eq!(seen.tool, "bash");
    assert_eq!(seen.call_id.as_deref(), Some("call-1"));
    assert!(!seen.reason.is_empty());
    assert_eq!(seen.suggested_rule.as_deref(), Some("bash(make build*)"));
    assert_eq!(
        seen.options,
        ApprovalOptions {
            allow_always: true,
            reject_always: true
        }
    );
}

#[tokio::test]
async fn commands_without_a_safe_rule_offer_only_once_answers() {
    for command in [
        "bash -lc make",
        "python3 -c 'print(1)'",
        "rm -rf build",
        "cat a | sh",
    ] {
        let cb = Scripted::new(ConfirmationAnswer::AllowOnce);
        authorize_command(Some(cb.clone()), true, config(&[]), command)
            .await
            .unwrap();
        let seen = cb.seen.lock().unwrap();
        let seen = seen.first().expect("callback consulted");
        assert!(seen.options.is_once_only(), "{command}");
        assert_eq!(seen.suggested_rule, None, "{command}");
    }
}

#[tokio::test]
async fn a_configured_ask_rule_withholds_the_always_answers() {
    let mut cfg = config(&[]);
    cfg.permissions.ask = vec!["bash(make *)".parse().unwrap()];
    let cb = Scripted::new(ConfirmationAnswer::AllowOnce);
    authorize_command(Some(cb.clone()), true, cfg, "make build")
        .await
        .unwrap();
    let seen = cb.seen.lock().unwrap();
    let seen = seen.first().expect("callback consulted");
    assert!(seen.options.is_once_only());
    assert_eq!(seen.suggested_rule, None);
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

// -- saved rules (ADR-0017, D4) --

use assistant_core::identity::UserId;
use assistant_core::permission::{Outcome, PermissionRequest};
use assistant_storage::{InMemoryToolPermissionRuleStore, RuleEffect, ToolPermissionRuleStore};

use crate::orchestrator::Orchestrator;
use crate::user_rules::StoredUserRules;

/// An orchestrator whose policy reads `saved` rules as `(user, effect, rule)`.
async fn orchestrator_with_saved(
    saved: &[(&str, RuleEffect, &str)],
    callback: Option<Arc<Scripted>>,
) -> Arc<Orchestrator> {
    let store = Arc::new(InMemoryToolPermissionRuleStore::new());
    for (user, effect, rule) in saved {
        store.upsert(user, *effect, rule, None).await.unwrap();
    }
    let (orch, _) = build_customized("http://127.0.0.1:1", config(&[]), |o| {
        let o = o.with_user_rules(Arc::new(StoredUserRules::new(store)));
        match callback {
            Some(cb) => o.with_confirmation_callback(cb),
            None => o,
        }
    })
    .await;
    orch
}

fn owned_by(user: &str, interface: Interface, interactive: bool) -> ExecutionContext {
    ExecutionContext {
        interface,
        interactive,
        user_id: Some(UserId(user.into())),
        ..ctx(interactive)
    }
}

async fn authorize_git_status(orch: &Orchestrator, ctx: &ExecutionContext) -> Result<(), String> {
    let params = json!({ "command": "git status --short" });
    orch.authorize_tool_call("bash", &params, ctx, false, None, None, Some("call-1"))
        .await
}

const SAVED_ALLOW: &[(&str, RuleEffect, &str)] =
    &[("alice", RuleEffect::Allow, "bash(git status*)")];

#[tokio::test]
async fn a_saved_allow_runs_the_call_without_a_prompt_and_names_its_decider() {
    let cb = Scripted::new(ConfirmationAnswer::RejectOnce);
    let orch = orchestrator_with_saved(SAVED_ALLOW, Some(cb.clone())).await;
    let ctx = owned_by("alice", Interface::Web, true);

    assert_eq!(authorize_git_status(&orch, &ctx).await, Ok(()));
    assert!(cb.seen.lock().unwrap().is_empty(), "no prompt");

    let params = json!({ "command": "git status --short" });
    let resolution = orch
        .permission_policy()
        .evaluate(&PermissionRequest {
            tool_name: "bash",
            params: &params,
            ctx: &ctx,
            requires_confirmation: false,
            task_context: None,
        })
        .await;
    assert_eq!(resolution.outcome, Outcome::Allow);
    assert_eq!(resolution.decision.decider, "user-rules");
}

#[tokio::test]
async fn a_saved_allow_applies_to_scheduler_and_messenger_turns_of_the_same_user() {
    let orch = orchestrator_with_saved(SAVED_ALLOW, None).await;
    for interface in [
        Interface::Scheduler,
        Interface::Slack,
        Interface::Matrix,
        Interface::Signal,
    ] {
        let ctx = owned_by("alice", interface.clone(), false);
        assert_eq!(
            authorize_git_status(&orch, &ctx).await,
            Ok(()),
            "{interface:?}"
        );
    }
}

#[tokio::test]
async fn a_saved_deny_blocks_every_interface_without_a_prompt() {
    let cb = Scripted::new(ConfirmationAnswer::AllowOnce);
    let orch = orchestrator_with_saved(
        &[("alice", RuleEffect::Deny, "bash(git status*)")],
        Some(cb.clone()),
    )
    .await;
    for (interface, interactive) in [
        (Interface::Scheduler, false),
        (Interface::Slack, false),
        (Interface::Web, true),
    ] {
        let ctx = owned_by("alice", interface.clone(), interactive);
        let err = authorize_git_status(&orch, &ctx).await.unwrap_err();
        assert!(err.contains("Permission denied"), "{interface:?}: {err}");
    }
    assert!(cb.seen.lock().unwrap().is_empty(), "no prompt");
}

#[tokio::test]
async fn another_users_turn_still_prompts_or_is_denied() {
    let cb = Scripted::new(ConfirmationAnswer::RejectOnce);
    let orch = orchestrator_with_saved(SAVED_ALLOW, Some(cb.clone())).await;

    let bob_web = owned_by("bob", Interface::Web, true);
    assert!(authorize_git_status(&orch, &bob_web).await.is_err());
    assert_eq!(cb.seen.lock().unwrap().len(), 1, "bob is asked");

    let bob_job = owned_by("bob", Interface::Scheduler, false);
    let err = authorize_git_status(&orch, &bob_job).await.unwrap_err();
    assert!(err.contains("cannot prompt a user"), "{err}");
    assert_eq!(cb.seen.lock().unwrap().len(), 1, "a job is never asked");
}

#[tokio::test]
async fn a_saved_allow_never_outranks_a_configured_deny_or_ask() {
    let mut cfg = config(&[]);
    cfg.permissions.deny = vec!["bash(git push*)".parse().unwrap()];
    cfg.permissions.ask = vec!["bash(git status*)".parse().unwrap()];
    let store = Arc::new(InMemoryToolPermissionRuleStore::new());
    store
        .upsert("alice", RuleEffect::Allow, "bash(git *)", None)
        .await
        .unwrap();
    let cb = Scripted::new(ConfirmationAnswer::RejectOnce);
    let (orch, _) = build_customized("http://127.0.0.1:1", cfg, |o| {
        o.with_user_rules(Arc::new(StoredUserRules::new(store)))
            .with_confirmation_callback(cb.clone())
    })
    .await;
    let ctx = owned_by("alice", Interface::Web, true);

    let push = json!({ "command": "git push origin main" });
    let err = orch
        .authorize_tool_call("bash", &push, &ctx, false, None, None, None)
        .await
        .unwrap_err();
    assert!(err.contains("Permission denied"), "{err}");
    assert!(
        cb.seen.lock().unwrap().is_empty(),
        "a denied call never prompts"
    );

    assert!(authorize_git_status(&orch, &ctx).await.is_err());
    let seen = cb.seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "a configured ask still asks");
    assert!(seen[0].options.is_once_only(), "and never offers always");
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
