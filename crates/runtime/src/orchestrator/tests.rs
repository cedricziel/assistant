use std::sync::Arc;

use std::time::Duration;

use assistant_core::{
    bus_messages, topic, types::Interface, AssistantConfig, MessageBus, PublishRequest,
};
use assistant_llm::{
    ChatHistoryMessage, ChatRole, LlmClient, LlmClientConfig, LlmProvider, ToolCallItem,
};
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use assistant_tool_executor::ToolExecutor;
use serde_json::{json, Value};
use uuid::Uuid;
use wiremock::{
    matchers::{body_string_contains, method, path},
    Mock, MockServer, ResponseTemplate,
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
            retry_config: assistant_llm::RetryConfig::disabled(),
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

/// Extract the `messages` array from an intercepted Ollama request body.
fn messages_in(req: &wiremock::Request) -> Vec<Value> {
    let body: Value = serde_json::from_slice(&req.body).unwrap();
    body["messages"].as_array().cloned().unwrap_or_default()
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn first_turn_sends_only_current_message() {
    let server = MockServer::start().await;
    mount_answer(&server, "pong").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    orch.run_turn("hello", conv_id, Interface::Cli, None)
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

    orch.run_turn("first message", conv_id, Interface::Cli, None)
        .await
        .unwrap();
    orch.run_turn("second message", conv_id, Interface::Cli, None)
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

    orch.run_turn("turn one", conv_id, Interface::Cli, None)
        .await
        .unwrap();
    orch.run_turn("turn two", conv_id, Interface::Cli, None)
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

    let mut seed_user = assistant_core::Message::user(conv_id, "seeded user message");
    seed_user.turn = 0;
    conv_store.save_message(&seed_user).await.unwrap();

    let mut seed_bot = assistant_core::Message::assistant(conv_id, "seeded bot reply");
    seed_bot.turn = 1;
    conv_store.save_message(&seed_bot).await.unwrap();

    orch.run_turn("follow-up", conv_id, Interface::Slack, None)
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

    orch.run_turn("turn 1", conv_id, Interface::Cli, None)
        .await
        .unwrap();
    orch.run_turn("turn 2", conv_id, Interface::Cli, None)
        .await
        .unwrap();
    orch.run_turn("turn 3", conv_id, Interface::Cli, None)
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

    orch.run_turn("conv-a message", conv_a, Interface::Cli, None)
        .await
        .unwrap();
    orch.run_turn("conv-b message", conv_b, Interface::Cli, None)
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
        .run_turn("go", Uuid::new_v4(), Interface::Cli, None)
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
    orch.run_turn("go", Uuid::new_v4(), Interface::Cli, None)
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
    orch.run_turn("go", Uuid::new_v4(), Interface::Cli, None)
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
    orch.run_turn("go", Uuid::new_v4(), Interface::Cli, None)
        .await
        .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        2,
        "three tool calls must be handled in ONE iteration"
    );
}

// ── Mock extension handlers ─────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::ExecutionContext;
use async_trait::async_trait;

/// A fake extension tool that records how many times it was called.
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

/// A fake reply extension tool whose `params_schema` has `"required": ["text"]`
/// so auto-post picks it up.  Records every `text` value it receives.
struct MockReplyExtTool {
    call_count: AtomicUsize,
    texts: tokio::sync::Mutex<Vec<String>>,
}

impl MockReplyExtTool {
    fn new() -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            texts: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ToolHandler for MockReplyExtTool {
    fn name(&self) -> &str {
        "reply"
    }

    fn description(&self) -> &str {
        "mock reply extension tool"
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" }
            },
            "required": ["text"]
        })
    }

    async fn run(
        &self,
        params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        if let Some(Value::String(t)) = params.get("text") {
            self.texts.lock().await.push(t.clone());
        }
        Ok(ToolOutput::success("ok"))
    }
}

// ── end_turn rejection tests ──────────────────────────────────────────────

#[tokio::test]
async fn end_turn_rejected_when_reply_tool_exists_but_not_called() {
    let server = MockServer::start().await;

    // 1st LLM call: model calls end_turn without calling reply first.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[(
                "end_turn",
                json!({"reason": "replied"}),
            )])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 2nd LLM call: after rejection, model calls reply then end_turn.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[
                ("reply", json!({"text": "hello!"})),
                ("end_turn", json!({"reason": "replied"})),
            ])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let reply_handler = Arc::new(MockExtTool::new("reply"));

    orch.run_turn_with_tools(
        "hi",
        Uuid::new_v4(),
        Interface::Slack,
        vec![reply_handler.clone() as Arc<dyn ToolHandler>],
        None,
        vec![],
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        2,
        "expected 2 LLM calls: first end_turn rejected, second with reply"
    );

    // The rejection message should appear in the second LLM call.
    let msgs = messages_in(&reqs[1]);
    let has_rejection = msgs.iter().any(|m| {
        m["role"] == "tool"
            && m["content"]
                .as_str()
                .unwrap_or("")
                .contains("end_turn rejected")
    });
    assert!(
        has_rejection,
        "second LLM call must contain the end_turn rejection; msgs: {msgs:?}"
    );

    assert_eq!(
        reply_handler.calls(),
        1,
        "reply handler must have been called exactly once"
    );
}

#[tokio::test]
async fn end_turn_accepted_without_reply_tool_in_cli_mode() {
    let server = MockServer::start().await;

    // Model calls end_turn — no reply extension tool exists (CLI mode).
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[(
                "end_turn",
                json!({"reason": "done"}),
            )])),
        )
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;

    // No extension tools — CLI mode, end_turn should be accepted.
    orch.run_turn_with_tools("hi", Uuid::new_v4(), Interface::Cli, vec![], None, vec![])
        .await
        .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        1,
        "end_turn without reply tools should be accepted in a single LLM call"
    );
}

#[tokio::test]
async fn end_turn_accepted_after_reply_tool_called() {
    let server = MockServer::start().await;

    // Model calls reply first, then end_turn — should be accepted immediately.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[
                ("reply", json!({"text": "hello!"})),
                ("end_turn", json!({"reason": "replied"})),
            ])),
        )
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let reply_handler = Arc::new(MockExtTool::new("reply"));

    orch.run_turn_with_tools(
        "hi",
        Uuid::new_v4(),
        Interface::Slack,
        vec![reply_handler.clone() as Arc<dyn ToolHandler>],
        None,
        vec![],
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        1,
        "reply + end_turn in same call should complete in a single LLM call"
    );

    assert_eq!(reply_handler.calls(), 1, "reply must have been called once");
}

