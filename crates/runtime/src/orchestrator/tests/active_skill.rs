//! The skill loaded in one turn must not be visible to a concurrent turn.
//!
//! The post-turn skill learner skips turns that had an active skill, so the
//! turns the judge is asked about reveal whose skill each turn saw.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};
use assistant_core::{
    ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse, LlmResponseMeta, MessageBus,
    StreamChunk, ToolCallItem, ToolCallResponse, ToolSpec, types::agent::AssistantConfig,
};
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::{Notify, mpsc};
use uuid::Uuid;

use crate::orchestrator::Orchestrator;

const JUDGE_MARKER: &str = "skill extraction engine";

/// Plays one tool call per step for each turn (identified by the tag in its
/// user message), then a final answer. Records which turns the skill judge
/// was asked about.
struct TaggedLlm {
    scripts: HashMap<&'static str, Vec<&'static str>>,
    judged: Arc<Mutex<Vec<String>>>,
}

impl TaggedLlm {
    fn respond(&self, system_prompt: &str, history: &[ChatHistoryMessage]) -> LlmResponse {
        if system_prompt.contains(JUDGE_MARKER) {
            let transcript = history
                .iter()
                .filter_map(|m| match m {
                    ChatHistoryMessage::Text { content, .. } => Some(content.as_str()),
                    _ => None,
                })
                .collect::<String>();
            let tag = self
                .scripts
                .keys()
                .find(|tag| transcript.contains(*tag))
                .copied()
                .unwrap_or("unknown");
            self.judged.lock().unwrap().push(tag.to_string());
            return LlmResponse::FinalAnswer(
                json!({"create": false, "reason": "test"}).to_string(),
                LlmResponseMeta::default(),
            );
        }

        let first_user_message = history.iter().find_map(|m| match m {
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content,
            } => Some(content.as_str()),
            _ => None,
        });
        let tag = self
            .scripts
            .keys()
            .find(|tag| first_user_message.is_some_and(|m| m.contains(*tag)))
            .copied()
            .expect("every turn message carries a known tag");
        let step = history
            .iter()
            .filter(|m| matches!(m, ChatHistoryMessage::ToolResult { .. }))
            .count();

        match self.scripts[tag].get(step) {
            Some(tool) => LlmResponse::ToolCalls(ToolCallResponse {
                items: vec![ToolCallItem {
                    name: (*tool).to_string(),
                    params: json!({"name": "alpha"}),
                    id: None,
                }],
                meta: LlmResponseMeta::default(),
                thinking: None,
            }),
            None => LlmResponse::FinalAnswer("done".to_string(), LlmResponseMeta::default()),
        }
    }
}

#[async_trait]
impl LlmProvider for TaggedLlm {
    fn capabilities(&self) -> assistant_core::Capabilities {
        assistant_core::Capabilities {
            tools: assistant_core::ToolSupport::Native,
            streaming: false,
            vision: false,
            hosted_tools: Vec::new(),
            context_window_tokens: None,
        }
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        _tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        Ok(self.respond(system_prompt, history))
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        _tools: &[ToolSpec],
        _chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        Ok(self.respond(system_prompt, history))
    }
}

/// Succeeds immediately. Registered under the names the turns call.
struct InstantTool(&'static str);

#[async_trait]
impl ToolHandler for InstantTool {
    fn name(&self) -> &str {
        self.0
    }
    fn description(&self) -> &str {
        "instant test tool"
    }
    fn params_schema(&self) -> Value {
        json!({"type": "object", "properties": {}, "required": []})
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        Ok(ToolOutput::success("ok"))
    }
}

/// Parks its turn mid-flight until the test releases it.
struct HoldTool {
    name: &'static str,
    entered: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait]
impl ToolHandler for HoldTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "blocks until released"
    }
    fn params_schema(&self) -> Value {
        json!({"type": "object", "properties": {}, "required": []})
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(ToolOutput::success("released"))
    }
}

struct Harness {
    orch: Arc<Orchestrator>,
    judged: Arc<Mutex<Vec<String>>>,
    held: Arc<Notify>,
    release: Arc<Notify>,
}

async fn harness(scripts: HashMap<&'static str, Vec<&'static str>>) -> Harness {
    let judged = Arc::new(Mutex::new(Vec::new()));
    let llm: Arc<dyn LlmProvider> = Arc::new(TaggedLlm {
        scripts,
        judged: judged.clone(),
    });

    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    config.permissions.allow = vec!["*".parse().expect("valid rule")];
    config.learning.enabled = true;
    config.learning.auto_create_skills = true;
    config.learning.min_tool_calls_for_skill = 1;

    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let held = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    executor.register_ambient_tool(Arc::new(InstantTool("load-skill")));
    executor.register_ambient_tool(Arc::new(InstantTool("probe")));
    executor.register_ambient_tool(Arc::new(HoldTool {
        name: "hold",
        entered: held.clone(),
        release: release.clone(),
    }));

    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Arc::new(Orchestrator::new(
        llm, storage, executor, registry, bus, &config,
    ));
    Harness {
        orch,
        judged,
        held,
        release,
    }
}

fn spawn_turn(orch: &Arc<Orchestrator>, tag: &'static str) -> tokio::task::JoinHandle<()> {
    let orch = orch.clone();
    tokio::spawn(async move {
        orch.run_turn(
            tag,
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();
    })
}

/// Lets fire-and-forget post-turn evaluations run, then returns who was judged.
async fn judged_turns(h: &Harness) -> Vec<String> {
    tokio::time::sleep(Duration::from_millis(300)).await;
    let mut judged = h.judged.lock().unwrap().clone();
    judged.sort();
    judged
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_new_turn_does_not_clear_the_skill_of_a_running_turn() {
    let h = harness(HashMap::from([
        ("turn-a", vec!["load-skill", "hold"]),
        ("turn-b", vec!["probe"]),
    ]))
    .await;

    let turn_a = spawn_turn(&h.orch, "turn-a");
    h.held.notified().await;
    spawn_turn(&h.orch, "turn-b").await.unwrap();
    h.release.notify_one();
    turn_a.await.unwrap();

    assert_eq!(
        judged_turns(&h).await,
        vec!["turn-b".to_string()],
        "turn-a loaded a skill, so only turn-b may be offered to the skill judge"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_skill_loaded_by_one_turn_is_not_seen_by_a_running_turn() {
    let h = harness(HashMap::from([
        ("turn-a", vec!["load-skill", "probe"]),
        ("turn-b", vec!["hold"]),
    ]))
    .await;

    let turn_b = spawn_turn(&h.orch, "turn-b");
    h.held.notified().await;
    spawn_turn(&h.orch, "turn-a").await.unwrap();
    h.release.notify_one();
    turn_b.await.unwrap();

    assert_eq!(
        judged_turns(&h).await,
        vec!["turn-b".to_string()],
        "turn-b never loaded a skill, so it must still be offered to the skill judge"
    );
}
