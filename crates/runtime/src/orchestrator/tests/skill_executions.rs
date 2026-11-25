//! Tool calls made while a skill is active are recorded as skill executions.

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::clock::{Clock, FakeClock};
use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};
use assistant_core::{
    LlmProvider, LlmResponse, LlmResponseMeta, MessageBus, ToolCallItem, ToolCallResponse,
    types::agent::OrgConfig,
};
use assistant_llm_provider::ScriptedLlmProvider;
use assistant_storage::{
    SkillExecutionStore, SkillOutcome, SkillStatsProvider, StorageLayer, registry::SkillRegistry,
};
use assistant_tool_executor::ToolExecutor;
use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::orchestrator::Orchestrator;

/// Returns `Ok(output)` with the given success flag, or `Err` when `hard_fail`.
struct ScriptedTool {
    name: &'static str,
    outcome: ToolBehaviour,
}

enum ToolBehaviour {
    Succeed,
    SoftFail(&'static str),
    HardFail(&'static str),
}

#[async_trait]
impl ToolHandler for ScriptedTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "scripted test tool"
    }
    fn params_schema(&self) -> Value {
        json!({"type": "object", "properties": {}, "required": []})
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        match self.outcome {
            ToolBehaviour::Succeed => Ok(ToolOutput::success("ok")),
            ToolBehaviour::SoftFail(msg) => Ok(ToolOutput::error(msg)),
            ToolBehaviour::HardFail(msg) => anyhow::bail!(msg),
        }
    }
}

fn call(tool: &str, params: Value) -> LlmResponse {
    LlmResponse::ToolCalls(ToolCallResponse {
        items: vec![ToolCallItem {
            name: tool.to_string(),
            params,
            id: None,
        }],
        meta: LlmResponseMeta::default(),
        thinking: None,
        provider_state: None,
    })
}

#[tokio::test]
async fn tool_calls_under_an_active_skill_are_recorded() {
    let llm: Arc<dyn LlmProvider> =
        Arc::new(ScriptedLlmProvider::new().with_canned_responses(vec![
            call("probe", json!({})), // before any skill: not recorded
            call("load-skill", json!({"name": "alpha"})),
            call("probe", json!({})),
            call("soft-fail", json!({})),
            call("hard-fail", json!({})),
            LlmResponse::FinalAnswer("done".to_string(), LlmResponseMeta::default()),
        ]));

    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.permissions.allow = vec!["*".parse().expect("valid rule")];

    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    for (name, outcome) in [
        ("probe", ToolBehaviour::Succeed),
        ("load-skill", ToolBehaviour::Succeed),
        ("soft-fail", ToolBehaviour::SoftFail("bad input")),
        ("hard-fail", ToolBehaviour::HardFail("exploded")),
    ] {
        executor.register_ambient_tool(Arc::new(ScriptedTool { name, outcome }));
    }

    let clock = FakeClock::new_at_seed();
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Orchestrator::new(llm, storage.clone(), executor, registry, bus, &config)
        .with_clock(Arc::new(clock.clone()));

    let conversation_id = Uuid::new_v4();
    orch.run_turn(
        "go",
        conversation_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let store = storage.skill_execution_store();
    let recorded = store.recent("alpha", 10).await.unwrap();

    let summary: Vec<(&str, SkillOutcome, Option<&str>)> = recorded
        .iter()
        .map(|e| (e.tool_name.as_str(), e.outcome, e.error.as_deref()))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("hard-fail", SkillOutcome::Error, Some("exploded")),
            ("soft-fail", SkillOutcome::Error, Some("bad input")),
            ("probe", SkillOutcome::Success, None),
        ],
        "only calls made after load-skill are recorded, newest first"
    );
    assert!(
        recorded.iter().all(|e| e.conversation_id == conversation_id
            && e.agent_id == config.agent.id
            && e.created_at == clock.now()),
        "rows carry the conversation, persona and clock time"
    );

    let stats = store.stats_for_active_skill("alpha", 10).await.unwrap();
    assert_eq!((stats.total, stats.error_count), (3, 2));
}
