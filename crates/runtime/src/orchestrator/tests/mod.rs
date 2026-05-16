use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};
use assistant_core::{
    ChatHistoryMessage, ChatRole, ContentBlock, LlmProvider, MessageBus, OrgId, SpaceId,
    ToolCallItem, UserId, types::agent::AssistantConfig,
};
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

// ── MultimodalUser / OTel serialisation tests ────────────────────────────

#[test]
fn serialize_history_multimodal_user_omits_base64_data() {
    use crate::otel_spans::serialize_history_for_span;

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
            assert_eq!(*role, ChatRole::User);
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
        .run_turn(
            "make a chart",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
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
        .run_turn(
            "hello",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
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
        .run_turn_streaming(
            "generate report",
            Uuid::new_v4(),
            Interface::Cli,
            tx,
            None,
            vec![],
            TurnIdentity::default(),
        )
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
        vec![],
        TurnIdentity::default(),
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
        content: vec![ContentBlock::Text("image msg".into())],
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

#[test]
fn sanitize_history_orphaned_tool_result_dropped() {
    // Simulates: a system-injected tool result (e.g. skill-learner) appears
    // at the start of history with no preceding AssistantToolCalls.
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "hello".into(),
        },
        ChatHistoryMessage::ToolResult {
            name: "skill-learner".into(),
            content: "Auto-created skill 'foo'".into(),
        },
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content: "hi".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    // The orphaned ToolResult should be dropped
    assert_eq!(history.len(), 2);
    assert!(matches!(
        &history[0],
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            ..
        }
    ));
    assert!(matches!(
        &history[1],
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            ..
        }
    ));
}

#[test]
fn sanitize_history_tool_result_after_matched_calls_dropped() {
    // Extra ToolResult beyond what the tool calls declared should be dropped.
    let mut history = vec![
        ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
            name: "my-tool".into(),
            params: serde_json::json!({}),
            id: Some("call_1".into()),
        }]),
        ChatHistoryMessage::ToolResult {
            name: "my-tool".into(),
            content: "result".into(),
        },
        // Spurious extra result with no matching call
        ChatHistoryMessage::ToolResult {
            name: "skill-learner".into(),
            content: "injected".into(),
        },
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content: "done".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    // The extra ToolResult should be dropped
    assert_eq!(history.len(), 3);
    assert!(matches!(
        &history[0],
        ChatHistoryMessage::AssistantToolCalls(_)
    ));
    assert!(matches!(
        &history[1],
        ChatHistoryMessage::ToolResult { name, .. } if name == "my-tool"
    ));
    assert!(matches!(
        &history[2],
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            ..
        }
    ));
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
        .run_turn(
            "trigger loop",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
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

// ── AudioStore integration ──────────────────────────────────────────────

#[tokio::test]
async fn with_audio_store_makes_store_available() {
    let store = Arc::new(assistant_transcription::AudioStore::new());
    let mut config = AssistantConfig::default();
    config.memory.enabled = false;

    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: "http://localhost:1".to_string(),
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
    let orch = Orchestrator::new(llm, storage, executor, registry, bus, &config)
        .with_audio_store(store.clone());

    assert!(
        orch.audio_store.is_some(),
        "audio_store should be set after with_audio_store"
    );

    // Verify the store is the same instance.
    let id = store
        .insert(b"fake-mp3".to_vec(), "audio/mpeg".to_string())
        .await;
    let retrieved = orch.audio_store.as_ref().unwrap().get(id).await;
    assert!(
        retrieved.is_some(),
        "orchestrator's audio store should share state with the provided store"
    );
}

#[tokio::test]
async fn audio_store_none_by_default() {
    let server = MockServer::start().await;
    mount_answer(&server, "ok").await;
    let (orch, _) = build(&server.uri()).await;
    assert!(
        orch.audio_store.is_none(),
        "audio_store should be None by default"
    );
}

// ── Identity threading test ──────────────────────────────────────────────

/// A mock tool that captures the [`ExecutionContext`] it receives.
struct IdentityCaptureTool {
    captured: Arc<tokio::sync::Mutex<Option<ExecutionContext>>>,
}

#[async_trait::async_trait]
impl ToolHandler for IdentityCaptureTool {
    fn name(&self) -> &str {
        "identity-probe"
    }
    fn description(&self) -> &str {
        "Captures execution context for testing"
    }
    fn params_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        *self.captured.lock().await = Some(ctx.clone());
        Ok(ToolOutput::success("captured"))
    }
}

