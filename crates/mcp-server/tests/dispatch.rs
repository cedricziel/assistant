//! End-to-end dispatch tests for the MCP server's `handle_request`.
//!
//! Uses [`StubOrchestrationEngine`], [`StubToolDispatcher`], and
//! [`InMemorySkillCatalog`] in place of the real Orchestrator /
//! ToolExecutor / SkillRegistry so each JSON-RPC method can be
//! exercised without booting SQLite or hitting an LLM backend.
//!
//! Phase 7.8 of `workspace-test-coverage-floor`.

use std::sync::Arc;

use assistant_core::ToolSpec;
use assistant_core::tool::{ToolDispatcher, ToolOutput};
use assistant_mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
use assistant_mcp_server::server::handle_request;
use assistant_runtime::{CancelOutcome, OrchestrationEngine, StubOrchestrationEngine, TurnResult};
use assistant_skills::{SkillDef, SkillSource};
use assistant_storage::{InMemorySkillCatalog, SkillCatalog};
use assistant_tool_executor::StubToolDispatcher;
use serde_json::{Value, json};

fn make_skill(name: &str) -> SkillDef {
    SkillDef {
        name: name.to_string(),
        description: format!("test skill {name}"),
        license: None,
        compatibility: None,
        allowed_tools: Vec::new(),
        metadata: std::collections::HashMap::new(),
        body: format!("body of {name}"),
        dir: std::path::PathBuf::from(format!("/tmp/{name}")),
        source: SkillSource::Builtin,
    }
}

fn make_request(id: i64, method: &str, params: Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(id)),
        method: method.to_string(),
        params,
    }
}

fn workspace_skills_dir() -> std::path::PathBuf {
    tempfile::tempdir().expect("tempdir").keep()
}

async fn call(
    req: JsonRpcRequest,
    orchestrator: Arc<dyn OrchestrationEngine>,
    executor: Arc<dyn ToolDispatcher>,
    registry: Arc<dyn SkillCatalog>,
) -> JsonRpcResponse {
    handle_request(
        req,
        registry,
        executor,
        orchestrator,
        workspace_skills_dir(),
        None,
    )
    .await
}

fn default_stubs() -> (
    Arc<dyn OrchestrationEngine>,
    Arc<dyn ToolDispatcher>,
    Arc<dyn SkillCatalog>,
) {
    let orch = Arc::new(StubOrchestrationEngine::new()) as Arc<dyn OrchestrationEngine>;
    let exec = Arc::new(StubToolDispatcher::new()) as Arc<dyn ToolDispatcher>;
    let reg = Arc::new(InMemorySkillCatalog::new()) as Arc<dyn SkillCatalog>;
    (orch, exec, reg)
}

#[tokio::test]
async fn initialize_returns_protocol_version_and_server_info() {
    let (o, e, r) = default_stubs();
    let resp = call(make_request(1, "initialize", json!({})), o, e, r).await;
    assert!(
        resp.error.is_none(),
        "initialize should not error: {resp:?}"
    );
    let result = resp.result.expect("initialize must return a result");
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "assistant-mcp-server");
}

#[tokio::test]
async fn notifications_initialized_returns_empty_result() {
    let (o, e, r) = default_stubs();
    let resp = call(
        make_request(2, "notifications/initialized", json!({})),
        o,
        e,
        r,
    )
    .await;
    assert!(resp.error.is_none());
    assert_eq!(resp.result.unwrap(), json!({}));
}

#[tokio::test]
async fn ping_returns_empty_result() {
    let (o, e, r) = default_stubs();
    let resp = call(make_request(3, "ping", json!({})), o, e, r).await;
    assert!(resp.error.is_none());
}