#[tokio::test]
async fn end_turn_accepted_after_react_tool_called() {
    let server = MockServer::start().await;

    // Model calls react then end_turn — reaction is a valid acknowledgement.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[
                ("react", json!({"emoji": "thumbsup"})),
                ("end_turn", json!({"reason": "acknowledged with reaction"})),
            ])),
        )
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let reply_handler = Arc::new(MockExtTool::new("reply"));
    let react_handler = Arc::new(MockExtTool::new("react"));

    orch.run_turn_with_tools(
        "thanks!",
        Uuid::new_v4(),
        Interface::Slack,
        vec![
            reply_handler.clone() as Arc<dyn ToolHandler>,
            react_handler.clone() as Arc<dyn ToolHandler>,
        ],
        None,
        vec![],
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        1,
        "react + end_turn should complete in a single LLM call"
    );

    assert_eq!(react_handler.calls(), 1, "react must have been called once");
    assert_eq!(reply_handler.calls(), 0, "reply must not have been called");
}

// ── empty FinalAnswer history-poisoning tests ──────────────────────────────

#[tokio::test]
async fn empty_final_answer_not_persisted_and_retries() {
    // Scenario: LLM returns a tool call, then an empty FinalAnswer, then a
    // real answer.  The empty FinalAnswer must NOT be saved to the DB, and
    // the loop must retry until a non-empty answer is produced.
    let server = MockServer::start().await;

    // 1st LLM call: model calls a builtin tool (will get an error observation
    //   because "some-tool" is unknown, but that's fine — we just need a
    //   tool-call iteration to precede the empty answer).
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["some-tool"])))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 2nd LLM call: model returns an empty FinalAnswer — should be retried.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("")))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 3rd LLM call: model returns a non-empty FinalAnswer — should be
    //   persisted and auto-posted via the reply tool.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_answer("here is your answer")),
        )
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();
    let reply_handler = Arc::new(MockReplyExtTool::new());

    orch.run_turn_with_tools(
        "hi",
        conv_id,
        Interface::Slack,
        vec![reply_handler.clone() as Arc<dyn ToolHandler>],
        None,
        vec![],
    )
    .await
    .unwrap();

    // Verify: 3 LLM calls (tool call → empty answer retry → real answer).
    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        3,
        "expected 3 LLM calls: tool-call, empty-answer retry, real answer; got {}",
        reqs.len()
    );

    // Verify: reply handler was called exactly once with the real answer.
    assert_eq!(
        reply_handler.calls(),
        1,
        "reply handler must be called once for the non-empty answer"
    );
    let texts = reply_handler.texts.lock().await;
    assert_eq!(
        texts[0], "here is your answer",
        "reply handler must receive the non-empty answer text"
    );
    drop(texts);

    // Verify: no empty assistant *text* messages in the DB.
    // (Tool-call messages legitimately have empty content + tool_calls_json.)
    let conv_store = storage.conversation_store();
    let history = conv_store.load_history(conv_id).await.unwrap();
    let empty_text_assistant_msgs: Vec<_> = history
        .iter()
        .filter(|m| {
            m.role == assistant_core::types::MessageRole::Assistant
                && m.content.trim().is_empty()
                && m.tool_calls_json.is_none()
        })
        .collect();
    assert!(
        empty_text_assistant_msgs.is_empty(),
        "no empty FinalAnswer assistant messages should be persisted; found {} in DB",
        empty_text_assistant_msgs.len()
    );

    // Verify: the non-empty answer IS persisted.
    let assistant_msgs: Vec<_> = history
        .iter()
        .filter(|m| m.role == assistant_core::types::MessageRole::Assistant)
        .collect();
    assert!(
        assistant_msgs
            .iter()
            .any(|m| m.content == "here is your answer"),
        "the non-empty answer must be persisted in the DB; assistant msgs: {assistant_msgs:?}"
    );
}

#[tokio::test]
async fn empty_final_answer_not_persisted_in_run_turn() {
    // Verify the same protection in the simpler `run_turn` path (CLI mode).
    let server = MockServer::start().await;
    mount_answer(&server, "").await;

    let (orch, storage) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    let result = orch
        .run_turn("hello", conv_id, Interface::Cli, None)
        .await
        .unwrap();

    // run_turn still returns the (empty) answer to the caller...
    assert_eq!(result.answer, "");

    // ...but must NOT have persisted it to the DB.
    let conv_store = storage.conversation_store();
    let history = conv_store.load_history(conv_id).await.unwrap();
    let empty_assistant_msgs: Vec<_> = history
        .iter()
        .filter(|m| {
            m.role == assistant_core::types::MessageRole::Assistant && m.content.trim().is_empty()
        })
        .collect();
    assert!(
        empty_assistant_msgs.is_empty(),
        "empty assistant message must not be persisted in run_turn; found {} in DB",
        empty_assistant_msgs.len()
    );
}

// ── MultimodalUser / OTel serialisation tests ────────────────────────────

#[test]
fn serialize_history_multimodal_user_omits_base64_data() {
    use crate::otel_spans::serialize_history_for_span;
    use assistant_llm::ContentBlock;

    let history = vec![ChatHistoryMessage::MultimodalUser {
        content: vec![
            ContentBlock::Text("describe this".to_string()),
            ContentBlock::Image {
                media_type: "image/png".to_string(),
                data: "A".repeat(10_000), // large base64 payload
            },
        ],
    }];

    let json_str = serialize_history_for_span(&history);
    assert!(
        !json_str.contains(&"A".repeat(100)),
        "base64 data must NOT appear in span output"
    );
    assert!(
        json_str.contains("image/png"),
        "media_type should be present"
    );
    assert!(
        json_str.contains("size_base64_chars"),
        "size_base64_chars field should be present"
    );
}

#[tokio::test]
async fn prepare_history_with_attachments_emits_multimodal_user() {
    use assistant_llm::ContentBlock;

    let server = MockServer::start().await;
    mount_answer(&server, "ok").await;
    let (orch, _) = build(&server.uri()).await;

    let conv_id = Uuid::new_v4();
    let attachments = vec![ContentBlock::Image {
        media_type: "image/jpeg".to_string(),
        data: "base64data".to_string(),
    }];

    let (_conv_store, history, _turn) = orch
        .prepare_history("look at this", conv_id, attachments, &[], &orch.agent_id)
        .await
        .unwrap();

    // The last message in history should be MultimodalUser.
    let last = history.last().expect("history non-empty");
    match last {
        ChatHistoryMessage::MultimodalUser { content } => {
            assert_eq!(content.len(), 2, "text block + image block");
            assert!(
                matches!(&content[0], ContentBlock::Text(t) if t == "look at this"),
                "first block should be the text"
            );
            assert!(
                matches!(&content[1], ContentBlock::Image { media_type, .. } if media_type == "image/jpeg"),
                "second block should be the image"
            );
        }
        other => panic!("expected MultimodalUser, got {:?}", other),
    }
}

#[tokio::test]
async fn prepare_history_without_attachments_emits_plain_text() {
    let server = MockServer::start().await;
    mount_answer(&server, "ok").await;
    let (orch, _) = build(&server.uri()).await;

    let conv_id = Uuid::new_v4();
    let (_conv_store, history, _turn) = orch
        .prepare_history("hello", conv_id, Vec::new(), &[], &orch.agent_id)
        .await
        .unwrap();

    let last = history.last().expect("history non-empty");
    match last {
        ChatHistoryMessage::Text { role, content } => {
            assert_eq!(*role, assistant_llm::ChatRole::User);
            assert_eq!(content, "hello");
        }
        other => panic!("expected Text, got {:?}", other),
    }
}

