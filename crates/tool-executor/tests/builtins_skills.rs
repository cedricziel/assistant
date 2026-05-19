//! Coverage tests for skill-related builtins (load-skill, list-skills).

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::ToolHandler;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_storage::{SkillRegistry, StorageLayer};
use assistant_tool_executor::builtins::list_skills::ListSkillsHandler;
use assistant_tool_executor::builtins::load_skill::LoadSkillHandler;
use serde_json::json;

fn ctx() -> ExecutionContext {
    ExecutionContext {
        conversation_id: uuid::Uuid::new_v4(),
        agent_id: "default".into(),
        turn: 0,
        interface: Interface::Cli,
        interactive: false,
        allowed_tools: None,
        depth: 0,
        user_id: None,
        org_id: None,
        space_id: None,
    }
}

fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

async fn make_registry() -> Arc<SkillRegistry> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
    Arc::new(registry)
}

// ── load_skill ────────────────────────────────────────────────────────────

#[tokio::test]
async fn load_skill_missing_name_returns_error() {
    let registry = make_registry().await;
    let handler = LoadSkillHandler::new(registry);
    let out = handler.run(params(&[]), &ctx()).await.unwrap();
    assert!(!out.success);
    assert!(out.content.contains("name"));
}

#[tokio::test]
async fn load_skill_unknown_returns_error() {
    let registry = make_registry().await;
    let handler = LoadSkillHandler::new(registry);
    let out = handler
        .run(params(&[("name", json!("nonexistent-skill"))]), &ctx())
        .await
        .unwrap();
    assert!(!out.success);
    assert!(out.content.contains("not found"));
}

#[tokio::test]
async fn load_skill_empty_name_returns_error() {
    let registry = make_registry().await;
    let handler = LoadSkillHandler::new(registry);
    let out = handler
        .run(params(&[("name", json!(""))]), &ctx())
        .await
        .unwrap();
    assert!(!out.success);
}

// ── list_skills ────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_skills_empty_registry_returns_friendly_message() {
    let registry = make_registry().await;
    let handler = ListSkillsHandler::new(registry);
    let out = handler.run(params(&[]), &ctx()).await.unwrap();
    assert!(out.success);
    assert!(out.content.contains("No skills"));
}

// ── memory_get ────────────────────────────────────────────────────────────

#[tokio::test]
async fn memory_get_missing_target_returns_error() {
    use assistant_core::types::agent::AssistantConfig;
    use assistant_tool_executor::builtins::memory_get::MemoryGetHandler;

    let config = Arc::new(AssistantConfig::default());
    let handler = MemoryGetHandler::new(config);
    let out = handler.run(params(&[]), &ctx()).await.unwrap();
    assert!(!out.success);
    assert!(out.content.contains("target"));
}

#[tokio::test]
async fn memory_get_unknown_target_returns_error() {
    use assistant_core::types::agent::AssistantConfig;
    use assistant_tool_executor::builtins::memory_get::MemoryGetHandler;

    let config = Arc::new(AssistantConfig::default());
    let handler = MemoryGetHandler::new(config);
    let out = handler
        .run(params(&[("target", json!("not-a-target"))]), &ctx())
        .await
        .unwrap();
    assert!(!out.success);
    assert!(out.content.contains("Unknown target"));
}

#[tokio::test]
async fn memory_get_invalid_notes_date_format_returns_error() {
    use assistant_core::types::agent::AssistantConfig;
    use assistant_tool_executor::builtins::memory_get::MemoryGetHandler;

    let config = Arc::new(AssistantConfig::default());
    let handler = MemoryGetHandler::new(config);
    let out = handler
        .run(params(&[("target", json!("notes/not-a-date"))]), &ctx())
        .await
        .unwrap();
    assert!(!out.success);
}
