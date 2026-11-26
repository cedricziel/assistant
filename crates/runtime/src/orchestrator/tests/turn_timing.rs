//! `run_turn_core` measures turn timing on the orchestrator's injected clock,
//! persists it per assistant message and returns it on `TurnResult.timing`.
//!
//! All tests run on the current-thread runtime so the timed scripted provider
//! and the forwarding task interleave deterministically.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use assistant_core::auth::AuthContext;
use assistant_core::clock::FakeClock;
use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};
use assistant_core::{
    LlmProvider, LlmResponse, LlmResponseMeta, MessageBus, StreamChunk, ToolCallItem,
    ToolCallResponse, TurnTiming,
};
use assistant_llm_provider::ScriptedLlmProvider;
use assistant_llm_provider::scripted::TimedCall;
use assistant_storage::{MessageTimingStore, StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::super::{Orchestrator, OrchestratorEvent, SubmitTurn, TurnResult};

/// A tool that advances the shared fake clock by `advance` when it runs.
struct ClockTool {
    name: &'static str,
    advance: Duration,
    clock: FakeClock,
}

#[async_trait]
impl ToolHandler for ClockTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "advances the fake clock"
    }
    fn params_schema(&self) -> Value {
        json!({"type": "object", "properties": {}, "required": []})
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        self.clock.advance(self.advance);
        Ok(ToolOutput::success("ok"))
    }
}

struct Rig {
    orch: Arc<Orchestrator>,
    storage: Arc<StorageLayer>,
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn text(advance_ms: u64, s: &str) -> (Duration, StreamChunk) {
    (ms(advance_ms), StreamChunk::Text(s.to_string()))
}

fn thinking(advance_ms: u64, s: &str) -> (Duration, StreamChunk) {
    (ms(advance_ms), StreamChunk::Thinking(s.to_string()))
}

fn timed(steps: Vec<(Duration, StreamChunk)>, return_after_ms: u64) -> TimedCall {
    TimedCall {
        steps,
        advance_before_return: ms(return_after_ms),
    }
}

fn meta(output_tokens: Option<u64>) -> LlmResponseMeta {
    LlmResponseMeta {
        output_tokens,
        ..Default::default()
    }
}

fn answer(s: &str, output_tokens: Option<u64>) -> LlmResponse {
    LlmResponse::FinalAnswer(s.to_string(), meta(output_tokens))
}

fn tool_call(name: &str, output_tokens: Option<u64>) -> LlmResponse {
    LlmResponse::ToolCalls(ToolCallResponse {
        items: vec![ToolCallItem {
            name: name.to_string(),
            params: json!({}),
            id: None,
        }],
        meta: meta(output_tokens),
        thinking: None,
        provider_state: None,
    })
}

async fn rig(calls: Vec<TimedCall>, responses: Vec<LlmResponse>) -> Rig {
    let clock = FakeClock::new_at_seed();
    let llm: Arc<dyn LlmProvider> = Arc::new(
        ScriptedLlmProvider::new()
            .with_timed_streaming_calls(clock.clone(), calls)
            .with_canned_responses(responses),
    );
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
    for (name, advance) in [("slow", ms(5_000)), ("probe", ms(0))] {
        executor.register_ambient_tool(Arc::new(ClockTool {
            name,
            advance,
            clock: clock.clone(),
        }));
    }
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
        .with_clock(Arc::new(clock)),
    );
    executor.set_subagent_runner(orch.clone());
    Rig { orch, storage }
}

