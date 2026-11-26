//! Shared rig for the `final_answer_traceparent*` integration tests: run one
//! scripted turn and return the saved final-answer row.

use std::sync::Arc;

use uuid::Uuid;

use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{Interface, Message, MessageRole, TurnIdentity};
use assistant_core::{LlmProvider, LlmResponse, LlmResponseMeta, MessageBus};
use assistant_llm_provider::scripted::ScriptedLlmProvider;
use assistant_runtime::Orchestrator;
use assistant_storage::conversations::ConversationStore;
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;

/// Run a one-answer turn and return the assistant row it saved.
pub async fn run_turn_and_load_answer() -> Message {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(ScriptedLlmProvider::new().with_canned_responses(
        vec![LlmResponse::FinalAnswer(
            "the answer".into(),
            LlmResponseMeta::default(),
        )],
    ));
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Orchestrator::new(llm, storage.clone(), executor, registry, bus, &config);

    let conversation_id = Uuid::new_v4();
    orch.run_turn(
        "question",
        conversation_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let history = storage
        .conversation_store_for_agent(orch.agent_id())
        .load_history(conversation_id)
        .await
        .unwrap();
    history
        .into_iter()
        .find(|m| m.role == MessageRole::Assistant)
        .unwrap()
}
