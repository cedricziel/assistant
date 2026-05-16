use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};
use assistant_core::{LlmProvider, MessageBus, types::agent::AssistantConfig};
use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;
use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

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

/// Mount a mock that returns a final answer for every POST /api/chat.
async fn mount_answer(server: &MockServer, text: &str) {
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer(text)))
        .mount(server)
        .await;
}

/// Build an [`Orchestrator`] wired to `base_url` with a fresh in-memory DB.
async fn build(base_url: &str) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    build_with_config(base_url, config).await
}

async fn build_with_config(
    base_url: &str,
    config: AssistantConfig,
) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
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

/// Like [`build`], but also returns the [`ToolExecutor`] so tests can
/// register custom ambient tools before driving a turn.
async fn build_with_executor(
    base_url: &str,
) -> (Arc<Orchestrator>, Arc<StorageLayer>, Arc<ToolExecutor>) {
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

mod bus_worker;
mod end_turn;
mod misc;
mod multimodal;
mod sanitize;
mod streaming;
mod subagent;

// ── Tests ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn first_turn_sends_only_current_message() {
    let server = MockServer::start().await;
    mount_answer(&server, "pong").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    orch.run_turn(
        "hello",
        conv_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);

    let msgs = messages_in(&reqs[0]);
    assert_eq!(msgs.len(), 2, "expected [system, user], got {msgs:?}");
    assert_eq!(msgs[0]["role"], "system");
    assert_eq!(msgs[1]["role"], "user");
    assert_eq!(msgs[1]["content"], "hello");
}

