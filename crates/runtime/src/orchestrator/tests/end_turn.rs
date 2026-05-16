//! `end_turn` rejection and empty-FinalAnswer history-poisoning tests.
//!
//! Covers the reply-tool contract enforced by the orchestrator: if an
//! extension reply tool exists, the assistant must invoke it before
//! `end_turn` is accepted. Also covers the guard that an empty
//! `FinalAnswer` from the LLM is not persisted (which would otherwise
//! poison subsequent history requests).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};

use super::{
    MockExtTool, build, messages_in, mount_answer, ollama_answer, ollama_tool_calls,
    ollama_tool_calls_with_args,
};

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
        vec![],
        TurnIdentity::default(),
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
    orch.run_turn_with_tools(
        "hi",
        Uuid::new_v4(),
        Interface::Cli,
        vec![],
        None,
        vec![],
        vec![],
        TurnIdentity::default(),
    )
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
        vec![],
        TurnIdentity::default(),
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
        vec![],
        TurnIdentity::default(),
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
        vec![],
        TurnIdentity::default(),
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
            m.role == assistant_core::types::conversation::MessageRole::Assistant
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
        .filter(|m| m.role == assistant_core::types::conversation::MessageRole::Assistant)
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
        .run_turn(
            "hello",
            conv_id,
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
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
            m.role == assistant_core::types::conversation::MessageRole::Assistant
                && m.content.trim().is_empty()
        })
        .collect();
    assert!(
        empty_assistant_msgs.is_empty(),
        "empty assistant message must not be persisted in run_turn; found {} in DB",
        empty_assistant_msgs.len()
    );
}