// ── Attachment collection tests ──────────────────────────────────────────

/// A fake tool handler that returns attachments in its output.
struct MockAttachmentTool {
    attachments: Vec<assistant_core::Attachment>,
}

impl MockAttachmentTool {
    fn new(attachments: Vec<assistant_core::Attachment>) -> Self {
        Self { attachments }
    }
}

#[async_trait]
impl ToolHandler for MockAttachmentTool {
    fn name(&self) -> &str {
        "attachment-tool"
    }

    fn description(&self) -> &str {
        "returns attachments for testing"
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        Ok(
            ToolOutput::success("generated 1 attachment")
                .with_attachments(self.attachments.clone()),
        )
    }
}

/// Helper that returns orchestrator, storage, AND executor so tests can
/// register custom ambient tools.
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
            retry_config: assistant_llm::RetryConfig::disabled(),
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

#[tokio::test]
async fn run_turn_collects_attachments_from_tool_output() {
    let server = MockServer::start().await;

    // 1st LLM call: model calls "attachment-tool".
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["attachment-tool"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 2nd LLM call: final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("here you go")))
        .mount(&server)
        .await;

    let (orch, _, executor) = build_with_executor(&server.uri()).await;

    // Register our mock tool that returns an attachment.
    let png_bytes = vec![0x89, 0x50, 0x4E, 0x47];
    executor.register_ambient_tool(Arc::new(MockAttachmentTool::new(vec![
        assistant_core::Attachment::new("chart.png", "image/png", png_bytes.clone()),
    ])));

    let result = orch
        .run_turn("make a chart", Uuid::new_v4(), Interface::Cli, None)
        .await
        .unwrap();

    assert_eq!(result.answer, "here you go");
    assert_eq!(
        result.attachments.len(),
        1,
        "expected 1 attachment in TurnResult"
    );
    assert_eq!(result.attachments[0].filename, "chart.png");
    assert_eq!(result.attachments[0].mime_type, "image/png");
    assert_eq!(result.attachments[0].data, png_bytes);
}

#[tokio::test]
async fn run_turn_collects_multiple_attachments_across_tool_calls() {
    let server = MockServer::start().await;

    // Model calls attachment-tool twice in one turn.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ollama_tool_calls(&["attachment-tool", "attachment-tool"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _, executor) = build_with_executor(&server.uri()).await;

    executor.register_ambient_tool(Arc::new(MockAttachmentTool::new(vec![
        assistant_core::Attachment::new("file.txt", "text/plain", b"hello".to_vec()),
    ])));

    let result = orch
        .run_turn("go", Uuid::new_v4(), Interface::Cli, None)
        .await
        .unwrap();

    assert_eq!(
        result.attachments.len(),
        2,
        "each tool call should contribute one attachment"
    );
    assert_eq!(result.attachments[0].filename, "file.txt");
    assert_eq!(result.attachments[1].filename, "file.txt");
}

#[tokio::test]
async fn run_turn_no_attachments_when_tools_return_none() {
    let server = MockServer::start().await;
    mount_answer(&server, "pong").await;

    let (orch, _, _) = build_with_executor(&server.uri()).await;

    let result = orch
        .run_turn("hello", Uuid::new_v4(), Interface::Cli, None)
        .await
        .unwrap();

    assert!(
        result.attachments.is_empty(),
        "no tool calls means no attachments"
    );
}

#[tokio::test]
async fn run_turn_streaming_collects_attachments() {
    let server = MockServer::start().await;

    // 1st LLM call: model calls "attachment-tool".
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["attachment-tool"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 2nd LLM call: final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _, executor) = build_with_executor(&server.uri()).await;
    executor.register_ambient_tool(Arc::new(MockAttachmentTool::new(vec![
        assistant_core::Attachment::new("report.pdf", "application/pdf", vec![0x25, 0x50]),
    ])));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<super::OrchestratorEvent>(64);

    // Drain events in background.
    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    let result = orch
        .run_turn_streaming("generate report", Uuid::new_v4(), Interface::Cli, tx, None)
        .await
        .unwrap();

    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].filename, "report.pdf");
    assert_eq!(result.attachments[0].mime_type, "application/pdf");
}

#[tokio::test]
async fn run_turn_with_tools_collects_attachments_from_extension() {
    let server = MockServer::start().await;

    // Model calls the extension tool then reply then end_turn.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[
                ("ext-attach", json!({})),
                ("reply", json!({"text": "done"})),
                ("end_turn", json!({"reason": "done"})),
            ])),
        )
        .mount(&server)
        .await;

    let (orch, _, _) = build_with_executor(&server.uri()).await;

    // Create an extension tool that returns attachments.
    struct ExtAttachTool;

    #[async_trait]
    impl ToolHandler for ExtAttachTool {
        fn name(&self) -> &str {
            "ext-attach"
        }
        fn description(&self) -> &str {
            "returns an attachment"
        }
        fn params_schema(&self) -> Value {
            json!({"type": "object", "properties": {}, "required": []})
        }
        async fn run(
            &self,
            _params: HashMap<String, Value>,
            _ctx: &ExecutionContext,
        ) -> anyhow::Result<ToolOutput> {
            Ok(ToolOutput::success("image generated").with_attachment(
                assistant_core::Attachment::new("img.png", "image/png", vec![1, 2, 3]),
            ))
        }
    }

    let reply_handler = Arc::new(MockExtTool::new("reply"));
    let ext_attach = Arc::new(ExtAttachTool);

    // run_turn_with_tools returns Ok(()) — we can't inspect attachments
    // directly, but we verify the call succeeds without panicking and
    // that the extension tool is executed (reply is called).
    orch.run_turn_with_tools(
        "make image",
        Uuid::new_v4(),
        Interface::Slack,
        vec![
            ext_attach as Arc<dyn ToolHandler>,
            reply_handler.clone() as Arc<dyn ToolHandler>,
        ],
        None,
        vec![],
    )
    .await
    .unwrap();

    assert_eq!(
        reply_handler.calls(),
        1,
        "reply tool should have been called"
    );
}

// ── sanitize_history tests ────────────────────────────────────────────────

#[test]
fn sanitize_history_empty_is_noop() {
    let mut history = vec![];
    crate::history::sanitize_history(&mut history);
    assert!(history.is_empty());
}

#[test]
fn sanitize_history_valid_alternation_is_noop() {
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "hello".into(),
        },
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content: "hi".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    assert_eq!(history.len(), 2, "valid alternation should not be modified");
}

