//! `ask-user` is in the tool list of interactive web turns that have an
//! interaction broker, and of no other turn (ADR-0017, D5).

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;
use uuid::Uuid;

use assistant_core::identity::UserId;
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{Interface, TurnIdentity};
use assistant_core::{
    ChatHistoryMessage, LlmProvider, LlmResponse, LlmResponseMeta, MessageBus, ToolCallItem,
    ToolCallResponse,
};
use assistant_llm_provider::scripted::ScriptedLlmProvider;
use assistant_storage::{
    InMemoryInteractionStore, InteractionStatus, InteractionStore, StorageLayer,
    registry::SkillRegistry,
};
use assistant_tool_executor::ToolExecutor;

use super::super::{Orchestrator, OrchestratorEvent};
use crate::interaction::{InteractionBroker, InteractionKinds, Responder};

struct Rig {
    orch: Arc<Orchestrator>,
    llm: Arc<ScriptedLlmProvider>,
    store: Arc<InMemoryInteractionStore>,
    broker: Arc<InteractionBroker>,
}

fn ask_user_call() -> LlmResponse {
    LlmResponse::ToolCalls(ToolCallResponse {
        items: vec![ToolCallItem {
            name: "ask-user".into(),
            params: json!({"questions": [
                {"prompt": "Proceed?", "options": [{"label": "yes"}, {"label": "no"}]}
            ]}),
            id: Some("toolu_ask".into()),
        }],
        meta: LlmResponseMeta::default(),
        thinking: None,
        provider_state: None,
    })
}

fn answer() -> LlmResponse {
    LlmResponse::FinalAnswer("done".into(), LlmResponseMeta::default())
}

async fn rig(script: Vec<LlmResponse>, with_broker: bool) -> Rig {
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
    let store = Arc::new(InMemoryInteractionStore::new());
    let broker = Arc::new(
        InteractionBroker::new(store.clone(), Arc::new(InteractionKinds::with_builtin()))
            .with_timeout(Duration::from_secs(60)),
    );
    let mut orch = Orchestrator::new(provider, storage, executor.clone(), registry, bus, &config);
    if with_broker {
        orch = orch.with_interaction_broker(broker.clone());
    }
    let orch = Arc::new(orch);
    executor.set_subagent_runner(orch.clone());
    Rig {
        orch,
        llm,
        store,
        broker,
    }
}

fn identity(interactive: bool) -> TurnIdentity {
    TurnIdentity {
        user_id: Some(UserId::from("alice")),
        interactive,
        run_id: Some(Uuid::new_v4()),
        ..TurnIdentity::default()
    }
}

fn offers_ask_user(rig: &Rig, call_index: usize) -> bool {
    rig.llm.recorded_calls()[call_index]
        .tools
        .iter()
        .any(|t| t.name == "ask-user")
}

async fn offered_to(rig: &Rig, interface: Interface, interactive: bool) -> bool {
    rig.orch
        .run_turn(
            "go",
            Uuid::new_v4(),
            interface,
            None,
            vec![],
            identity(interactive),
        )
        .await
        .unwrap();
    offers_ask_user(rig, 0)
}

#[tokio::test]
async fn an_interactive_web_turn_is_offered_ask_user() {
    let rig = rig(vec![answer()], true).await;
    assert!(offered_to(&rig, Interface::Web, true).await);
}

#[tokio::test]
async fn turns_nobody_is_watching_are_not_offered_ask_user() {
    // Scheduler, and a voice or quick-message turn (web, not interactive).
    for (interface, interactive) in [
        (Interface::Scheduler, false),
        (Interface::Web, false),
        (Interface::Slack, false),
    ] {
        let rig = rig(vec![answer()], true).await;
        assert!(
            !offered_to(&rig, interface.clone(), interactive).await,
            "{interface:?} interactive={interactive}"
        );
    }
}

#[tokio::test]
async fn a_cli_turn_has_no_renderer_for_questions() {
    let rig = rig(vec![answer()], true).await;
    assert!(!offered_to(&rig, Interface::Cli, true).await);
}

#[tokio::test]
async fn without_a_broker_no_turn_is_offered_ask_user() {
    let rig = rig(vec![answer()], false).await;
    assert!(!offered_to(&rig, Interface::Web, true).await);
}

#[tokio::test]
async fn the_model_asks_the_owner_and_reads_the_answer() {
    let rig = rig(vec![ask_user_call(), answer()], true).await;
    let conversation_id = Uuid::new_v4();
    let identity = identity(true);
    let run_id = identity.run_id.unwrap();
    let (tx, mut rx) = mpsc::channel::<OrchestratorEvent>(64);

    let orch = rig.orch.clone();
    let turn = tokio::spawn(async move {
        orch.run_turn_streaming(
            "go",
            conversation_id,
            Interface::Web,
            tx,
            None,
            vec![],
            identity,
        )
        .await
    });

    let requested = loop {
        match rx
            .recv()
            .await
            .expect("the turn ends only after the answer")
        {
            OrchestratorEvent::InteractionRequested {
                interaction_id,
                kind,
                tool_call_id,
                ..
            } => break (interaction_id, kind, tool_call_id),
            _ => continue,
        }
    };
    assert_eq!(requested.1, "question");
    assert_eq!(
        requested.2.as_deref(),
        Some("toolu_ask"),
        "the card belongs to the ask-user call"
    );
    let row = rig.store.get(&requested.0).await.unwrap().unwrap();
    assert_eq!(row.status, InteractionStatus::Pending);

    rig.broker
        .respond(
            &Responder {
                user_id: "alice",
                conversation_id: &conversation_id.to_string(),
                run_id: &run_id.to_string(),
            },
            &requested.0,
            json!({"answers": [{"question_id": "q1", "selected_option_ids": ["o1"]}]}),
        )
        .await
        .unwrap();

    let result = turn.await.unwrap().unwrap();
    assert_eq!(result.answer, "done");
    let last = rig.llm.recorded_calls().last().cloned().unwrap();
    let observation = last
        .history
        .iter()
        .find_map(|m| match m {
            ChatHistoryMessage::ToolResult { name, content } if name == "ask-user" => {
                Some(content.clone())
            }
            _ => None,
        })
        .expect("the model sees the tool result");
    let observed: Value = serde_json::from_str(&observation).unwrap();
    assert_eq!(
        observed,
        json!({"answers": [{"question": "Proceed?", "selected": ["yes"], "text": ""}]})
    );
}
