//! A conversation's `/model` override picks the model its turns run on.

use assistant_core::ConversationConfig;
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{Interface, TurnIdentity};
use serde_json::Value;
use uuid::Uuid;
use wiremock::MockServer;

use super::{Orchestrator, allow_all_tools, build_with_config, mount_answer};

async fn run(orch: &Orchestrator, conversation_id: Uuid) {
    orch.run_turn(
        "hello",
        conversation_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();
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