#[test]
fn sanitize_history_trailing_user_inserts_assistant() {
    let mut history = vec![ChatHistoryMessage::Text {
        role: ChatRole::User,
        content: "orphaned".into(),
    }];
    crate::history::sanitize_history(&mut history);
    assert_eq!(
        history.len(),
        2,
        "should insert a synthetic assistant message"
    );
    match &history[1] {
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content,
        } => {
            assert!(
                content.contains("error"),
                "synthetic message should mention error"
            );
        }
        other => panic!("expected Text(Assistant), got {:?}", other),
    }
}

#[test]
fn sanitize_history_trailing_multimodal_user_inserts_assistant() {
    let mut history = vec![ChatHistoryMessage::MultimodalUser {
        content: vec![assistant_llm::ContentBlock::Text("image msg".into())],
    }];
    crate::history::sanitize_history(&mut history);
    assert_eq!(history.len(), 2);
    assert!(matches!(
        &history[1],
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            ..
        }
    ));
}

#[test]
fn sanitize_history_orphaned_tool_calls_get_synthetic_results() {
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "do stuff".into(),
        },
        ChatHistoryMessage::AssistantToolCalls(vec![
            ToolCallItem {
                name: "tool-a".into(),
                params: serde_json::json!({}),
                id: None,
            },
            ToolCallItem {
                name: "tool-b".into(),
                params: serde_json::json!({}),
                id: None,
            },
        ]),
        // Only one ToolResult — tool-b is missing.
        ChatHistoryMessage::ToolResult {
            name: "tool-a".into(),
            content: "ok".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    // Should have: User, AssistantToolCalls, ToolResult(a), ToolResult(b-synthetic)
    assert_eq!(history.len(), 4, "missing tool result should be inserted");
    match &history[3] {
        ChatHistoryMessage::ToolResult { name, content } => {
            assert_eq!(name, "tool-b");
            assert!(
                content.contains("lost") || content.contains("crash") || content.contains("error"),
                "synthetic result should indicate failure: {content}"
            );
        }
        other => panic!("expected ToolResult, got {:?}", other),
    }
}

#[test]
fn sanitize_history_fully_orphaned_tool_calls_all_results_inserted() {
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "run tools".into(),
        },
        ChatHistoryMessage::AssistantToolCalls(vec![
            ToolCallItem {
                name: "alpha".into(),
                params: serde_json::json!({}),
                id: None,
            },
            ToolCallItem {
                name: "beta".into(),
                params: serde_json::json!({}),
                id: None,
            },
        ]),
        // No ToolResult at all — process crashed right after persisting tool calls.
    ];
    crate::history::sanitize_history(&mut history);
    // Should have: User, AssistantToolCalls, ToolResult(alpha), ToolResult(beta)
    assert_eq!(
        history.len(),
        4,
        "both missing tool results should be inserted"
    );
    assert!(matches!(&history[2], ChatHistoryMessage::ToolResult { name, .. } if name == "alpha"));
    assert!(matches!(&history[3], ChatHistoryMessage::ToolResult { name, .. } if name == "beta"));
}

#[test]
fn sanitize_history_combined_orphaned_tools_and_trailing_user() {
    // Simulates: process crashed during tool execution on turn 1,
    // then on turn 2 the user message was persisted but LLM failed.
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "turn 1".into(),
        },
        ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
            name: "my-tool".into(),
            params: serde_json::json!({}),
            id: None,
        }]),
        // Missing ToolResult, then orphaned user from turn 2:
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "turn 2".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    // Should have: User, AssistantToolCalls, ToolResult(synthetic), User, Assistant(synthetic)
    assert_eq!(history.len(), 5);
    assert!(
        matches!(&history[2], ChatHistoryMessage::ToolResult { name, .. } if name == "my-tool")
    );
    assert!(matches!(
        &history[4],
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            ..
        }
    ));
}

// ── Bus integration tests ────────────────────────────────────────────────

#[test]
fn parse_interface_known_values() {
    use super::parse_interface;
    assert_eq!(parse_interface("Cli"), Interface::Cli);
    assert_eq!(parse_interface("cli"), Interface::Cli);
    assert_eq!(parse_interface("Slack"), Interface::Slack);
    assert_eq!(parse_interface("MATTERMOST"), Interface::Mattermost);
    assert_eq!(parse_interface("Signal"), Interface::Signal);
    assert_eq!(parse_interface("mcp"), Interface::Mcp);
}

#[test]
fn parse_interface_unknown_falls_back_to_cli() {
    use super::parse_interface;
    assert_eq!(parse_interface("unknown"), Interface::Cli);
    assert_eq!(parse_interface(""), Interface::Cli);
}