#[tokio::test]
async fn second_turn_includes_prior_history() {
    let server = MockServer::start().await;
    mount_answer(&server, "pong").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    orch.run_turn(
        "first message",
        conv_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();
    orch.run_turn(
        "second message",
        conv_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 2);

    let msgs = messages_in(&reqs[1]);
    assert_eq!(msgs.len(), 4, "expected 4 messages on turn 2, got {msgs:?}");
    assert_eq!(msgs[0]["role"], "system");
    assert_eq!(msgs[1]["role"], "user");
    assert_eq!(msgs[1]["content"], "first message");
    assert_eq!(msgs[2]["role"], "assistant");
    assert_eq!(msgs[2]["content"], "pong");
    assert_eq!(msgs[3]["role"], "user");
    assert_eq!(msgs[3]["content"], "second message");
}

#[tokio::test]
async fn current_message_not_duplicated() {
    let server = MockServer::start().await;
    mount_answer(&server, "pong").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    orch.run_turn(
        "turn one",
        conv_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();
    orch.run_turn(
        "turn two",
        conv_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    let msgs = messages_in(reqs.last().unwrap());

    let count = msgs
        .iter()
        .filter(|m| m["role"] == "user" && m["content"] == "turn two")
        .count();
    assert_eq!(
        count, 1,
        "current message must appear exactly once; found {count}"
    );
}

#[tokio::test]
async fn seeded_history_included_in_llm_call() {
    let server = MockServer::start().await;
    mount_answer(&server, "pong").await;

    let (orch, storage) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    let conv_store = storage.conversation_store();
    conv_store
        .create_conversation_with_id(conv_id, Some("slack:C001:1234"))
        .await
        .unwrap();

    let mut seed_user =
        assistant_core::types::conversation::Message::user(conv_id, "seeded user message");
    seed_user.turn = 0;
    conv_store.save_message(&seed_user).await.unwrap();

    let mut seed_bot =
        assistant_core::types::conversation::Message::assistant(conv_id, "seeded bot reply");
    seed_bot.turn = 1;
    conv_store.save_message(&seed_bot).await.unwrap();

    orch.run_turn(
        "follow-up",
        conv_id,
        Interface::Slack,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);

    let msgs = messages_in(&reqs[0]);
    assert_eq!(msgs.len(), 4, "expected 4 messages, got {msgs:?}");
    assert_eq!(msgs[1]["role"], "user");
    assert_eq!(msgs[1]["content"], "seeded user message");
    assert_eq!(msgs[2]["role"], "assistant");
    assert_eq!(msgs[2]["content"], "seeded bot reply");
    assert_eq!(msgs[3]["role"], "user");
    assert_eq!(msgs[3]["content"], "follow-up");
}

#[tokio::test]
async fn three_turns_accumulate_history() {
    let server = MockServer::start().await;
    mount_answer(&server, "reply").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    orch.run_turn(
        "turn 1",
        conv_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();
    orch.run_turn(
        "turn 2",
        conv_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();
    orch.run_turn(
        "turn 3",
        conv_id,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 3);

    let msgs = messages_in(&reqs[2]);
    assert_eq!(msgs.len(), 6, "expected 6 messages on turn 3, got {msgs:?}");
    assert_eq!(msgs[1]["content"], "turn 1");
    assert_eq!(msgs[2]["content"], "reply");
    assert_eq!(msgs[3]["content"], "turn 2");
    assert_eq!(msgs[4]["content"], "reply");
    assert_eq!(msgs[5]["content"], "turn 3");
}

#[tokio::test]
async fn different_conversations_are_isolated() {
    let server = MockServer::start().await;
    mount_answer(&server, "pong").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_a = Uuid::new_v4();
    let conv_b = Uuid::new_v4();

    orch.run_turn(
        "conv-a message",
        conv_a,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();
    orch.run_turn(
        "conv-b message",
        conv_b,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();

    let msgs_b = messages_in(&reqs[1]);
    let bleed = msgs_b.iter().any(|m| m["content"] == "conv-a message");
    assert!(
        !bleed,
        "conv-a history must not appear in conv-b's LLM call"
    );
}

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

#[tokio::test]
async fn single_tool_call_adds_observation_to_next_request() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["unknown-skill"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let result = orch
        .run_turn(
            "go",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();
    assert_eq!(result.answer, "done");

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 2, "expected exactly 2 LLM calls");

    let msgs = messages_in(&reqs[1]);
    let has_obs = msgs.iter().any(|m| {
        m["role"] == "tool"
            && m["content"]
                .as_str()
                .unwrap_or("")
                .contains("unknown-skill")
    });
    assert!(
        has_obs,
        "second LLM call should contain the tool observation; msgs: {msgs:?}"
    );
}

#[tokio::test]
async fn two_tool_calls_handled_in_single_iteration() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["skill-a", "skill-b"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    orch.run_turn(
        "go",
        Uuid::new_v4(),
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        2,
        "two tool calls must be handled in ONE iteration — expected 2 LLM calls, got {}",
        reqs.len()
    );
}

#[tokio::test]
async fn two_tool_calls_both_observations_sent_to_llm() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["skill-a", "skill-b"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    orch.run_turn(
        "go",
        Uuid::new_v4(),
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    let msgs = messages_in(&reqs[1]);

    let tool_msgs: Vec<&Value> = msgs.iter().filter(|m| m["role"] == "tool").collect();
    assert_eq!(
        tool_msgs.len(),
        2,
        "expected 2 tool observation messages in second LLM call, got {}: {msgs:?}",
        tool_msgs.len()
    );

    let content_a = tool_msgs[0]["content"].as_str().unwrap_or("");
    let content_b = tool_msgs[1]["content"].as_str().unwrap_or("");
    assert!(
        content_a.contains("skill-a"),
        "first observation should mention skill-a; got: {content_a}"
    );
    assert!(
        content_b.contains("skill-b"),
        "second observation should mention skill-b; got: {content_b}"
    );
}

#[tokio::test]
async fn three_tool_calls_handled_in_single_iteration() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["s1", "s2", "s3"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    orch.run_turn(
        "go",
        Uuid::new_v4(),
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        2,
        "three tool calls must be handled in ONE iteration"
    );
}
