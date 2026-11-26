use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::conversation::ExecutionContext;
use assistant_core::{LlmProvider, MessageBus, types::agent::OrgConfig};
use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;
use async_trait::async_trait;
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::Orchestrator;

// ── Helpers ───────────────────────────────────────────────────────────────

/// Minimal Ollama final-answer response.
fn ollama_answer(text: &str) -> Value {
    json!({
        "model": "test",
        "message": { "role": "assistant", "content": text },
        "done": true
    })
}

/// Ollama response with empty content but non-empty thinking — triggers
/// `LlmResponse::Thinking` in the LLM client parser.
fn ollama_thinking(thought: &str) -> Value {
    json!({
        "model": "test",
        "message": { "role": "assistant", "content": "", "thinking": thought },
        "done": true
    })
}

/// Build an Ollama tool-call response with no arguments per call.
fn ollama_tool_calls(names: &[&str]) -> Value {
    ollama_tool_calls_with_args(&names.iter().map(|n| (*n, json!({}))).collect::<Vec<_>>())
}

/// Build a tool-call Ollama response where each entry is `(name, arguments)`.
fn ollama_tool_calls_with_args(calls: &[(&str, Value)]) -> Value {
    let tc: Vec<Value> = calls
        .iter()
        .map(|(n, a)| json!({ "function": { "name": n, "arguments": a } }))
        .collect();
    json!({
        "model": "test",
        "message": { "role": "assistant", "content": null, "tool_calls": tc },
        "done": true
    })
}

/// Mount a mock that returns a final answer for every POST /api/chat.
async fn mount_answer(server: &MockServer, text: &str) {
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer(text)))
        .mount(server)
        .await;
}

/// Orchestrator plumbing tests drive arbitrary mock tools; the permission
/// policy itself is covered in `permission.rs`. Built-in hard denies still
/// apply.
fn allow_all_tools(config: &mut OrgConfig) {
    config.permissions.allow = vec!["*".parse().expect("valid rule")];
}

/// Build an [`Orchestrator`] wired to `base_url` with a fresh in-memory DB.
async fn build(base_url: &str) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    allow_all_tools(&mut config);
    build_with_config(base_url, config).await
}

async fn build_with_config(
    base_url: &str,
    config: OrgConfig,
) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
    build_customized(base_url, config, |o| o).await
}

/// Like [`build_with_config`], but lets the test adjust the orchestrator
/// (e.g. attach a confirmation callback) before it is shared.
async fn build_customized(
    base_url: &str,
    config: OrgConfig,
    customize: impl FnOnce(Orchestrator) -> Orchestrator,
) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: base_url.to_string(),
            timeout_secs: 10,
            retry_config: assistant_llm_provider::retry::RetryConfig::disabled(),
        })
        .unwrap(),
    );
    build_with_llm(llm, config, customize).await
}

/// Like [`build_customized`], with a caller-supplied provider instead of an
/// Ollama client pointed at a mock server.
async fn build_with_llm(
    llm: Arc<dyn LlmProvider>,
    config: OrgConfig,
    customize: impl FnOnce(Orchestrator) -> Orchestrator,
) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Arc::new(customize(Orchestrator::new(
        llm,
        storage.clone(),
        executor.clone(),
        registry.clone(),
        bus,
        &config,
    )));
    executor.set_subagent_runner(orch.clone());
    (orch, storage)
}

/// Like [`build`], but also returns the [`ToolExecutor`] so tests can
/// register custom ambient tools before driving a turn.
async fn build_with_executor(
    base_url: &str,
) -> (Arc<Orchestrator>, Arc<StorageLayer>, Arc<ToolExecutor>) {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    allow_all_tools(&mut config);
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
    (orch, storage, executor)
}

/// Extract the `messages` array from an intercepted Ollama request body.
fn messages_in(req: &wiremock::Request) -> Vec<Value> {
    let body: Value = serde_json::from_slice(&req.body).unwrap();
    body["messages"].as_array().cloned().unwrap_or_default()
}

/// A fake extension tool that records how many times it was called.
/// Shared with sibling test modules.
struct MockExtTool {
    tool_name: &'static str,
    call_count: AtomicUsize,
}

impl MockExtTool {
    fn new(name: &'static str) -> Self {
        Self {
            tool_name: name,
            call_count: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ToolHandler for MockExtTool {
    fn name(&self) -> &str {
        self.tool_name
    }

    fn description(&self) -> &str {
        "mock extension tool"
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" }
            },
            "required": []
        })
    }

    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(ToolOutput::success("ok"))
    }
}

mod active_skill;
mod bus_worker;
mod call_context;
mod chat_options;
mod dispatch;
mod end_turn;
mod final_answer_meta;
mod history;
mod misc;
mod model_override;
mod multimodal;
mod permission;
mod sanitize;
mod skill_executions;
mod streaming;
mod subagent;
mod summary_history;
mod surfaces;
mod tool_data;
mod turn_options;
mod turn_timing;
mod turn_tools;
