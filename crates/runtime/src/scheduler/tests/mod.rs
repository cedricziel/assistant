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

use assistant_core::{LlmProvider, MessageBus, types::agent::OrgConfig};
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
mod loop_body;
mod prune;
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
    let mut config = OrgConfig::default();
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

/// A Persona directory in a temporary install root, removed when dropped.
pub(crate) struct AgentDirGuard {
    _install_root: tempfile::TempDir,
    dir: std::path::PathBuf,
}

impl std::ops::Deref for AgentDirGuard {
    type Target = std::path::Path;
    fn deref(&self) -> &Self::Target {
        &self.dir
    }
}

/// Build an orchestrator whose Persona lives in a temporary install root, so
/// heartbeat-style tests can stage `HEARTBEAT.md` there.
pub(crate) async fn build_with_agent_dir(
    base_url: &str,
) -> (Arc<Orchestrator>, Arc<StorageLayer>, AgentDirGuard, String) {
    let agent_id = Uuid::new_v4().to_string();
    let install_root = tempfile::tempdir().unwrap();
    let agent_dir = assistant_core::context::agent_base_dir(install_root.path(), &agent_id);
    std::fs::create_dir_all(&agent_dir).unwrap();

    let mut config = OrgConfig::default();
    assistant_core::apply_agent_context(&mut config, install_root.path(), &agent_id);
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
    let bus_arc: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Arc::new(Orchestrator::new(
        llm,
        storage.clone(),
        executor,
        registry,
        bus_arc,
        &config,
    ));
    let guard = AgentDirGuard {
        _install_root: install_root,
        dir: agent_dir,
    };
    (orch, storage, guard, agent_id)
}