#[tokio::test]
async fn run_worker_processes_turn_request() {
    let server = MockServer::start().await;
    mount_answer(&server, "bus response").await;

    let (orch, _storage) = build(&server.uri()).await;

    // Spawn the worker in the background.
    let orch_worker = orch.clone();
    let worker = tokio::spawn(async move {
        orch_worker.run_worker("test-worker").await;
    });

    // Publish a TurnRequest to the bus.
    let conv_id = Uuid::new_v4();
    let turn_req = bus_messages::TurnRequest {
        prompt: "hello from bus".to_string(),
        conversation_id: conv_id,
        extension_tools: vec![],
        timestamp: None,
        traceparent: None,
        attachment_ids: vec![],
    };
    orch.bus()
        .publish(
            PublishRequest::new(
                topic::TURN_REQUEST,
                serde_json::to_value(&turn_req).unwrap(),
            )
            .with_agent_id("default")
            .with_conversation_id(conv_id)
            .with_interface("Cli"),
        )
        .await
        .unwrap();

    // Poll for the worker to process and publish the result instead of
    // a fixed sleep, which can be flaky under CI load.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let results = loop {
        let r = orch.bus().list(topic::TURN_RESULT, None, 10).await.unwrap();
        if !r.is_empty() {
            break r;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for TurnResult"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(results.len(), 1, "expected one TurnResult on the bus");
    let result: bus_messages::TurnResult =
        serde_json::from_value(results[0].payload.clone()).unwrap();
    assert_eq!(result.conversation_id, conv_id);
    assert_eq!(result.content, "bus response");

    // The original request should be acked (done).
    let pending = orch
        .bus()
        .list(
            topic::TURN_REQUEST,
            Some(assistant_core::MessageStatus::Pending),
            10,
        )
        .await
        .unwrap();
    assert!(pending.is_empty(), "turn request should be acked");

    worker.abort();
}

#[tokio::test]
async fn submit_turn_publishes_and_waits_for_result() {
    let server = MockServer::start().await;
    mount_answer(&server, "submitted answer").await;

    let (orch, _storage) = build(&server.uri()).await;

    // Spawn the worker so it can process the submitted turn.
    let orch_worker = orch.clone();
    tokio::spawn(async move {
        orch_worker.run_worker("test-worker").await;
    });

    let conv_id = Uuid::new_v4();
    let result = orch
        .submit_turn("hello via submit", conv_id, Interface::Cli, None)
        .await
        .unwrap();
    assert_eq!(result.answer, "submitted answer");
}

#[tokio::test]
async fn worker_propagates_submit_correlation_fields_to_turn_result() {
    let server = MockServer::start().await;
    mount_answer(&server, "correlated response").await;

    let (orch, _storage) = build(&server.uri()).await;

    let orch_worker = orch.clone();
    let worker = tokio::spawn(async move {
        orch_worker.run_worker("test-worker").await;
    });

    let conv_id = Uuid::new_v4();
    let request_id = Uuid::new_v4();
    let turn_req = bus_messages::TurnRequest {
        prompt: "hello correlation".to_string(),
        conversation_id: conv_id,
        extension_tools: vec![],
        timestamp: None,
        traceparent: None,
        attachment_ids: vec![],
    };

    let request_msg_id = orch
        .bus()
        .publish(
            PublishRequest::new(
                topic::TURN_REQUEST,
                serde_json::to_value(&turn_req).unwrap(),
            )
            .with_agent_id("default")
            .with_conversation_id(conv_id)
            .with_interface("Cli")
            .with_correlation_id(request_id)
            .with_batch_id(request_id),
        )
        .await
        .unwrap();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let results = loop {
        let r = orch.bus().list(topic::TURN_RESULT, None, 10).await.unwrap();
        if !r.is_empty() {
            break r;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for correlated TurnResult"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    let result_msg = &results[0];
    assert_eq!(
        result_msg.batch_id,
        Some(request_id),
        "TURN_RESULT should preserve the request batch_id"
    );
    assert_eq!(
        result_msg.correlation_id,
        Some(request_id),
        "TURN_RESULT should preserve the request correlation_id"
    );
    assert_eq!(
        result_msg.causation_id,
        Some(request_msg_id),
        "TURN_RESULT should set causation_id to the TURN_REQUEST message id"
    );

    worker.abort();
}

// ── Subagent integration tests ────────────────────────────────────────────

use assistant_core::{AgentReportStatus, AgentSpawn, SubagentRunner, DEFAULT_MAX_AGENT_DEPTH};

#[tokio::test]
async fn subagent_spawn_complete_round_trip() {
    let server = MockServer::start().await;

    // The subagent's LLM will return a final answer directly.
    mount_answer(&server, "subagent result").await;

    let (orch, storage) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "test-agent-1".into(),
        task: "What is 2+2?".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch.run_subagent(spawn, 0).await.unwrap();

    assert_eq!(report.status, AgentReportStatus::Completed);
    assert_eq!(report.content, "subagent result");

    // Verify lifecycle was recorded in the DB.
    let agent_store = storage.agent_store();
    let record = agent_store
        .get("test-agent-1")
        .await
        .unwrap()
        .expect("agent record should exist");
    assert_eq!(record.status, assistant_storage::AgentStatus::Completed);
    assert!(record.completed_at.is_some());
    assert_eq!(record.task, "What is 2+2?");
}

#[tokio::test]
async fn conversation_can_delegate_to_anonymous_subagent() {
    let server = MockServer::start().await;

    let task = "Search the web for rust async testing tips";

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains("delegate anonymously"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[(
                "agent-spawn",
                json!({
                    "task": task,
                    "allowed_tools": ["web-search"]
                }),
            )])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains(task))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("research-result")))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains("research-result"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ollama_answer("parent acknowledged anonymous result")),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    let turn = orch
        .run_turn("please delegate anonymously", conv_id, Interface::Cli, None)
        .await
        .unwrap();
    assert_eq!(turn.answer, "parent acknowledged anonymous result");

    let records = storage
        .agent_store()
        .list_by_parent_conversation(&conv_id.to_string())
        .await
        .unwrap();
    assert_eq!(records.len(), 1, "expected one delegated subagent record");
    assert_eq!(records[0].parent_conversation_id, conv_id.to_string());
    assert_eq!(records[0].parent_agent_id.as_deref(), Some("default"));

    let child_conv_id = Uuid::parse_str(&records[0].conversation_id).unwrap();
    let anonymous_scope = format!("anonymous::{}", records[0].id);
    let child_conv = storage
        .conversation_store_for_agent(&anonymous_scope)
        .get_conversation(child_conv_id)
        .await
        .unwrap()
        .expect("anonymous subagent conversation should be scoped to isolated persona");
    assert_eq!(child_conv.agent_id, anonymous_scope);

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        3,
        "expected parent->subagent->parent LLM call sequence"
    );
}

#[tokio::test]
async fn conversation_can_delegate_to_existing_agent_context() {
    let server = MockServer::start().await;

    let task = "Draft a launch blurb for next week";

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains("use marketing"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[(
                "agent-spawn",
                json!({
                    "agent_id": "marketing",
                    "task": task
                }),
            )])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains(task))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("marketing-result")))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains("marketing-result"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ollama_answer("parent acknowledged marketing result")),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;
    storage.persona_store().ensure_default().await.unwrap();
    storage
        .persona_store()
        .ensure_exists("marketing")
        .await
        .unwrap();

    let conv_id = Uuid::new_v4();
    let turn = orch
        .run_turn("please use marketing", conv_id, Interface::Cli, None)
        .await
        .unwrap();
    assert_eq!(turn.answer, "parent acknowledged marketing result");

    let delegated = storage
        .agent_store()
        .list_by_parent_conversation(&conv_id.to_string())
        .await
        .unwrap();
    assert_eq!(delegated.len(), 1, "expected one delegated subagent record");
    assert_eq!(delegated[0].parent_conversation_id, conv_id.to_string());
    assert_eq!(delegated[0].parent_agent_id.as_deref(), Some("default"));

    let child_conv_id = Uuid::parse_str(&delegated[0].conversation_id).unwrap();
    let child_conv = storage
        .conversation_store_for_agent("marketing")
        .get_conversation(child_conv_id)
        .await
        .unwrap()
        .expect("persona-bound subagent conversation should be scoped to target persona");
    assert_eq!(child_conv.agent_id, "marketing");

    let marketing = storage
        .agent_store()
        .get("marketing")
        .await
        .unwrap()
        .expect("marketing subagent record should exist");
    assert_eq!(marketing.status, assistant_storage::AgentStatus::Completed);
    assert_eq!(marketing.task, task);
}

