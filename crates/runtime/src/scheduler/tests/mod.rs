//! Shared test fixtures and submodule declarations for `scheduler::tests`.
//!
//! Per-domain test files live alongside this `mod.rs`:
//!
//! - `cron`         — `compute_next_run` parser
//! - `dispatch`     — `run_due_tasks` (cron-driven dispatch)
//! - `heartbeat`    — `run_heartbeat` (HEARTBEAT.md publisher)
//! - `home_channel` — `resolve_home_channel_tools`
//! - `identity`     — persona-owner identity threading
//! - `reap`         — `reap_stale_and_recover` (crash recovery)

#![cfg(test)]

use std::sync::Arc;

use assistant_core::{LlmProvider, MessageBus, types::agent::AssistantConfig};
use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
use assistant_storage::StorageLayer;
use assistant_storage::registry::SkillRegistry;
use assistant_tool_executor::ToolExecutor;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::orchestrator::Orchestrator;

mod cron;
mod dispatch;
mod heartbeat;
mod home_channel;
mod identity;
mod reap;

// ── LLM mocking ────────────────────────────────────────────────────────────

pub(crate) fn ollama_answer(text: &str) -> serde_json::Value {
    serde_json::json!({
        "model": "test",
        "message": { "role": "assistant", "content": text },
        "done": true
    })
}

pub(crate) async fn mount_answer(server: &MockServer, text: &str) {
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer(text)))
        .mount(server)
        .await;
}

// ── Orchestrator fixtures ──────────────────────────────────────────────────

pub(crate) async fn build(base_url: &str) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: base_url.to_string(),
            timeout_secs: 10,
            retry_config: assistant_llm_provider::retry::RetryConfig::disabled(),
        })
        .unwrap(),
    );
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Arc::new(Orchestrator::new(
        llm,
        storage.clone(),
        executor.clone(),
        registry.clone(),
        bus,
        &config,
    ));
    executor.set_subagent_runner(orch.clone());
    (orch, storage)
}

/// RAII guard that removes the agent directory when dropped.
pub(crate) struct AgentDirGuard(pub(crate) std::path::PathBuf);

impl Drop for AgentDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for AgentDirGuard {
    type Target = std::path::Path;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Build an orchestrator whose `agent_id` maps to a unique on-disk agent
/// dir so heartbeat-style tests can stage `HEARTBEAT.md` without touching
/// `~/.assistant`.  The returned `AgentDirGuard` removes the directory
/// when it's dropped.
pub(crate) async fn build_with_agent_dir(
    base_url: &str,
) -> (Arc<Orchestrator>, Arc<StorageLayer>, AgentDirGuard, String) {
    let agent_id = Uuid::new_v4().to_string();
    let agent_dir = assistant_core::context::agent_base_dir(&agent_id);
    std::fs::create_dir_all(&agent_dir).unwrap();

    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    config.agent.id = agent_id.clone();

    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: base_url.to_string(),
            timeout_secs: 10,
            retry_config: assistant_llm_provider::retry::RetryConfig::disabled(),
        })
        .unwrap(),
    );
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus_arc: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Arc::new(Orchestrator::new(
        llm,
        storage.clone(),
        executor,
        registry,
        bus_arc,
        &config,
    ));
    (orch, storage, AgentDirGuard(agent_dir), agent_id)
}