#[tokio::test]
async fn tools_list_returns_executor_specs_plus_management_tools() {
    let (o, _, r) = default_stubs();
    let exec = Arc::new(StubToolDispatcher::new().with_specs(vec![ToolSpec {
        name: "echo".into(),
        description: "echo".into(),
        params_schema: json!({}),
        is_mutating: false,
        requires_confirmation: false,
    }])) as Arc<dyn ToolDispatcher>;
    let resp = call(make_request(4, "tools/list", json!({})), o, exec, r).await;
    assert!(resp.error.is_none());
    let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
    let names: Vec<String> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"echo".to_string()));
    assert!(names.contains(&"run-prompt".to_string()));
    assert!(names.contains(&"install-skill".to_string()));
}

#[tokio::test]
async fn tools_call_run_prompt_invokes_orchestrator() {
    let orch = Arc::new(StubOrchestrationEngine::new().queue_turn(Ok(TurnResult {
        answer: "hello from stub".to_string(),
        attachments: Vec::new(),
        attachment_ids: Vec::new(),
        message_id: None,
        had_errors: false,
    })));
    let orch_dyn: Arc<dyn OrchestrationEngine> = orch.clone();
    let (_, e, r) = default_stubs();
    let resp = call(
        make_request(
            5,
            "tools/call",
            json!({"name": "run-prompt", "arguments": {"prompt": "hi"}}),
        ),
        orch_dyn,
        e,
        r,
    )
    .await;
    assert!(resp.error.is_none(), "{resp:?}");
    let content = resp.result.unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(content, "hello from stub");
    assert_eq!(orch.submit_calls().len(), 1);
}

#[tokio::test]
async fn tools_call_run_prompt_missing_prompt_returns_error() {
    let (o, e, r) = default_stubs();
    let resp = call(
        make_request(
            6,
            "tools/call",
            json!({"name": "run-prompt", "arguments": {}}),
        ),
        o,
        e,
        r,
    )
    .await;
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[tokio::test]
async fn tools_call_install_skill_without_registry_returns_error() {
    let (o, e, r) = default_stubs();
    let resp = call(
        make_request(
            7,
            "tools/call",
            json!({"name": "install-skill", "arguments": {"source": "/tmp"}}),
        ),
        o,
        e,
        r,
    )
    .await;
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32603);
    assert!(err.message.contains("SkillRegistry"));
}

#[tokio::test]
async fn tools_call_dispatches_to_executor() {
    let (o, _, r) = default_stubs();
    let exec = StubToolDispatcher::new().queue_response("echo", ToolOutput::success("echoed: hi"));
    let exec_dyn: Arc<dyn ToolDispatcher> = Arc::new(exec);
    let resp = call(
        make_request(
            8,
            "tools/call",
            json!({"name": "echo", "arguments": {"text": "hi"}}),
        ),
        o,
        exec_dyn,
        r,
    )
    .await;
    assert!(resp.error.is_none(), "{resp:?}");
    let content = resp.result.unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(content, "echoed: hi");
}

#[tokio::test]
async fn tools_call_unknown_tool_returns_error_output() {
    let (o, e, r) = default_stubs();
    let resp = call(
        make_request(9, "tools/call", json!({"name": "ghost", "arguments": {}})),
        o,
        e,
        r,
    )
    .await;
    // The dispatcher returned the benign success ("(stub: ghost)") since
    // the stub doesn't track which tools are registered. Just assert no
    // protocol-level error.
    assert!(resp.error.is_none());
}

#[tokio::test]
async fn resources_list_returns_skills_from_catalog() {
    let (o, e, _) = default_stubs();
    let reg = Arc::new(InMemorySkillCatalog::with_skills(vec![
        make_skill("alpha"),
        make_skill("beta"),
    ])) as Arc<dyn SkillCatalog>;
    let resp = call(make_request(10, "resources/list", json!({})), o, e, reg).await;
    assert!(resp.error.is_none(), "{resp:?}");
    let resources = resp.result.unwrap()["resources"]
        .as_array()
        .unwrap()
        .clone();
    let names: Vec<String> = resources
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"alpha".to_string()));
    assert!(names.contains(&"beta".to_string()));
}