#[tokio::test]
async fn subagent_nesting_depth_limit_enforced() {
    let server = MockServer::start().await;
    mount_answer(&server, "should not reach here").await;

    let (orch, _) = build(&server.uri()).await;

    // Spawn at max depth — should be rejected.
    let spawn = AgentSpawn {
        agent_id: "deep-agent".into(),
        task: "too deep".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch
        .run_subagent(spawn, DEFAULT_MAX_AGENT_DEPTH)
        .await
        .unwrap();

    assert_eq!(report.status, AgentReportStatus::Failed);
    assert!(
        report.content.contains("depth"),
        "error should mention depth: {}",
        report.content
    );

    // No LLM call should have been made.
    let reqs = server.received_requests().await.unwrap();
    assert!(
        reqs.is_empty(),
        "no LLM calls should be made when depth limit is exceeded"
    );
}

#[tokio::test]
async fn subagent_tool_filtering_restricts_tools() {
    let server = MockServer::start().await;

    // Subagent LLM tries to call "bash" which is NOT in the allowed list.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["bash"])))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second call returns final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "restricted-agent".into(),
        task: "try to use bash".into(),
        system_prompt: None,
        model: None,
        // Only allow file-read — bash should be rejected.
        allowed_tools: vec!["file-read".into()],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch.run_subagent(spawn, 0).await.unwrap();

    // The subagent should still complete (the LLM got a rejection
    // observation and then returned a final answer).
    assert_eq!(report.status, AgentReportStatus::Completed);
    assert_eq!(report.content, "done");

    // Verify the first LLM call had the restricted tool set —
    // the request should only contain "file-read", not "bash".
    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 2);
    let body: Value = serde_json::from_slice(&reqs[0].body).unwrap();
    let tool_names: Vec<String> = body["tools"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|t| t["function"]["name"].as_str().map(String::from))
        .collect();
    assert!(
        tool_names.contains(&"file-read".to_string()),
        "file-read should be in tool specs: {tool_names:?}"
    );
    assert!(
        !tool_names.contains(&"bash".to_string()),
        "bash should NOT be in tool specs: {tool_names:?}"
    );
}

#[tokio::test]
async fn subagent_cancellation_stops_loop() {
    let server = MockServer::start().await;

    // The subagent LLM returns tool calls indefinitely, so the subagent
    // would loop forever if not cancelled.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ollama_tool_calls(&["unknown-tool"]))
                // Add a small delay so the cancel has time to trigger
                .set_body_json(ollama_tool_calls(&["unknown-tool"])),
        )
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "cancel-me".into(),
        task: "infinite loop task".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    // Cancel the agent before it starts by pre-cancelling.
    // We can't easily cancel mid-loop in a unit test, but we can
    // test that the cancel_agent mechanism works by:
    // 1. Registering the token manually would require access to internals.
    // Instead, test cancel_agent returns false for unknown agents.
    let cancelled = orch.cancel_agent("nonexistent").await.unwrap();
    assert!(
        !cancelled,
        "cancelling nonexistent agent should return false"
    );

    // Test the actual cancellation flow: spawn in a task, cancel shortly after.
    let orch2 = orch.clone();
    let handle = tokio::spawn(async move { orch2.run_subagent(spawn, 0).await.unwrap() });

    // Give the subagent a moment to start and register the token.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let cancelled = orch.cancel_agent("cancel-me").await.unwrap();
    assert!(cancelled, "should find and cancel the running agent");

    // Wait for the subagent to finish.
    let report = handle.await.unwrap();
    assert_eq!(
        report.status,
        AgentReportStatus::Cancelled,
        "subagent should report Cancelled status, got: {:?}",
        report.status
    );

    // Verify lifecycle recorded as cancelled.
    let agent_store = storage.agent_store();
    let record = agent_store
        .get("cancel-me")
        .await
        .unwrap()
        .expect("agent record should exist");
    assert_eq!(record.status, assistant_storage::AgentStatus::Cancelled);
}

#[tokio::test]
async fn subagent_llm_error_records_failed_status() {
    let server = MockServer::start().await;

    // LLM returns a 500 error.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "error-agent".into(),
        task: "this will fail".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch.run_subagent(spawn, 0).await.unwrap();

    assert_eq!(report.status, AgentReportStatus::Failed);
    assert!(report.content.contains("LLM error"));

    let agent_store = storage.agent_store();
    let record = agent_store
        .get("error-agent")
        .await
        .unwrap()
        .expect("agent record should exist");
    assert_eq!(record.status, assistant_storage::AgentStatus::Failed);
}

// ── value_to_params_map tests ─────────────────────────────────────────────────

#[test]
fn value_to_params_map_converts_object() {
    let val = json!({"foo": "bar", "n": 42});
    let map = super::value_to_params_map(&val);
    assert_eq!(
        map.len(),
        2,
        "object with two keys should produce two params"
    );
    assert_eq!(map["foo"], json!("bar"), "foo should map to \"bar\"");
    assert_eq!(map["n"], json!(42), "n should map to 42");
}

#[test]
fn value_to_params_map_empty_object() {
    let val = json!({});
    let map = super::value_to_params_map(&val);
    assert!(map.is_empty(), "empty object should produce empty map");
}

#[test]
fn value_to_params_map_non_object_returns_empty() {
    for val in [json!(null), json!("string"), json!(42), json!([1, 2])] {
        let map = super::value_to_params_map(&val);
        assert!(map.is_empty(), "non-object {val} should produce empty map");
    }
}

// ── Thinking persistence tests ────────────────────────────────────────────────

/// Ollama response with empty content but non-empty thinking — triggers
/// LlmResponse::Thinking in the LLM client parser.
fn ollama_thinking(thought: &str) -> Value {
    json!({
        "model": "test",
        "message": { "role": "assistant", "content": "", "thinking": thought },
        "done": true
    })
}

#[tokio::test]
async fn subagent_thinking_step_persisted_to_db() {
    let server = MockServer::start().await;

    // First call: LLM returns a thinking step (empty content + thinking field).
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_thinking("deep thought")))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second call: LLM returns a normal final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "thinking-agent".into(),
        task: "think about it".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };
    let anon_scope = format!("anonymous::{}", spawn.agent_id);

    let report = orch.run_subagent(spawn, 0).await.unwrap();
    assert_eq!(report.status, AgentReportStatus::Completed);

    // Retrieve the subagent's conversation_id from the agent record.
    let agent_store = storage.agent_store();
    let record = agent_store
        .get("thinking-agent")
        .await
        .unwrap()
        .expect("agent record should exist");
    let conv_id =
        Uuid::parse_str(&record.conversation_id).expect("conversation_id should be a valid UUID");

    // Load persisted messages and verify the thinking step is present.
    // Anonymous subagents (persona_bound: false) scope their conversation under
    // "anonymous::<agent_id>", so we must use the matching store to find messages.
    let conv_store = storage.conversation_store_for_agent(&anon_scope);
    let messages = conv_store.load_history(conv_id).await.unwrap();
    let thinking_msg = messages.iter().find(|m| m.content.contains("<think>"));
    assert!(
        thinking_msg.is_some(),
        "thinking step should be persisted to DB; messages: {:?}",
        messages.iter().map(|m| &m.content).collect::<Vec<_>>()
    );
    assert!(
        thinking_msg.unwrap().content.contains("deep thought"),
        "persisted thinking message should contain the thought text"
    );
}

// ── Max-iterations error tests ────────────────────────────────────────────────