#[tokio::test]
async fn turn_identity_reaches_tool_handler() {
    let server = MockServer::start().await;

    // First LLM call → invoke our probe tool; second → final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["identity-probe"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _storage) = build(&server.uri()).await;

    // Register our probe tool.
    let captured = Arc::new(tokio::sync::Mutex::new(None));
    let probe = Arc::new(IdentityCaptureTool {
        captured: captured.clone(),
    });
    orch.executor.register_ambient_tool(probe);

    // Run a turn with explicit identity.
    let identity = TurnIdentity {
        user_id: Some(UserId::from("usr_alice")),
        org_id: Some(OrgId::from("org_acme")),
        space_id: Some(SpaceId::from("spc_default")),
    };

    let result = orch
        .run_turn(
            "probe identity",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            identity,
        )
        .await
        .unwrap();

    assert_eq!(result.answer, "done");

    // Verify the tool handler received the correct identity.
    let ctx = captured.lock().await;
    let ctx = ctx.as_ref().expect("tool should have been called");
    assert_eq!(
        ctx.user_id.as_ref().map(|u| u.0.as_str()),
        Some("usr_alice"),
        "user_id should reach tool handler"
    );
    assert_eq!(
        ctx.org_id.as_ref().map(|o| o.0.as_str()),
        Some("org_acme"),
        "org_id should reach tool handler"
    );
    assert_eq!(
        ctx.space_id.as_ref().map(|s| s.0.as_str()),
        Some("spc_default"),
        "space_id should reach tool handler"
    );
}

// ── Slice C: turn_had_errors signal ───────────────────────────────────────

/// Tool that always returns an `Err(...)` — used to drive the
/// `turn_had_errors` signal in the orchestrator turn loop.
struct FailingTool;

#[async_trait]
impl ToolHandler for FailingTool {
    fn name(&self) -> &str {
        "failing-tool"
    }

    fn description(&self) -> &str {
        "always errors"
    }

    fn params_schema(&self) -> Value {
        json!({ "type": "object", "properties": {}, "required": [] })
    }

    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        anyhow::bail!("simulated tool failure")
    }
}

#[tokio::test]
async fn run_turn_marks_had_errors_when_tool_fails() {
    let server = MockServer::start().await;

    // 1st LLM call: model invokes the failing tool.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["failing-tool"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 2nd LLM call: final answer (model recovers from the tool error).
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("recovered")))
        .mount(&server)
        .await;

    let (orch, _, executor) = build_with_executor(&server.uri()).await;
    executor.register_ambient_tool(Arc::new(FailingTool));

    let result = orch
        .run_turn(
            "trigger error",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert_eq!(
        result.answer, "recovered",
        "turn should still return the recovered final answer after a failing tool"
    );
    assert!(
        result.had_errors,
        "TurnResult.had_errors should be true after a tool returned Err"
    );
}

#[tokio::test]
async fn run_turn_clears_had_errors_when_no_tool_fails() {
    let server = MockServer::start().await;
    mount_answer(&server, "all good").await;

    let (orch, _, _executor) = build_with_executor(&server.uri()).await;

    let result = orch
        .run_turn(
            "no tool calls",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert_eq!(
        result.answer, "all good",
        "turn without tool failures should return the model answer"
    );
    assert!(
        !result.had_errors,
        "TurnResult.had_errors should be false when no tool errored"
    );
}

#[tokio::test]
async fn run_turn_with_tools_marks_had_errors_when_extension_fails() {
    // Exercises the Slack/Mattermost extension-tool path: a failing extension
    // handler must propagate `had_errors = true` through
    // `handle_final_answer_with_extensions` and the per-turn accumulator,
    // mirroring `run_turn_marks_had_errors_when_tool_fails` for `run_turn`.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["failing-tool"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("recovered")))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let failing_ext: Arc<dyn ToolHandler> = Arc::new(FailingTool);

    // Use Cli interface so a plain final-answer turn is accepted without a
    // `reply` extension tool; the focus of this test is the extension-handler
    // failure path, not the messenger reply protocol.
    //
    // `run_turn_with_tools_impl` always returns `answer: String::new()` (the
    // assistant text is persisted to the conversation store, not surfaced
    // through TurnResult), so we only assert on `had_errors`.
    let result = orch
        .run_turn_with_tools(
            "trigger error",
            Uuid::new_v4(),
            Interface::Cli,
            vec![failing_ext],
            None,
            vec![],
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert!(
        result.had_errors,
        "TurnResult.had_errors must be true after a failing extension tool \
         (covers the run_turn_with_tools_impl wiring)"
    );
}
