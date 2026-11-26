//! Tool results carry structured `data` and a truthful status on the
//! `ToolResult` event: `ToolOutput.success == false` is reported as `error`.

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::mcp_app::McpApp;
use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{ExecutionContext, Interface, MessageRole, TurnIdentity};
use assistant_core::{
    ChatHistoryMessage, LlmProvider, LlmResponse, LlmResponseMeta, MessageBus, ToolCallItem,
    ToolCallResponse,
};
use assistant_llm_provider::ScriptedLlmProvider;
use assistant_storage::{
    ConversationStore, SkillExecutionStore, SkillOutcome, StorageLayer, WebhookStore,
    registry::SkillRegistry,
};
use assistant_tool_executor::ToolExecutor;
use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::orchestrator::{Orchestrator, OrchestratorEvent};

type OutputFn = Box<dyn Fn() -> ToolOutput + Send + Sync>;

/// A handler that returns whatever its factory builds.
struct StubTool {
    name: &'static str,
    output: OutputFn,
}

#[async_trait]
impl ToolHandler for StubTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "stub tool"
    }
    fn params_schema(&self) -> Value {
        json!({"type": "object", "properties": {}, "required": []})
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        Ok((self.output)())
    }
}

fn call(tool: &str) -> LlmResponse {
    call_with(tool, json!({}))
}

fn call_with(tool: &str, params: Value) -> LlmResponse {
    LlmResponse::ToolCalls(ToolCallResponse {
        items: vec![ToolCallItem {
            name: tool.to_string(),
            params,
            id: Some("call_1".to_string()),
        }],
        meta: LlmResponseMeta::default(),
        thinking: None,
        provider_state: None,
    })
}

struct Run {
    events: Vec<OrchestratorEvent>,
    storage: Arc<StorageLayer>,
    llm: Arc<ScriptedLlmProvider>,
    conversation_id: Uuid,
}

impl Run {
    fn tool_results(&self) -> Vec<(String, Option<String>, Option<Value>)> {
        self.events
            .iter()
            .filter_map(|e| match e {
                OrchestratorEvent::ToolResult {
                    status,
                    result,
                    data,
                    ..
                } => Some((status.clone(), result.clone(), data.clone())),
                _ => None,
            })
            .collect()
    }

    async fn persisted_tool_content(&self) -> Vec<String> {
        self.storage
            .conversation_store()
            .load_history(self.conversation_id)
            .await
            .unwrap()
            .into_iter()
            .filter(|m| matches!(m.role, MessageRole::Tool))
            .map(|m| m.content)
            .collect()
    }
}

/// Run one streaming turn: the LLM calls `tool` once, then answers.
/// `prelude` responses run first (used to activate a skill).
async fn run_with(
    prelude: Vec<LlmResponse>,
    tool: &'static str,
    output: OutputFn,
    deny: bool,
) -> Run {
    let mut responses = prelude;
    responses.push(call(tool));
    responses.push(LlmResponse::FinalAnswer(
        "done".to_string(),
        LlmResponseMeta::default(),
    ));
    let llm = Arc::new(ScriptedLlmProvider::new().with_canned_responses(responses));
    let llm_dyn: Arc<dyn LlmProvider> = llm.clone();

    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    if !deny {
        config.permissions.allow = vec!["*".parse().expect("valid rule")];
    }

    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm_dyn.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    executor.register_ambient_tool(Arc::new(StubTool { name: tool, output }));
    executor.register_ambient_tool(Arc::new(StubTool {
        name: "load-skill",
        output: Box::new(|| ToolOutput::success("loaded")),
    }));

    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Orchestrator::new(llm_dyn, storage.clone(), executor, registry, bus, &config);

    let conversation_id = Uuid::new_v4();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);
    orch.run_turn_with_tools_streaming(
        "go",
        conversation_id,
        Interface::Cli,
        vec![],
        None,
        vec![],
        tx,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    Run {
        events,
        storage,
        llm,
        conversation_id,
    }
}

async fn run(tool: &'static str, output: OutputFn) -> Run {
    run_with(vec![], tool, output, false).await
}

