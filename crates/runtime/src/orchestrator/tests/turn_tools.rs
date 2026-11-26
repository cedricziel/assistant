//! Tools a turn gets from its [`TurnToolSource`] are that turn's alone, and
//! their trust level goes through the same confirmation gate as other tools.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use assistant_core::identity::UserId;
use assistant_core::tool::{ToolHandler, ToolOutput, TurnScope, TurnToolSource};
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};
use assistant_core::{
    LlmProvider, LlmResponse, LlmResponseMeta, MessageBus, ToolCallItem, ToolCallResponse,
};
use assistant_llm_provider::scripted::ScriptedLlmProvider;
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;

use super::super::{ConfirmationAnswer, ConfirmationCallback, ConfirmationRequest, Orchestrator};

/// A tool that records that it ran.
struct Probe {
    name: String,
    confirm: bool,
    runs: Arc<AtomicUsize>,
}

#[async_trait]
impl ToolHandler for Probe {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        "probe"
    }
    fn params_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    fn is_mutating(&self) -> bool {
        true
    }
    fn requires_confirmation(&self) -> bool {
        self.confirm
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        Ok(ToolOutput::success(format!("ran {}", self.name)))
    }
}

/// Hands each user their own tools; a system turn gets `system_tools`.
struct PerUser {
    tools: HashMap<String, Vec<Arc<dyn ToolHandler>>>,
    seen: std::sync::Mutex<Vec<(String, String, Option<String>)>>,
}

#[async_trait]
impl TurnToolSource for PerUser {
    async fn tools_for_turn(&self, scope: &TurnScope<'_>) -> Vec<Arc<dyn ToolHandler>> {
        self.seen.lock().unwrap().push((
            scope.org_slug.into(),
            scope.space_slug.into(),
            scope.user_id.map(|u| u.0.clone()),
        ));
        scope
            .user_id
            .and_then(|u| self.tools.get(&u.0))
            .cloned()
            .unwrap_or_default()
    }
}

struct Counting {
    answer: bool,
    asked: Arc<AtomicUsize>,
}

#[async_trait]
impl ConfirmationCallback for Counting {
    async fn confirm(&self, _req: ConfirmationRequest<'_>) -> ConfirmationAnswer {
        self.asked.fetch_add(1, Ordering::SeqCst);
        if self.answer {
            ConfirmationAnswer::AllowOnce
        } else {
            ConfirmationAnswer::RejectOnce
        }
    }
}

fn probe(name: &str, confirm: bool, runs: &Arc<AtomicUsize>) -> Arc<dyn ToolHandler> {
    Arc::new(Probe {
        name: name.into(),
        confirm,
        runs: runs.clone(),
    })
}

fn call(name: &str) -> LlmResponse {
    LlmResponse::ToolCalls(ToolCallResponse {
        items: vec![ToolCallItem {
            name: name.into(),
            params: json!({}),
            id: None,
        }],
        meta: LlmResponseMeta::default(),
        thinking: None,
        provider_state: None,
    })
}

fn answer() -> LlmResponse {
    LlmResponse::FinalAnswer("done".into(), LlmResponseMeta::default())
}

struct Rig {
    orch: Arc<Orchestrator>,
    llm: Arc<ScriptedLlmProvider>,
    source: Arc<PerUser>,
}

async fn rig(
    script: Vec<LlmResponse>,
    tools: Vec<(&str, Vec<Arc<dyn ToolHandler>>)>,
    confirm: Option<Arc<dyn ConfirmationCallback>>,
) -> Rig {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm = Arc::new(ScriptedLlmProvider::new().with_canned_responses(script));
    let provider: Arc<dyn LlmProvider> = llm.clone();
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        provider.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let source = Arc::new(PerUser {
        tools: tools
            .into_iter()
            .map(|(user, tools)| (user.to_owned(), tools))
            .collect(),
        seen: Default::default(),
    });
    let mut orch = Orchestrator::new(provider, storage, executor.clone(), registry, bus, &config)
        .with_turn_tool_source(source.clone(), "acme", "eng");
    if let Some(callback) = confirm {
        orch = orch.with_confirmation_callback(callback);
    }
    let orch = Arc::new(orch);
    executor.set_subagent_runner(orch.clone());
    Rig { orch, llm, source }
}

fn identity(user: Option<&str>) -> TurnIdentity {
    TurnIdentity {
        user_id: user.map(UserId::from),
        org_id: None,
        space_id: None,
        ..TurnIdentity::default()
    }
}

async fn run(rig: &Rig, interface: Interface, user: Option<&str>) -> String {
    run_as(rig, interface, identity(user)).await
}

/// A turn whose submitter said a human is waiting on it.
async fn run_interactive(rig: &Rig, interface: Interface, user: Option<&str>) -> String {
    let identity = TurnIdentity {
        interactive: true,
        ..identity(user)
    };
    run_as(rig, interface, identity).await
}

async fn run_as(rig: &Rig, interface: Interface, identity: TurnIdentity) -> String {
    rig.orch
        .run_turn("go", Uuid::new_v4(), interface, None, vec![], identity)
        .await
        .unwrap()
        .answer
}

fn offered(rig: &Rig, call_index: usize) -> Vec<String> {
    rig.llm.recorded_calls()[call_index]
        .tools
        .iter()
        .map(|t| t.name.clone())
        .collect()
}