async fn run_streaming(rig: &Rig, conversation_id: Uuid) -> anyhow::Result<TurnResult> {
    let (tx, _rx) = mpsc::channel::<OrchestratorEvent>(64);
    rig.orch
        .run_turn_streaming(
            "hi",
            conversation_id,
            Interface::Cli,
            tx,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
}

async fn run_plain(rig: &Rig, conversation_id: Uuid) -> anyhow::Result<TurnResult> {
    rig.orch
        .run_turn(
            "hi",
            conversation_id,
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
}

fn timing(
    ttft: Option<u64>,
    total: u64,
    output_tokens: Option<u64>,
    decode: Option<u64>,
) -> TurnTiming {
    TurnTiming {
        time_to_first_token_ms: ttft,
        total_ms: total,
        output_tokens,
        decode_ms: decode,
    }
}

#[tokio::test]
async fn streaming_turn_measures_ttft_total_and_decode() {
    let rig = rig(
        vec![timed(vec![text(800, "Hello")], 2_200)],
        vec![answer("Hello", Some(120))],
    )
    .await;
    let result = run_streaming(&rig, Uuid::new_v4()).await.unwrap();
    assert_eq!(
        result.timing,
        Some(timing(Some(800), 3_000, Some(120), Some(2_200)))
    );
}

#[tokio::test]
async fn turn_without_a_token_sink_has_no_first_token_time() {
    let rig = rig(
        vec![timed(vec![text(800, "Hello")], 2_200)],
        vec![answer("Hello", Some(120))],
    )
    .await;
    let result = run_plain(&rig, Uuid::new_v4()).await.unwrap();
    assert_eq!(
        result.timing,
        Some(timing(None, 3_000, Some(120), Some(3_000))),
        "decode is the whole call when no chunk is observed"
    );
}

#[tokio::test]
async fn thinking_does_not_count_as_the_first_token() {
    let rig = rig(
        vec![timed(vec![thinking(200, "hmm"), text(1_300, "Hello")], 500)],
        vec![answer("Hello", Some(50))],
    )
    .await;
    let result = run_streaming(&rig, Uuid::new_v4()).await.unwrap();
    assert_eq!(
        result.timing,
        Some(timing(Some(1_500), 2_000, Some(50), Some(1_800))),
        "ttft waits for text; decode starts at the first chunk of any kind"
    );
}

#[tokio::test]
async fn tool_time_counts_toward_total_but_not_decode() {
    let rig = rig(
        vec![
            timed(vec![text(300, "a")], 700),
            timed(vec![text(200, "b")], 800),
        ],
        vec![tool_call("slow", Some(10)), answer("done", Some(20))],
    )
    .await;
    let result = run_streaming(&rig, Uuid::new_v4()).await.unwrap();
    assert_eq!(
        result.timing,
        Some(timing(Some(300), 7_000, Some(30), Some(1_500)))
    );
}

#[tokio::test]
async fn tool_call_only_call_counts_its_whole_duration() {
    let rig = rig(
        vec![timed(vec![], 1_000), timed(vec![text(200, "ok")], 1_000)],
        vec![tool_call("probe", Some(30)), answer("ok", Some(100))],
    )
    .await;
    let result = run_streaming(&rig, Uuid::new_v4()).await.unwrap();
    assert_eq!(
        result.timing,
        Some(timing(Some(1_200), 2_200, Some(130), Some(2_000)))
    );
}

#[tokio::test]
async fn missing_usage_drops_token_figures_but_keeps_total() {
    let rig = rig(
        vec![
            timed(vec![text(300, "a")], 700),
            timed(vec![text(200, "b")], 800),
        ],
        vec![tool_call("slow", None), answer("done", Some(20))],
    )
    .await;
    let result = run_streaming(&rig, Uuid::new_v4()).await.unwrap();
    assert_eq!(result.timing, Some(timing(Some(300), 7_000, None, None)));
}

#[tokio::test]
async fn timing_is_stored_under_the_message_id() {
    let rig = rig(
        vec![timed(vec![text(800, "Hello")], 2_200)],
        vec![answer("Hello", Some(120))],
    )
    .await;
    let conversation_id = Uuid::new_v4();
    let result = run_streaming(&rig, conversation_id).await.unwrap();
    let stored = rig
        .storage
        .message_timing_store()
        .get_for_conversation(conversation_id)
        .await
        .unwrap();
    let message_id = result.message_id.expect("answer was persisted");
    assert_eq!(stored.get(&message_id), result.timing.as_ref());
    assert_eq!(stored.len(), 1);
}

#[tokio::test]
async fn empty_final_answer_has_no_timing_and_no_row() {
    let rig = rig(vec![timed(vec![], 500)], vec![answer("   ", Some(1))]).await;
    let conversation_id = Uuid::new_v4();
    let result = run_streaming(&rig, conversation_id).await.unwrap();
    assert_eq!(result.timing, None);
    let stored = rig
        .storage
        .message_timing_store()
        .get_for_conversation(conversation_id)
        .await
        .unwrap();
    assert!(stored.is_empty(), "no row for an unsaved answer");
}

#[tokio::test]
async fn failing_timing_store_does_not_fail_the_turn() {
    let rig = rig(
        vec![timed(vec![text(800, "Hello")], 2_200)],
        vec![answer("Hello", Some(120))],
    )
    .await;
    sqlx::query("DROP TABLE message_timings")
        .execute(&rig.storage.pool)
        .await
        .unwrap();
    let result = run_streaming(&rig, Uuid::new_v4())
        .await
        .expect("a failed timing put must not fail the turn");
    assert_eq!(result.answer, "Hello");
    assert!(result.message_id.is_some());
}

#[tokio::test]
async fn llm_error_produces_no_timing_row() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    let (orch, storage) = super::build(&server.uri()).await;
    let conversation_id = Uuid::new_v4();
    let result = orch
        .run_turn(
            "hi",
            conversation_id,
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await;
    assert!(result.is_err());
    let stored = storage
        .message_timing_store()
        .get_for_conversation(conversation_id)
        .await
        .unwrap();
    assert!(stored.is_empty());
}

#[tokio::test]
async fn timing_crosses_the_bus_through_submit() {
    let rig = rig(
        vec![timed(vec![text(800, "Hello")], 2_200)],
        vec![answer("Hello", Some(120))],
    )
    .await;
    let worker = {
        let orch = rig.orch.clone();
        tokio::spawn(async move { orch.run_worker("timing-worker").await })
    };
    let conversation_id = Uuid::new_v4();
    let result = rig
        .orch
        .submit(
            SubmitTurn::new("hi", conversation_id, Interface::Cli)
                .with_auth(&AuthContext::system()),
        )
        .await
        .unwrap();
    worker.abort();

    let stored = rig
        .storage
        .message_timing_store()
        .get_for_conversation(conversation_id)
        .await
        .unwrap();
    let computed = stored.values().next().cloned();
    assert!(computed.is_some(), "the worker persisted a timing");
    assert_eq!(result.timing, computed);
}