#[tokio::test]
async fn tool_data_reaches_the_event() {
    let run = run(
        "probe",
        Box::new(|| ToolOutput::success("hi").with_data(json!({"kind": "terminal"}))),
    )
    .await;
    let results = run.tool_results();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "ok");
    assert_eq!(results[0].2, Some(json!({"kind": "terminal"})));
}

#[tokio::test]
async fn tool_without_data_has_none() {
    let run = run("probe", Box::new(|| ToolOutput::success("hi"))).await;
    assert_eq!(run.tool_results()[0].2, None);
}

#[tokio::test]
async fn data_over_the_cap_is_dropped_and_result_stays() {
    let big = "x".repeat(40 * 1024);
    let run = run(
        "probe",
        Box::new(move || ToolOutput::success("kept").with_data(json!({"blob": big.clone()}))),
    )
    .await;
    let (status, result, data) = run.tool_results().remove(0);
    assert_eq!(status, "ok");
    assert_eq!(result.as_deref(), Some("kept"));
    assert_eq!(data, None, "oversized data is dropped whole");
}

#[tokio::test]
async fn data_exactly_at_the_cap_is_kept() {
    // `{"b":"…"}` is 8 bytes of JSON around the string payload.
    let payload = "x".repeat(32 * 1024 - 8);
    let expected = json!({"b": payload.clone()});
    assert_eq!(expected.to_string().len(), 32 * 1024);
    let run = run(
        "probe",
        Box::new(move || ToolOutput::success("ok").with_data(json!({"b": payload.clone()}))),
    )
    .await;
    assert_eq!(run.tool_results()[0].2, Some(expected));
}

#[tokio::test]
async fn multibyte_char_straddling_the_display_limit_does_not_panic() {
    // 511 ASCII bytes, then a 3-byte char covering bytes 511..514.
    let content = format!("{}€€€", "a".repeat(511));
    let expected_cut = "a".repeat(511);
    let run = run(
        "probe",
        Box::new(move || ToolOutput::success(content.clone())),
    )
    .await;
    let result = run.tool_results()[0].1.clone().unwrap();
    assert_eq!(result, format!("{expected_cut}…"));
}

#[tokio::test]
async fn llm_history_holds_content_not_data() {
    let run = run(
        "probe",
        Box::new(|| ToolOutput::success("visible").with_data(json!({"secret": "SENTINEL"}))),
    )
    .await;
    let calls = run.llm.recorded_calls();
    let history = &calls.last().expect("a follow-up LLM call").history;
    let tool_content: Vec<&str> = history
        .iter()
        .filter_map(|m| match m {
            ChatHistoryMessage::ToolResult { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(tool_content, vec!["visible"]);
    assert_eq!(run.persisted_tool_content().await, vec!["visible"]);
}

#[tokio::test]
async fn denied_call_has_no_data() {
    // No allow rule: a non-interactive-style denial of an undecided tool.
    let run = run_with(
        vec![],
        "probe",
        Box::new(|| ToolOutput::success("x").with_data(json!({"kind": "terminal"}))),
        true,
    )
    .await;
    let (status, _, data) = run.tool_results().remove(0);
    assert_eq!(status, "denied");
    assert_eq!(data, None);
}

#[tokio::test]
async fn tool_error_output_reports_status_error_in_event_and_webhook() {
    let hook = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&hook)
        .await;

    let llm = Arc::new(ScriptedLlmProvider::new().with_canned_responses(vec![
        call("probe"),
        LlmResponse::FinalAnswer("done".to_string(), LlmResponseMeta::default()),
    ]));
    let llm_dyn: Arc<dyn LlmProvider> = llm.clone();
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.permissions.allow = vec!["*".parse().expect("valid rule")];
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    storage
        .webhook_store_for_agent(&config.agent.id)
        .create(
            "hook-1",
            "hook",
            &hook.uri(),
            "secret",
            &["tool.result".to_string()],
        )
        .await
        .unwrap();
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm_dyn.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    executor.register_ambient_tool(Arc::new(StubTool {
        name: "probe",
        output: Box::new(|| ToolOutput::error("boom")),
    }));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Orchestrator::new(llm_dyn, storage, executor, registry, bus, &config);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);
    orch.run_turn_with_tools_streaming(
        "go",
        Uuid::new_v4(),
        Interface::Cli,
        vec![],
        None,
        vec![],
        tx,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let mut statuses = Vec::new();
    while let Ok(event) = rx.try_recv() {
        if let OrchestratorEvent::ToolResult { status, .. } = event {
            statuses.push(status);
        }
    }
    assert_eq!(statuses, vec!["error"]);

    let requests = hook.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "one webhook delivery");
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["status"], "error");
    assert_eq!(body["observation"], "boom");
}

