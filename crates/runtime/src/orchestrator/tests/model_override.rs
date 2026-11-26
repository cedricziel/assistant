//! A conversation's `/model` override picks the model its turns run on.

use assistant_core::ConversationConfig;
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{Interface, MessageRole, TurnIdentity};
use assistant_storage::{ConversationStore, StorageLayer};
use serde_json::Value;
use uuid::Uuid;
use wiremock::MockServer;

use super::{Orchestrator, allow_all_tools, build_with_config, mount_answer};

async fn run(orch: &Orchestrator, conversation_id: Uuid) {
    run_as(orch, conversation_id, TurnIdentity::default()).await;
}

async fn run_as(orch: &Orchestrator, conversation_id: Uuid, identity: TurnIdentity) {
    orch.run_turn(
        "hello",
        conversation_id,
        Interface::Cli,
        None,
        vec![],
        identity,
    )
    .await
    .unwrap();
}

fn turn_model(model: &str) -> TurnIdentity {
    TurnIdentity {
        model_override: Some(model.to_string()),
        ..TurnIdentity::default()
    }
}

/// The model recorded on the conversation's assistant replies, in order.
async fn recorded_models(storage: &StorageLayer, conversation_id: Uuid) -> Vec<Option<String>> {
    storage
        .conversation_store()
        .load_history(conversation_id)
        .await
        .unwrap()
        .into_iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .map(|m| m.model)
        .collect()
}

/// Models named in the chat requests the server received, in order.
async fn requested_models(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter_map(|req| {
            let body: Value = serde_json::from_slice(&req.body).ok()?;
            body["model"].as_str().map(str::to_string)
        })
        .collect()
}

#[tokio::test]
async fn turns_use_the_conversations_model_override() {
    let server = MockServer::start().await;
    mount_answer(&server, "hi").await;

    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.llm.base_url = server.uri();
    allow_all_tools(&mut config);
    let (orch, _) = build_with_config(&server.uri(), config).await;

    let overridden = Uuid::new_v4();
    orch.conversation_configs.write().await.insert(
        overridden,
        ConversationConfig {
            model_override: Some("override-model".to_string()),
        },
    );

    run(&orch, overridden).await;
    assert_eq!(requested_models(&server).await, vec!["override-model"]);

    run(&orch, Uuid::new_v4()).await;
    assert_eq!(
        requested_models(&server).await,
        vec!["override-model", "test"],
        "a conversation without an override keeps the default model"
    );
}

#[tokio::test]
async fn the_model_command_switches_the_next_turn() {
    use crate::{CommandContext, CommandRegistry};

    let server = MockServer::start().await;
    mount_answer(&server, "hi").await;

    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.llm.base_url = server.uri();
    allow_all_tools(&mut config);
    let (orch, _) = build_with_config(&server.uri(), config).await;

    let conversation_id = Uuid::new_v4();
    let ctx = CommandContext {
        conversation_id,
        conversation_configs: orch.conversation_configs(),
        orchestrator: orch.clone(),
        evict_conversation: None,
        default_model: "test".to_string(),
    };
    CommandRegistry::new()
        .execute("model", &["chosen-model".to_string()], ctx)
        .await;

    run(&orch, conversation_id).await;
    assert_eq!(requested_models(&server).await, vec!["chosen-model"]);
}

#[tokio::test]
async fn a_turn_override_beats_the_conversation_override_without_persisting() {
    let server = MockServer::start().await;
    mount_answer(&server, "hi").await;

    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.llm.base_url = server.uri();
    allow_all_tools(&mut config);
    let (orch, _) = build_with_config(&server.uri(), config).await;

    let conversation_id = Uuid::new_v4();
    orch.conversation_configs.write().await.insert(
        conversation_id,
        ConversationConfig {
            model_override: Some("conversation-model".to_string()),
        },
    );

    run_as(&orch, conversation_id, turn_model("turn-model")).await;
    run(&orch, conversation_id).await;

    assert_eq!(
        requested_models(&server).await,
        vec!["turn-model", "conversation-model"],
        "the turn override wins once; the next turn is back on the /model override"
    );
    assert_eq!(
        orch.conversation_configs
            .read()
            .await
            .get(&conversation_id)
            .and_then(|c| c.model_override.as_deref()),
        Some("conversation-model"),
        "a turn override is never written to the conversation config"
    );
}

#[tokio::test]
async fn a_turn_override_beats_the_default_model() {
    let server = MockServer::start().await;
    mount_answer(&server, "hi").await;

    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.llm.base_url = server.uri();
    allow_all_tools(&mut config);
    let (orch, _) = build_with_config(&server.uri(), config).await;

    run_as(&orch, Uuid::new_v4(), turn_model("turn-model")).await;
    run(&orch, Uuid::new_v4()).await;

    assert_eq!(
        requested_models(&server).await,
        vec!["turn-model", "test"],
        "only the turn that asked for a model runs on it"
    );
}

#[tokio::test]
async fn assistant_replies_record_the_model_that_answered() {
    let server = MockServer::start().await;
    mount_answer(&server, "hi").await;

    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.llm.base_url = server.uri();
    allow_all_tools(&mut config);
    let (orch, storage) = build_with_config(&server.uri(), config).await;

    let default_conv = Uuid::new_v4();
    run(&orch, default_conv).await;
    assert_eq!(
        recorded_models(&storage, default_conv).await,
        vec![Some("test".to_string())],
        "the default provider's model name is recorded"
    );

    let overridden = Uuid::new_v4();
    run_as(&orch, overridden, turn_model("turn-model")).await;
    assert_eq!(
        recorded_models(&storage, overridden).await,
        vec![Some("turn-model".to_string())],
        "the turn override is recorded, not the default"
    );
}
