//! A turn is interactive only when its submitter says so, and it carries the
//! run id (the bus `batch_id` the submitter chose) into the tools it runs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_core::tool::{ToolHandler, ToolOutput, TurnScope, TurnToolSource};
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_core::{
    LlmProvider, LlmResponse, LlmResponseMeta, MessageBus, ToolCallItem, ToolCallResponse,
};
use assistant_llm_provider::scripted::ScriptedLlmProvider;
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;

use super::super::{Orchestrator, SubmitOptions};

/// What a tool saw in its [`ExecutionContext`].
type Seen = Arc<Mutex<Vec<(bool, Option<Uuid>)>>>;

struct CtxProbe {
    seen: Seen,
}

#[async_trait]
impl ToolHandler for CtxProbe {
    fn name(&self) -> &str {
        "mcp__user_probe__ctx"
    }
    fn description(&self) -> &str {
        "records its execution context"
    }
    fn params_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    fn is_mutating(&self) -> bool {
        true
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        if let Ok(mut seen) = self.seen.lock() {
            seen.push((ctx.interactive, ctx.run_id));
        }
        Ok(ToolOutput::success("ok"))
    }
}

struct ProbeSource {
    seen: Seen,
}

#[async_trait]
impl TurnToolSource for ProbeSource {
    async fn tools_for_turn(&self, _scope: &TurnScope<'_>) -> Vec<Arc<dyn ToolHandler>> {
        vec![Arc::new(CtxProbe {
            seen: self.seen.clone(),
        })]
    }
}

fn probe_call() -> LlmResponse {
    LlmResponse::ToolCalls(ToolCallResponse {
        items: vec![ToolCallItem {
            name: "mcp__user_probe__ctx".into(),
            params: json!({}),
            id: None,
        }],
        meta: LlmResponseMeta::default(),
        thinking: None,
        provider_state: None,
    })
}

/// An orchestrator with a running worker whose one turn calls the probe once.
async fn rig() -> (Arc<Orchestrator>, Seen) {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.permissions.allow = vec!["mcp__user_probe__ctx".parse().unwrap()];
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> =
        Arc::new(ScriptedLlmProvider::new().with_canned_responses(vec![
            probe_call(),
            LlmResponse::FinalAnswer("done".into(), LlmResponseMeta::default()),
        ]));
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let seen = Seen::default();
    let orch = Arc::new(
        Orchestrator::new(llm, storage, executor.clone(), registry, bus, &config)
            .with_turn_tool_source(Arc::new(ProbeSource { seen: seen.clone() }), "acme", "eng")
            .with_submit_timeout(30),
    );
    executor.set_subagent_runner(orch.clone());
    let worker = orch.clone();
    tokio::spawn(async move { worker.run_worker("options-worker").await });
    (orch, seen)
}

fn only(seen: &Seen) -> (bool, Option<Uuid>) {
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "the probe runs once per turn: {seen:?}");
    seen[0]
}

#[tokio::test]
async fn a_web_turn_with_default_options_is_not_interactive() {
    let (orch, seen) = rig().await;
    let request_id = Uuid::new_v4();
    orch.submit_turn_with_request_id(
        &AuthContext::system(),
        request_id,
        "go",
        Uuid::new_v4(),
        Interface::Web,
        None,
        vec![],
        SubmitOptions::default(),
    )
    .await
    .unwrap();
    assert_eq!(only(&seen), (false, Some(request_id)));
}

#[tokio::test]
async fn an_interactive_submit_reaches_the_tool_with_its_run_id() {
    let (orch, seen) = rig().await;
    let request_id = Uuid::new_v4();
    orch.submit_turn_with_request_id(
        &AuthContext::system(),
        request_id,
        "go",
        Uuid::new_v4(),
        Interface::Web,
        None,
        vec![],
        SubmitOptions { interactive: true },
    )
    .await
    .unwrap();
    assert_eq!(only(&seen), (true, Some(request_id)));
}

#[tokio::test]
async fn interactivity_follows_the_flag_not_the_interface() {
    let (orch, seen) = rig().await;
    orch.submit_turn(
        &AuthContext::system(),
        "go",
        Uuid::new_v4(),
        Interface::Cli,
        None,
        SubmitOptions::default(),
    )
    .await
    .unwrap();
    assert!(!only(&seen).0, "a CLI turn is not interactive by itself");
}