#[tokio::test]
async fn run_turn_max_iterations_returns_error() {
    let server = MockServer::start().await;

    // LLM always returns tool calls, never a final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["unknown-tool"])),
        )
        .mount(&server)
        .await;

    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    config.llm.max_iterations = 2;
    let (orch, _) = build_with_config(&server.uri(), config).await;

    let result = orch
        .run_turn("trigger loop", Uuid::new_v4(), Interface::Cli, None)
        .await;

    match result {
        Ok(_) => panic!("should fail when max iterations reached"),
        Err(e) => {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("Max iterations"),
                "error should mention max iterations: {err_msg}"
            );
        }
    }
}

#[tokio::test]
async fn subagent_max_iterations_returns_failed_status() {
    let server = MockServer::start().await;

    // LLM always returns tool calls, never a final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["unknown-tool"])),
        )
        .mount(&server)
        .await;

    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    config.llm.max_iterations = 2;
    let (orch, _) = build_with_config(&server.uri(), config).await;

    let spawn = AgentSpawn {
        agent_id: "loop-agent".into(),
        task: "infinite loop".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch.run_subagent(spawn, 0).await.unwrap();

    assert_eq!(
        report.status,
        AgentReportStatus::Failed,
        "subagent should report Failed when max iterations reached"
    );
    assert!(
        report.content.contains("max iterations"),
        "error should mention max iterations: {}",
        report.content
    );
}

// ── Worker integration tests ────────────────────────────────────────────────

/// Test that submit_turn works correctly when the worker is running.
/// This reproduces the bug where Slack/Mattermost-only modes didn't spawn
/// the worker, causing submit_turn to timeout waiting for a result.
#[tokio::test]
async fn submit_turn_with_worker_returns_result() {
    let server = MockServer::start().await;
    mount_answer(&server, "hello from worker").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    // Spawn the worker in the background (as the CLI would do).
    let worker_orch = orch.clone();
    let _worker = tokio::spawn(async move {
        worker_orch.run_worker("test-worker").await;
    });

    // Give the worker a moment to start.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result = orch
        .submit_turn("test message", conv_id, Interface::Slack, None)
        .await;

    assert!(
        result.is_ok(),
        "submit_turn should succeed when worker is running"
    );
    assert_eq!(result.unwrap().answer, "hello from worker");
}

/// Test that submit_turn times out when no worker is running.
/// This verifies the bug condition that was fixed.
#[tokio::test]
async fn submit_turn_without_worker_times_out() {
    let server = MockServer::start().await;
    mount_answer(&server, "this should not be received").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    // No worker spawned — simulates the bug condition.
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        orch.submit_turn("test message", conv_id, Interface::Slack, None),
    )
    .await;

    assert!(
        result.is_err() || result.unwrap().is_err(),
        "submit_turn should fail/timeout when worker is not running"
    );
}

/// Test that with_submit_timeout sets a custom deadline that is respected.
///
/// We configure a 1-second timeout and verify that submit_turn (with no worker
/// running) completes within a short wall-clock window — i.e. it honours the
/// configured value rather than the 3-hour default.
#[tokio::test]
async fn with_submit_timeout_respected() {
    let server = MockServer::start().await;
    mount_answer(&server, "not used").await;

    // Build without wrapping in Arc so we can apply with_submit_timeout first.
    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: server.uri(),
            timeout_secs: 10,
            retry_config: assistant_llm::RetryConfig::disabled(),
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
    let orch = Arc::new(
        Orchestrator::new(llm, storage, executor.clone(), registry, bus, &config)
            .with_submit_timeout(1), // 1-second deadline
    );
    executor.set_subagent_runner(orch.clone());

    let conv_id = Uuid::new_v4();
    let start = std::time::Instant::now();
    // Wrap in a generous outer timeout to prevent the test hanging if the
    // inner deadline is accidentally the 3-hour default.
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        orch.submit_turn("test message", conv_id, Interface::Slack, None),
    )
    .await;
    let elapsed = start.elapsed();

    // The inner submit_turn must have timed out (no worker running).
    assert!(
        result.is_ok() && result.unwrap().is_err(),
        "submit_turn should return Err when no worker is running"
    );
    // And it should have done so close to the configured 1-second deadline.
    assert!(
        elapsed < Duration::from_secs(5),
        "submit_turn should respect the 1-second timeout, not the 3-hour default (elapsed: {elapsed:?})"
    );
}

#[tokio::test]
async fn failed_turn_propagates_error() {
    // Verifies that when the LLM returns an error, run_turn propagates Err.
    // The error path is also where we set OtelStatus::Error on the turn span.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;

    let (orch, _storage) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    let result = orch.run_turn("test", conv_id, Interface::Cli, None).await;
    assert!(
        result.is_err(),
        "expected run_turn to return Err on LLM failure"
    );
}

/// Verify that when submit_turn times out it cancels the in-flight worker
/// turn via the CancellationToken registered in turn_cancellations.
///
/// The mock LLM is configured to delay its response by 5 s.  submit_turn
/// is given a 1-second deadline.  We assert:
///  1. submit_turn returns an error within ~2 s (the deadline fires).
///  2. The worker does NOT continue processing: if it did it would produce a
///     turn.result message that would linger on the bus, which we check for.
#[tokio::test]
async fn timeout_cancels_in_flight_worker_turn() {
    let server = MockServer::start().await;
    // Delay the LLM response so the turn is still running when submit_turn times out.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(5))
                .set_body_json(ollama_answer("delayed answer")),
        )
        .mount(&server)
        .await;

    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: server.uri(),
            timeout_secs: 30,
            retry_config: assistant_llm::RetryConfig::disabled(),
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
    let orch = Arc::new(
        Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry,
            bus,
            &config,
        )
        .with_submit_timeout(1), // 1-second deadline
    );
    executor.set_subagent_runner(orch.clone());

    // Spawn the worker so it picks up and processes the turn.
    let worker_orch = orch.clone();
    let _worker = tokio::spawn(async move {
        worker_orch.run_worker("test-worker").await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let conv_id = Uuid::new_v4();
    let start = std::time::Instant::now();
    let result = orch
        .submit_turn("slow task", conv_id, Interface::Slack, None)
        .await;
    let elapsed = start.elapsed();

    // submit_turn must have timed out with an error.
    assert!(result.is_err(), "submit_turn should error on timeout");
    assert!(
        elapsed < Duration::from_secs(4),
        "submit_turn should have stopped within ~1 s, not waited for the 5-s LLM response (elapsed: {elapsed:?})"
    );

    // Give the worker a moment — if cancellation worked there should be no
    // lingering turn.result message on the bus.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let leftover = storage
        .message_bus()
        .claim_filtered(
            assistant_core::topic::TURN_RESULT,
            "check",
            &assistant_core::ClaimFilter::new(),
        )
        .await
        .unwrap();
    assert!(
        leftover.is_none(),
        "worker should not have published a turn.result after cancellation"
    );
}

/// `run_turn_with_tools` uses `run_turn_with_tools_impl` which has its own
/// LLM call site.  Verify that an HTTP-500 from the LLM propagates as `Err`
/// through the extension-tool path (separate from `run_turn_core`).
#[tokio::test]
async fn failed_turn_with_tools_llm_error_propagates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;

    let result = orch
        .run_turn_with_tools(
            "test",
            Uuid::new_v4(),
            Interface::Slack,
            vec![],
            None,
            vec![],
        )
        .await;
    assert!(
        result.is_err(),
        "run_turn_with_tools must return Err on LLM HTTP-500"
    );
}