#[tokio::test]
async fn failed_output_with_content_is_error_and_failed_skill_execution() {
    let run = run_with(
        vec![call_with("load-skill", json!({"name": "alpha"}))],
        "probe",
        Box::new(|| ToolOutput {
            content: "exit code 2".to_string(),
            success: false,
            data: None,
            attachments: Vec::new(),
        }),
        false,
    )
    .await;
    let results = run.tool_results();
    let (status, result, _) = results.last().expect("probe result");
    assert_eq!(status, "error");
    assert_eq!(result.as_deref(), Some("exit code 2"));
    assert_eq!(
        run.persisted_tool_content()
            .await
            .last()
            .map(String::as_str),
        Some("exit code 2")
    );
    let recorded = run
        .storage
        .skill_execution_store()
        .recent("alpha", 10)
        .await
        .unwrap();
    let summary: Vec<(&str, SkillOutcome, Option<&str>)> = recorded
        .iter()
        .map(|e| (e.tool_name.as_str(), e.outcome, e.error.as_deref()))
        .collect();
    assert_eq!(
        summary,
        vec![("probe", SkillOutcome::Error, Some("exit code 2"))],
        "a failed output counts as a failed skill execution"
    );
}

// ── persisted tool-row metadata ───────────────────────────────────────────

impl Run {
    /// The persisted tool rows as `(tool_call_id, tool_status, tool_data)`.
    async fn persisted_tool_meta(&self) -> Vec<(Option<String>, Option<String>, Option<Value>)> {
        self.storage
            .conversation_store()
            .load_history(self.conversation_id)
            .await
            .unwrap()
            .into_iter()
            .filter(|m| matches!(m.role, MessageRole::Tool))
            .map(|m| {
                (
                    m.tool_call_id,
                    m.tool_status,
                    m.tool_data_json
                        .map(|raw| serde_json::from_str(&raw).unwrap()),
                )
            })
            .collect()
    }
}

#[tokio::test]
async fn ok_call_persists_id_status_and_data() {
    let run = run(
        "probe",
        Box::new(|| ToolOutput::success("hi").with_data(json!({"kind": "terminal"}))),
    )
    .await;
    assert_eq!(
        run.persisted_tool_meta().await,
        vec![(
            Some("call_1".to_string()),
            Some("ok".to_string()),
            Some(json!({"kind": "terminal"}))
        )]
    );
}

#[tokio::test]
async fn data_over_the_cap_is_not_persisted() {
    let big = "x".repeat(40 * 1024);
    let run = run(
        "probe",
        Box::new(move || ToolOutput::success("hi").with_data(json!({"blob": big.clone()}))),
    )
    .await;
    let meta = run.persisted_tool_meta().await;
    assert_eq!(meta[0].1.as_deref(), Some("ok"));
    assert_eq!(meta[0].2, None, "oversized data is dropped, not persisted");
}

// ── MCP apps ──────────────────────────────────────────────────────────────

fn weather_app(html: String) -> McpApp {
    McpApp {
        server: "weather".into(),
        resource_uri: "ui://weather/forecast".into(),
        html,
        csp: Default::default(),
        permissions: Default::default(),
        prefers_border: None,
    }
}

fn app_output(html: String) -> OutputFn {
    Box::new(move || {
        let mut output = ToolOutput::success("21 degrees");
        output.data = weather_app(html.clone()).attach_to(Some(json!({"temp": 21})));
        output
    })
}

impl Run {
    fn tool_result_apps(&self) -> Vec<Option<McpApp>> {
        self.events
            .iter()
            .filter_map(|e| match e {
                OrchestratorEvent::ToolResult { app, .. } => Some(app.clone()),
                _ => None,
            })
            .collect()
    }

    async fn persisted_tool_apps(&self) -> Vec<Option<McpApp>> {
        self.storage
            .conversation_store()
            .load_history(self.conversation_id)
            .await
            .unwrap()
            .into_iter()
            .filter(|m| matches!(m.role, MessageRole::Tool))
            .map(|m| {
                m.tool_app_json
                    .map(|raw| serde_json::from_str(&raw).unwrap())
            })
            .collect()
    }
}