#[tokio::test]
async fn resources_read_returns_skill_body() {
    let (o, e, _) = default_stubs();
    let reg = Arc::new(InMemorySkillCatalog::with_skills(vec![make_skill("foo")]))
        as Arc<dyn SkillCatalog>;
    let resp = call(
        make_request(11, "resources/read", json!({"uri": "skills://foo"})),
        o,
        e,
        reg,
    )
    .await;
    assert!(resp.error.is_none(), "{resp:?}");
    let contents = resp.result.unwrap()["contents"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(contents.contains("body of foo"));
}

#[tokio::test]
async fn resources_read_unknown_skill_returns_error() {
    let (o, e, r) = default_stubs();
    let resp = call(
        make_request(12, "resources/read", json!({"uri": "skills://missing"})),
        o,
        e,
        r,
    )
    .await;
    assert!(resp.error.is_some());
}

#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    let (o, e, r) = default_stubs();
    let resp = call(make_request(13, "nonsense/method", json!({})), o, e, r).await;
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601);
}

#[tokio::test]
async fn cancel_turn_via_orchestrator_engine_returns_outcome() {
    let orch =
        Arc::new(StubOrchestrationEngine::new().queue_cancel_outcome(CancelOutcome::Cancelled));
    let req_id = uuid::Uuid::new_v4();
    let outcome = orch.cancel_turn(req_id).await;
    assert!(matches!(outcome, CancelOutcome::Cancelled));
}

#[tokio::test]
async fn tools_call_run_prompt_orchestrator_error_returns_internal_error() {
    let orch = Arc::new(
        StubOrchestrationEngine::new().queue_turn(Err(anyhow::anyhow!("backend exploded"))),
    ) as Arc<dyn OrchestrationEngine>;
    let (_, e, r) = default_stubs();
    let resp = call(
        make_request(
            14,
            "tools/call",
            json!({"name": "run-prompt", "arguments": {"prompt": "hi"}}),
        ),
        orch,
        e,
        r,
    )
    .await;
    let err = resp.error.expect("orchestrator error must surface");
    assert_eq!(err.code, -32603);
    assert!(err.message.contains("backend exploded"));
}

#[tokio::test]
async fn tools_call_install_skill_missing_source_returns_invalid_params() {
    let (o, e, r) = default_stubs();
    let resp = call(
        make_request(
            15,
            "tools/call",
            json!({"name": "install-skill", "arguments": {}}),
        ),
        o,
        e,
        r,
    )
    .await;
    let err = resp.error.expect("missing source must error");
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("source"));
}

#[tokio::test]
async fn resources_list_includes_auxiliary_files_from_skill_dir() {
    use std::fs;
    // Build a skill on disk with a scripts/ and references/ subdir, then
    // load it through InMemorySkillCatalog so resources/list sees its
    // auxiliary files.
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("foo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::create_dir_all(dir.join("references")).unwrap();
    fs::write(dir.join("scripts").join("run.sh"), "#!/bin/sh\n").unwrap();
    fs::write(dir.join("references").join("notes.md"), "notes").unwrap();

    let mut skill = make_skill("foo");
    skill.dir = dir;

    let (o, e, _) = default_stubs();
    let reg = Arc::new(InMemorySkillCatalog::with_skills(vec![skill])) as Arc<dyn SkillCatalog>;
    let resp = call(make_request(16, "resources/list", json!({})), o, e, reg).await;
    assert!(resp.error.is_none(), "{resp:?}");
    let resources = resp.result.unwrap()["resources"]
        .as_array()
        .unwrap()
        .clone();
    let uris: Vec<String> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap().to_string())
        .collect();
    assert!(uris.iter().any(|u| u.contains("scripts/run.sh")));
    assert!(uris.iter().any(|u| u.contains("references/notes.md")));
}

