//! Smaller orchestrator test clusters that don't fit cleanly into the
//! larger sibling modules:
//!
//! - `value_to_params_map` JSON → params conversion
//! - max-iterations error handling
//! - turn identity threading
//! - `Slice C` had_errors signal

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::agent::AssistantConfig;
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};
use assistant_core::{OrgId, SpaceId, UserId};

use super::super::value_to_params_map;
use super::{
    build, build_with_config, build_with_executor, mount_answer, ollama_answer, ollama_tool_calls,
};

// ── value_to_params_map tests ─────────────────────────────────────────────────

#[test]
fn value_to_params_map_converts_object() {
    let val = json!({"foo": "bar", "n": 42});
    let map = value_to_params_map(&val);
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
    let map = value_to_params_map(&val);
    assert!(map.is_empty(), "empty object should produce empty map");
}

#[test]
fn value_to_params_map_non_object_returns_empty() {
    for val in [json!(null), json!("string"), json!(42), json!([1, 2])] {
        let map = value_to_params_map(&val);
        assert!(map.is_empty(), "non-object {val} should produce empty map");
    }
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