#[tokio::test]
async fn mcp_app_rides_on_the_event_apart_from_the_data() {
    let run = run("probe", app_output("<p>sunny</p>".into())).await;
    assert_eq!(
        run.tool_result_apps(),
        vec![Some(weather_app("<p>sunny</p>".into()))]
    );
    assert_eq!(
        run.tool_results()[0].2,
        Some(json!({"temp": 21})),
        "the app is taken out of the data"
    );
}

#[tokio::test]
async fn mcp_app_is_persisted_with_the_tool_row() {
    let run = run("probe", app_output("<p>sunny</p>".into())).await;
    assert_eq!(
        run.persisted_tool_apps().await,
        vec![Some(weather_app("<p>sunny</p>".into()))]
    );
    assert_eq!(
        run.persisted_tool_meta().await[0].2,
        Some(json!({"temp": 21})),
        "the persisted data does not repeat the app"
    );
}

#[tokio::test]
async fn mcp_app_larger_than_the_data_cap_is_kept() {
    let html = "x".repeat(40 * 1024);
    let run = run("probe", app_output(html.clone())).await;
    assert_eq!(run.tool_result_apps(), vec![Some(weather_app(html))]);
}

#[tokio::test]
async fn denied_call_has_no_app() {
    let run = run_with(vec![], "probe", app_output("<p>sunny</p>".into()), true).await;
    assert_eq!(run.tool_result_apps(), vec![None]);
}

#[tokio::test]
async fn tool_error_output_persists_status_error() {
    let run = run("probe", Box::new(|| ToolOutput::error("boom"))).await;
    assert_eq!(
        run.persisted_tool_meta().await,
        vec![(Some("call_1".to_string()), Some("error".to_string()), None)]
    );
}

/// A handler whose execution fails outright.
struct FailingTool;

#[async_trait]
impl ToolHandler for FailingTool {
    fn name(&self) -> &str {
        "probe"
    }
    fn description(&self) -> &str {
        "always fails"
    }
    fn params_schema(&self) -> Value {
        json!({"type": "object", "properties": {}, "required": []})
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        anyhow::bail!("exploded")
    }
}

#[tokio::test]
async fn executor_err_persists_status_error() {
    let llm = Arc::new(ScriptedLlmProvider::new().with_canned_responses(vec![
        call("probe"),
        LlmResponse::FinalAnswer("done".to_string(), LlmResponseMeta::default()),
    ]));
    let llm_dyn: Arc<dyn LlmProvider> = llm.clone();
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.permissions.allow = vec!["*".parse().expect("valid rule")];
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm_dyn.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    executor.register_ambient_tool(Arc::new(FailingTool));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Orchestrator::new(llm_dyn, storage.clone(), executor, registry, bus, &config);

    let conversation_id = Uuid::new_v4();
    let (tx, _rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);
    orch.run_turn_with_tools_streaming(
        "go",
        conversation_id,
        Interface::Cli,
        vec![],
        None,
        vec![],
        tx,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let run = Run {
        events: Vec::new(),
        storage,
        llm,
        conversation_id,
    };
    assert_eq!(
        run.persisted_tool_meta().await,
        vec![(Some("call_1".to_string()), Some("error".to_string()), None)]
    );
}

#[tokio::test]
async fn denied_call_persists_status_denied_without_data() {
    let run = run_with(
        vec![],
        "probe",
        Box::new(|| ToolOutput::success("x").with_data(json!({"kind": "terminal"}))),
        true,
    )
    .await;
    assert_eq!(
        run.persisted_tool_meta().await,
        vec![(Some("call_1".to_string()), Some("denied".to_string()), None)]
    );
}

#[tokio::test]
async fn end_turn_row_persists_status_ok_without_data() {
    let run = run("end_turn", Box::new(|| ToolOutput::success("unused"))).await;
    let meta = run.persisted_tool_meta().await;
    assert_eq!(meta.len(), 1, "one end_turn tool row");
    assert_eq!(meta[0].1.as_deref(), Some("ok"));
    assert_eq!(meta[0].2, None);
}