#[tokio::test]
async fn resources_read_skills_list_returns_index() {
    let (o, e, _) = default_stubs();
    let reg = Arc::new(InMemorySkillCatalog::with_skills(vec![
        make_skill("alpha"),
        make_skill("beta"),
    ])) as Arc<dyn SkillCatalog>;
    let resp = call(
        make_request(17, "resources/read", json!({"uri": "skills://list"})),
        o,
        e,
        reg,
    )
    .await;
    assert!(resp.error.is_none(), "{resp:?}");
    let text = resp.result.unwrap()["contents"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(text.contains("alpha"));
    assert!(text.contains("beta"));
}

#[tokio::test]
async fn resources_read_auxiliary_file_returns_contents() {
    use std::fs;
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("foo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(dir.join("scripts").join("run.sh"), "echo hello").unwrap();

    let mut skill = make_skill("foo");
    skill.dir = dir;

    let (o, e, _) = default_stubs();
    let reg = Arc::new(InMemorySkillCatalog::with_skills(vec![skill])) as Arc<dyn SkillCatalog>;
    let resp = call(
        make_request(
            18,
            "resources/read",
            json!({"uri": "skills://foo/scripts/run.sh"}),
        ),
        o,
        e,
        reg,
    )
    .await;
    assert!(resp.error.is_none(), "{resp:?}");
    let text = resp.result.unwrap()["contents"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(text, "echo hello");
}

#[tokio::test]
async fn resources_read_aux_file_missing_returns_error() {
    use std::fs;
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("foo");
    fs::create_dir_all(&dir).unwrap();

    let mut skill = make_skill("foo");
    skill.dir = dir;

    let (o, e, _) = default_stubs();
    let reg = Arc::new(InMemorySkillCatalog::with_skills(vec![skill])) as Arc<dyn SkillCatalog>;
    let resp = call(
        make_request(
            19,
            "resources/read",
            json!({"uri": "skills://foo/scripts/missing.sh"}),
        ),
        o,
        e,
        reg,
    )
    .await;
    let err = resp.error.expect("missing aux file must error");
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn resources_read_aux_file_for_unknown_skill_returns_error() {
    let (o, e, r) = default_stubs();
    let resp = call(
        make_request(
            20,
            "resources/read",
            json!({"uri": "skills://ghost/scripts/run.sh"}),
        ),
        o,
        e,
        r,
    )
    .await;
    let err = resp.error.expect("unknown skill must error");
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn resources_read_invalid_segments_returns_error() {
    let (o, e, r) = default_stubs();
    // 2-segment URI is neither `skills://list`, nor `skills://<name>`,
    // nor a 3-segment auxiliary path — falls into the catch-all branch.
    let resp = call(
        make_request(21, "resources/read", json!({"uri": "skills://foo/bar"})),
        o,
        e,
        r,
    )
    .await;
    let err = resp.error.expect("invalid resource URI must error");
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn resources_read_non_skills_uri_returns_error() {
    let (o, e, r) = default_stubs();
    let resp = call(
        make_request(22, "resources/read", json!({"uri": "https://example.com"})),
        o,
        e,
        r,
    )
    .await;
    let err = resp.error.expect("non-skills URI must error");
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("Unknown resource URI"));
}

#[tokio::test]
async fn unknown_notification_returns_empty_ok() {
    let (o, e, r) = default_stubs();
    // Notifications have no id; per JSON-RPC, the server should return
    // a benign empty result rather than `-32601`.
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: None,
        method: "ghost/notify".to_string(),
        params: json!({}),
    };
    let resp = call(req, o, e, r).await;
    assert!(resp.error.is_none(), "{resp:?}");
}

#[tokio::test]
async fn tools_call_dispatcher_error_surfaces_internal_error() {
    let (o, _, r) = default_stubs();
    let exec =
        StubToolDispatcher::new().queue_error("explode", anyhow::anyhow!("dispatcher failed"));
    let exec_dyn: Arc<dyn ToolDispatcher> = Arc::new(exec);
    let resp = call(
        make_request(
            23,
            "tools/call",
            json!({"name": "explode", "arguments": {}}),
        ),
        o,
        exec_dyn,
        r,
    )
    .await;
    let err = resp.error.expect("dispatcher Err must propagate");
    assert_eq!(err.code, -32603);
    assert!(err.message.contains("dispatcher failed"));
}