#[tokio::test]
async fn a_turn_offers_its_users_tools_and_no_one_elses() {
    let runs = Arc::new(AtomicUsize::new(0));
    let rig = rig(
        vec![answer(), answer()],
        vec![
            (
                "u",
                vec![
                    probe("mcp__org_linear__create", true, &runs),
                    probe("mcp__space_sentry__list", true, &runs),
                    probe("mcp__user_github__issue", true, &runs),
                ],
            ),
            ("v", vec![probe("mcp__org_linear__create", true, &runs)]),
        ],
        None,
    )
    .await;

    run(&rig, Interface::Web, Some("u")).await;
    run(&rig, Interface::Web, Some("v")).await;

    let u = offered(&rig, 0);
    for name in [
        "mcp__org_linear__create",
        "mcp__space_sentry__list",
        "mcp__user_github__issue",
    ] {
        assert!(u.contains(&name.to_string()), "U lacks {name}: {u:?}");
    }
    let v = offered(&rig, 1);
    assert!(v.contains(&"mcp__org_linear__create".to_string()));
    assert!(
        !v.iter()
            .any(|n| n.starts_with("mcp__user_") || n.starts_with("mcp__space_")),
        "V must not see U's tools: {v:?}"
    );
    assert!(u.contains(&"file-read".to_string()), "builtins stay: {u:?}");
}

#[tokio::test]
async fn the_source_is_asked_for_this_space_and_user() {
    let rig = rig(vec![answer()], vec![], None).await;
    run(&rig, Interface::Web, Some("u")).await;
    assert_eq!(
        *rig.source.seen.lock().unwrap(),
        vec![("acme".to_string(), "eng".to_string(), Some("u".to_string()))]
    );
}

#[tokio::test]
async fn a_turn_without_a_user_asks_with_no_user() {
    let rig = rig(vec![answer()], vec![], None).await;
    run(&rig, Interface::Scheduler, None).await;
    assert_eq!(rig.source.seen.lock().unwrap()[0].2, None);
}

#[tokio::test]
async fn the_overlay_never_reaches_the_shared_registry() {
    let runs = Arc::new(AtomicUsize::new(0));
    let rig = rig(
        vec![answer()],
        vec![("u", vec![probe("mcp__user_github__issue", true, &runs)])],
        None,
    )
    .await;
    run(&rig, Interface::Web, Some("u")).await;
    assert!(
        !rig.orch
            .executor
            .to_specs()
            .iter()
            .any(|s| s.name == "mcp__user_github__issue")
    );
}

#[tokio::test]
async fn a_confirm_tool_asks_the_user_and_runs_only_when_they_agree() {
    for (agree, expected_runs) in [(true, 1), (false, 0)] {
        let runs = Arc::new(AtomicUsize::new(0));
        let asked = Arc::new(AtomicUsize::new(0));
        let rig = rig(
            vec![call("mcp__user_github__issue"), answer()],
            vec![("u", vec![probe("mcp__user_github__issue", true, &runs)])],
            Some(Arc::new(Counting {
                answer: agree,
                asked: asked.clone(),
            })),
        )
        .await;

        run_interactive(&rig, Interface::Web, Some("u")).await;

        assert_eq!(asked.load(Ordering::SeqCst), 1, "agree={agree}");
        assert_eq!(runs.load(Ordering::SeqCst), expected_runs, "agree={agree}");
    }
}

#[tokio::test]
async fn the_models_tool_list_carries_each_tools_confirmation_flag() {
    let runs = Arc::new(AtomicUsize::new(0));
    let rig = rig(
        vec![answer()],
        vec![(
            "u",
            vec![
                probe("mcp__user_a__confirm", true, &runs),
                probe("mcp__user_a__trusted", false, &runs),
            ],
        )],
        None,
    )
    .await;

    run(&rig, Interface::Web, Some("u")).await;

    let specs = &rig.llm.recorded_calls()[0].tools;
    let flag = |name: &str| {
        specs
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.requires_confirmation)
    };
    assert_eq!(flag("mcp__user_a__confirm"), Some(true));
    assert_eq!(flag("mcp__user_a__trusted"), Some(false));
}

#[tokio::test]
async fn a_tool_of_another_user_cannot_be_called() {
    let runs = Arc::new(AtomicUsize::new(0));
    let rig = rig(
        vec![call("mcp__user_github__issue"), answer()],
        vec![
            ("u", vec![probe("mcp__user_github__issue", false, &runs)]),
            ("v", vec![]),
        ],
        None,
    )
    .await;

    run(&rig, Interface::Cli, Some("v")).await;

    assert_eq!(runs.load(Ordering::SeqCst), 0, "V must not run U's tool");
}

#[tokio::test]
async fn extension_turns_get_the_overlay_too() {
    let runs = Arc::new(AtomicUsize::new(0));
    let rig = rig(
        vec![answer()],
        vec![("u", vec![probe("mcp__user_github__issue", true, &runs)])],
        None,
    )
    .await;

    rig.orch
        .run_turn_with_tools(
            "go",
            Uuid::new_v4(),
            Interface::Slack,
            vec![],
            None,
            vec![],
            vec![],
            identity(Some("u")),
        )
        .await
        .unwrap();

    assert!(offered(&rig, 0).contains(&"mcp__user_github__issue".to_string()));
}