/// `run_turn_with_tools` must return `Err` when the LLM loops forever
/// (always returns tool calls, never a final answer).
#[tokio::test]
async fn failed_turn_with_tools_max_iterations_returns_error() {
    let server = MockServer::start().await;
    // LLM never produces a final answer — always returns a tool call.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["unknown-tool"])),
        )
        .mount(&server)
        .await;

    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    config.llm.max_iterations = 2;
    let (orch, _) = build_with_config(&server.uri(), config).await;

    let result = orch
        .run_turn_with_tools(
            "loop forever",
            Uuid::new_v4(),
            Interface::Cli,
            vec![],
            None,
            vec![],
        )
        .await;
    match result {
        Ok(_) => panic!("should fail when max iterations reached"),
        Err(e) => assert!(
            e.to_string().contains("Max iterations"),
            "error should mention max iterations: {e}"
        ),
    }
}

// ── Streaming tests ───────────────────────────────────────────────────────────

/// Build a newline-delimited JSON (NDJSON) streaming body that the Ollama
/// `chat_native_streaming` parser can consume.  Each `token` becomes a
/// `{"message":{"content":token},"done":false}` chunk; a final
/// `{"done":true}` chunk terminates the stream.
fn ollama_streaming_body(tokens: &[&str]) -> String {
    let mut body = String::new();
    for token in tokens {
        let line = json!({
            "model": "test",
            "message": {"role": "assistant", "content": token},
            "done": false,
        });
        body.push_str(&line.to_string());
        body.push('\n');
    }
    // Final chunk signals end-of-stream.
    let done = json!({
        "model": "test",
        "message": {"role": "assistant", "content": ""},
        "done": true,
        "done_reason": "stop",
    });
    body.push_str(&done.to_string());
    body.push('\n');
    body
}

/// `run_turn_with_tools_streaming` must forward tokens from the LLM text
/// stream through the caller-supplied `token_sink`.
#[tokio::test]
async fn run_turn_with_tools_streaming_emits_tokens_through_sink() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(ollama_streaming_body(&["Hello", ",", " world", "!"])),
        )
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<super::OrchestratorEvent>(64);

    orch.run_turn_with_tools_streaming(
        "hi",
        Uuid::new_v4(),
        Interface::Slack,
        vec![],
        None,
        vec![],
        tx,
    )
    .await
    .unwrap();

    // Drain all received tokens.
    let mut received = String::new();
    while let Ok(event) = rx.try_recv() {
        if let super::OrchestratorEvent::Token(t) = event {
            received.push_str(&t);
        }
    }

    assert_eq!(
        received, "Hello, world!",
        "token_sink must receive all LLM text tokens"
    );
}

/// When the token_sink receives tokens in multiple chunks, they must arrive
/// in order and form the complete response.
#[tokio::test]
async fn run_turn_with_tools_streaming_tokens_arrive_in_order() {
    let server = MockServer::start().await;
    let tokens = ["The", " quick", " brown", " fox"];
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ollama_streaming_body(&tokens)))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<super::OrchestratorEvent>(64);

    orch.run_turn_with_tools_streaming(
        "tell me something",
        Uuid::new_v4(),
        Interface::Slack,
        vec![],
        None,
        vec![],
        tx,
    )
    .await
    .unwrap();

    let mut received_tokens: Vec<String> = Vec::new();
    while let Ok(event) = rx.try_recv() {
        if let super::OrchestratorEvent::Token(t) = event {
            received_tokens.push(t);
        }
    }

    let joined: String = received_tokens.join("");
    assert_eq!(
        joined, "The quick brown fox",
        "tokens must arrive in order and reconstruct the full text"
    );
}

/// When the worker processes a turn that has BOTH extension tools AND a
/// token_sink registered, it must route to `run_turn_with_tools_streaming`
/// so the sink receives tokens (rather than silently ignoring the sink).
#[tokio::test]
async fn worker_routes_ext_plus_sink_to_streaming_path() {
    let server = MockServer::start().await;
    // Mount a streaming answer: the LLM returns text tokens (FinalAnswer path),
    // which flow through the token_sink.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(ollama_streaming_body(&["streamed", " answer"])),
        )
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    // Spawn the worker.
    let worker_orch = orch.clone();
    let _worker = tokio::spawn(async move {
        worker_orch.run_worker("test-worker").await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Register BOTH a token_sink AND extension tools (the Slack pattern).
    let (tx, mut rx) = tokio::sync::mpsc::channel::<super::OrchestratorEvent>(64);
    orch.register_token_sink(conv_id, tx).await;
    orch.register_extensions(conv_id, vec![], vec![]).await;

    // submit_turn triggers the worker which should pick up both registrations.
    let _ = orch
        .submit_turn("stream please", conv_id, Interface::Slack, None)
        .await;

    // Allow any in-flight sends to the channel to complete.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut received = String::new();
    while let Ok(event) = rx.try_recv() {
        if let super::OrchestratorEvent::Token(t) = event {
            received.push_str(&t);
        }
    }

    assert_eq!(
        received, "streamed answer",
        "worker must route to streaming path when ext tools + token_sink are both registered; \
         sink received: {received:?}"
    );
}

// ── resize_and_encode tests ──────────────────────────────────────────────

#[test]
fn resize_and_encode_small_image_returns_base64_without_resize() {
    use base64::Engine as _;

    let raw = b"tiny image bytes";
    let encoded = super::resize_and_encode(raw, "image/png", "anthropic");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .unwrap();
    assert_eq!(
        decoded, raw,
        "small images should be returned as-is (base64-encoded)"
    );
}

#[test]
fn resize_and_encode_unknown_mime_returns_base64() {
    use base64::Engine as _;

    let raw = b"some pdf content";
    let encoded = super::resize_and_encode(raw, "application/pdf", "openai");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .unwrap();
    assert_eq!(decoded, raw, "unknown MIME types should be returned as-is");
}

#[test]
fn max_image_bytes_anthropic_is_5mb() {
    assert_eq!(
        super::max_image_bytes_for_provider("anthropic"),
        5 * 1024 * 1024
    );
}

#[test]
fn max_image_bytes_openai_is_20mb() {
    assert_eq!(
        super::max_image_bytes_for_provider("openai"),
        20 * 1024 * 1024
    );
}

#[test]
fn max_image_bytes_default_is_conservative() {
    assert_eq!(
        super::max_image_bytes_for_provider("ollama"),
        5 * 1024 * 1024
    );
}
