//! End-to-end dispatch tests for [`AssistantMcpServer`], driven through an
//! rmcp client over an in-memory pipe.
//!
//! Uses [`StubOrchestrationEngine`], [`StubToolDispatcher`], and
//! [`InMemorySkillCatalog`] in place of the real Orchestrator /
//! ToolExecutor / SkillRegistry so each MCP method can be exercised
//! without booting SQLite or hitting an LLM backend.
//!
//! Phase 7.8 of `workspace-test-coverage-floor`.

mod common;

use std::sync::Arc;

use assistant_core::ToolSpec;
use assistant_core::permission::PermissionPolicy;
use assistant_core::tool::ToolOutput;
use assistant_runtime::StubOrchestrationEngine;
use assistant_tool_executor::StubToolDispatcher;
use common::*;
use rmcp::model::{ClientRequest, ErrorCode, ReadResourceRequestParams, ServerResult};
use serde_json::json;

fn read(uri: &str) -> ReadResourceRequestParams {
    ReadResourceRequestParams::new(uri)
}

fn echo_spec() -> ToolSpec {
    ToolSpec {
        name: "echo".into(),
        description: "echo".into(),
        params_schema: json!({}),
        is_mutating: false,
        requires_confirmation: false,
    }
}

#[tokio::test]
async fn initialize_returns_server_info_and_capabilities() {
    let client = default_stubs().connect().await;
    let info = client.peer_info().expect("peer info after handshake");
    assert_eq!(
        info.server_info.as_ref().map(|i| i.name.as_str()),
        Some("assistant-mcp-server")
    );
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.resources.is_some());
}

#[tokio::test]
async fn ping_returns_empty_result() {
    let client = default_stubs().connect().await;
    let result = client
        .send_request(ClientRequest::PingRequest(Default::default()))
        .await
        .unwrap();
    assert!(matches!(result, ServerResult::EmptyResult(_)), "{result:?}");
}

#[tokio::test]
async fn tools_list_is_stable_with_management_tools_last() {
    let specs = ["zeta", "alpha"].map(|name| ToolSpec {
        name: name.into(),
        ..echo_spec()
    });
    let exec = Arc::new(StubToolDispatcher::new().with_specs(specs.to_vec()));
    let client = default_stubs().executor(exec).connect().await;

    let names = |tools: Vec<rmcp::model::Tool>| {
        tools.iter().map(|t| t.name.to_string()).collect::<Vec<_>>()
    };
    let first = names(client.list_all_tools().await.unwrap());
    let second = names(client.list_all_tools().await.unwrap());

    assert_eq!(first, second, "listing twice must not reorder tools");
    assert_eq!(first, ["zeta", "alpha", "run-prompt", "install-skill"]);
}

#[tokio::test]
async fn tools_call_run_prompt_invokes_orchestrator() {
    let orch = Arc::new(StubOrchestrationEngine::new().queue_turn(Ok(turn("hello from stub"))));
    let client = default_stubs().orchestrator(orch.clone()).connect().await;
    let result = client
        .call_tool(call_params("run-prompt", json!({"prompt": "hi"})))
        .await
        .unwrap();
    assert_eq!(text_of(&result), "hello from stub");
    assert_eq!(orch.submit_calls().len(), 1);
}

#[tokio::test]
async fn tools_call_run_prompt_missing_prompt_returns_invalid_params() {
    let client = default_stubs().connect().await;
    let err = client
        .call_tool(call_params("run-prompt", json!({})))
        .await
        .unwrap_err();
    assert_eq!(code(err), ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn tools_call_install_skill_without_registry_returns_error() {
    let client = default_stubs().connect().await;
    let err = client
        .call_tool(call_params("install-skill", json!({"source": "/tmp"})))
        .await
        .unwrap_err();
    let data = protocol_error(err);
    assert_eq!(data.code, ErrorCode::INTERNAL_ERROR);
    assert!(data.message.contains("SkillRegistry"), "{}", data.message);
}

#[tokio::test]
async fn tools_call_install_skill_missing_source_returns_invalid_params() {
    let client = default_stubs().connect().await;
    let err = client
        .call_tool(call_params("install-skill", json!({})))
        .await
        .unwrap_err();
    let data = protocol_error(err);
    assert_eq!(data.code, ErrorCode::INVALID_PARAMS);
    assert!(data.message.contains("source"), "{}", data.message);
}

#[tokio::test]
async fn tools_call_install_skill_installs_from_a_local_path() {
    let source = tempfile::tempdir().unwrap();
    let skill_dir = source.path().join("installed-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: installed-skill\ndescription: from a test\n---\nbody\n",
    )
    .unwrap();
    let skills_dir = tempfile::tempdir().unwrap();
    let stubs = default_stubs();
    let server = assistant_mcp_server::AssistantMcpServer::new(
        stubs.orchestrator,
        stubs.executor,
        stubs.catalog,
        allow_everything(),
        skills_dir.path().to_path_buf(),
    )
    .with_install_registry(install_registry().await);
    let client = connect(server).await;

    let result = client
        .call_tool(call_params(
            "install-skill",
            json!({"source": skill_dir.to_str().unwrap()}),
        ))
        .await
        .unwrap();

    assert!(text_of(&result).contains("installed-skill"));
    assert!(skills_dir.path().join("installed-skill").is_dir());
}

#[tokio::test]
async fn tools_call_dispatches_to_executor() {
    let exec = StubToolDispatcher::new().queue_response("echo", ToolOutput::success("echoed: hi"));
    let client = default_stubs().executor(Arc::new(exec)).connect().await;
    let result = client
        .call_tool(call_params("echo", json!({"text": "hi"})))
        .await
        .unwrap();
    assert_eq!(text_of(&result), "echoed: hi");
}

#[tokio::test]
async fn tools_call_unknown_tool_is_not_a_protocol_error() {
    // The stub dispatcher answers with a benign success ("(stub: ghost)")
    // since it doesn't track which tools are registered.
    let client = default_stubs().connect().await;
    client
        .call_tool(call_params("ghost", json!({})))
        .await
        .expect("no protocol-level error");
}

#[tokio::test]
async fn unimplemented_method_returns_method_not_found() {
    let client = default_stubs().connect().await;
    let err = client.list_prompts(None).await.unwrap_err();
    assert_eq!(code(err), ErrorCode::METHOD_NOT_FOUND);
}

#[tokio::test]
async fn tools_call_run_prompt_orchestrator_error_returns_internal_error() {
    let orch = Arc::new(
        StubOrchestrationEngine::new().queue_turn(Err(anyhow::anyhow!("backend exploded"))),
    );
    let client = default_stubs().orchestrator(orch).connect().await;
    let err = client
        .call_tool(call_params("run-prompt", json!({"prompt": "hi"})))
        .await
        .unwrap_err();
    let data = protocol_error(err);
    assert_eq!(data.code, ErrorCode::INTERNAL_ERROR);
    assert!(
        data.message.contains("backend exploded"),
        "{}",
        data.message
    );
}

#[tokio::test]
async fn tools_call_dispatcher_error_surfaces_internal_error() {
    let exec =
        StubToolDispatcher::new().queue_error("explode", anyhow::anyhow!("dispatcher failed"));
    let client = default_stubs().executor(Arc::new(exec)).connect().await;
    let err = client
        .call_tool(call_params("explode", json!({})))
        .await
        .unwrap_err();
    let data = protocol_error(err);
    assert_eq!(data.code, ErrorCode::INTERNAL_ERROR);
    assert!(
        data.message.contains("dispatcher failed"),
        "{}",
        data.message
    );
}

// ── Resources ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn resources_list_returns_skills_from_catalog() {
    let client = default_stubs()
        .skills(vec![make_skill("alpha"), make_skill("beta")])
        .connect()
        .await;
    let resources = client.list_all_resources().await.unwrap();
    let names: Vec<&str> = resources.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
}

#[tokio::test]
async fn resources_list_includes_auxiliary_files_from_skill_dir() {
    use std::fs;
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("foo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::create_dir_all(dir.join("references")).unwrap();
    fs::write(dir.join("scripts").join("run.sh"), "#!/bin/sh\n").unwrap();
    fs::write(dir.join("references").join("notes.md"), "notes").unwrap();
    let mut skill = make_skill("foo");
    skill.dir = dir;

    let client = default_stubs().skills(vec![skill]).connect().await;
    let resources = client.list_all_resources().await.unwrap();
    let uris: Vec<&str> = resources.iter().map(|r| r.uri.as_str()).collect();
    assert!(uris.contains(&"skills://foo/scripts/run.sh"), "{uris:?}");
    assert!(
        uris.contains(&"skills://foo/references/notes.md"),
        "{uris:?}"
    );
}

#[tokio::test]
async fn resource_templates_describe_skill_files() {
    let client = default_stubs().connect().await;
    let templates = client.list_all_resource_templates().await.unwrap();
    assert_eq!(templates.len(), 1, "{templates:?}");
    assert_eq!(
        templates[0].uri_template,
        "skills://{name}/{category}/{+path}"
    );
}

#[tokio::test]
async fn resources_read_returns_skill_body() {
    let client = default_stubs()
        .skills(vec![make_skill("foo")])
        .connect()
        .await;
    let result = client.read_resource(read("skills://foo")).await.unwrap();
    assert!(resource_text(&result).contains("body of foo"));
}

#[tokio::test]
async fn resources_read_skills_list_returns_index() {
    let client = default_stubs()
        .skills(vec![make_skill("alpha"), make_skill("beta")])
        .connect()
        .await;
    let result = client.read_resource(read("skills://list")).await.unwrap();
    let text = resource_text(&result);
    assert!(text.contains("alpha"));
    assert!(text.contains("beta"));
}

#[tokio::test]
async fn resources_read_auxiliary_file_returns_contents_and_mime_type() {
    use std::fs;
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("foo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(dir.join("scripts").join("run.sh"), "echo hello").unwrap();
    let mut skill = make_skill("foo");
    skill.dir = dir;

    let client = default_stubs().skills(vec![skill]).connect().await;
    let result = client
        .read_resource(read("skills://foo/scripts/run.sh"))
        .await
        .unwrap();
    assert_eq!(resource_text(&result), "echo hello");
    let rmcp::model::ResourceContents::TextResourceContents { mime_type, .. } = &result.contents[0]
    else {
        panic!("expected text contents");
    };
    assert_eq!(mime_type.as_deref(), Some("text/plain"));
}

#[tokio::test]
async fn resources_read_aux_file_outside_skill_dir_is_not_read() {
    use std::fs;
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("foo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(tmp.path().join("secret.txt"), "top secret").unwrap();
    let mut skill = make_skill("foo");
    skill.dir = dir;

    let client = default_stubs().skills(vec![skill]).connect().await;
    let err = client
        .read_resource(read("skills://foo/scripts/../../secret.txt"))
        .await
        .expect_err("a path escaping the skill dir must not be served");
    assert_eq!(code(err), ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn resources_read_unknown_skill_returns_not_found() {
    let client = default_stubs().connect().await;
    let err = client
        .read_resource(read("skills://missing"))
        .await
        .unwrap_err();
    assert_eq!(code(err), ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn resources_read_aux_file_missing_returns_not_found() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("foo");
    std::fs::create_dir_all(&dir).unwrap();
    let mut skill = make_skill("foo");
    skill.dir = dir;

    let client = default_stubs().skills(vec![skill]).connect().await;
    let err = client
        .read_resource(read("skills://foo/scripts/missing.sh"))
        .await
        .unwrap_err();
    assert_eq!(code(err), ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn resources_read_aux_file_for_unknown_skill_returns_not_found() {
    let client = default_stubs().connect().await;
    let err = client
        .read_resource(read("skills://ghost/scripts/run.sh"))
        .await
        .unwrap_err();
    assert_eq!(code(err), ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn resources_read_invalid_segments_returns_invalid_params() {
    let client = default_stubs().connect().await;
    // Two segments are neither `skills://list`, `skills://<name>`, nor a
    // `<name>/<category>/<file>` path.
    let err = client
        .read_resource(read("skills://foo/bar"))
        .await
        .unwrap_err();
    assert_eq!(code(err), ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn resources_read_non_skills_uri_returns_invalid_params() {
    let client = default_stubs().connect().await;
    let err = client
        .read_resource(read("https://example.com"))
        .await
        .unwrap_err();
    let data = protocol_error(err);
    assert_eq!(data.code, ErrorCode::INVALID_PARAMS);
    assert!(
        data.message.contains("Unknown resource URI"),
        "{}",
        data.message
    );
}

// ── Permission policy ─────────────────────────────────────────────────────

/// Calls `echo` through `policy`; returns the result and the stub so the
/// test can see whether the tool ran.
async fn call_echo(
    policy: PermissionPolicy,
) -> (rmcp::model::CallToolResult, Arc<StubToolDispatcher>) {
    let stub = Arc::new(
        StubToolDispatcher::new().queue_response("echo", ToolOutput::success("echoed: hi")),
    );
    let client = default_stubs()
        .executor(stub.clone())
        .connect_with_policy(policy)
        .await;
    let result = client
        .call_tool(call_params("echo", json!({"text": "hi"})))
        .await
        .expect("a refusal is a tool result, not a protocol error");
    (result, stub)
}

fn assert_refused(result: &rmcp::model::CallToolResult) -> String {
    assert_eq!(result.is_error, Some(true), "{result:?}");
    text_of(result)
}

#[tokio::test]
async fn tools_call_runs_tools_the_policy_allows() {
    let (result, stub) = call_echo(policy(&["echo"], &[])).await;
    assert_ne!(result.is_error, Some(true), "{result:?}");
    assert_eq!(stub.recorded_calls().len(), 1);
}

#[tokio::test]
async fn tools_call_refuses_tools_the_policy_denies() {
    let (result, stub) = call_echo(policy(&["*"], &["echo"])).await;
    let text = assert_refused(&result);
    assert!(text.contains("Permission denied for 'echo'"), "{text}");
    assert!(
        stub.recorded_calls().is_empty(),
        "a denied tool must not run"
    );
}

#[tokio::test]
async fn tools_call_refuses_undecided_tools_because_nobody_can_be_asked() {
    let (result, stub) = call_echo(policy(&[], &[])).await;
    let text = assert_refused(&result);
    assert!(text.contains("cannot prompt a user"), "{text}");
    assert!(
        stub.recorded_calls().is_empty(),
        "an undecided tool must not run"
    );
}

#[tokio::test]
async fn tools_call_refuses_tools_that_require_confirmation_without_an_allow_rule() {
    let spec = ToolSpec {
        name: "risky".to_string(),
        description: "needs a human".to_string(),
        params_schema: json!({"type": "object"}),
        is_mutating: true,
        requires_confirmation: true,
    };
    let stub = Arc::new(StubToolDispatcher::new().with_specs(vec![spec]));
    let client = default_stubs()
        .executor(stub.clone())
        .connect_with_policy(policy(&[], &[]))
        .await;
    let result = client
        .call_tool(call_params("risky", json!({})))
        .await
        .unwrap();
    let text = assert_refused(&result);
    assert!(text.contains("requires confirmation"), "{text}");
    assert!(stub.recorded_calls().is_empty());
}

#[tokio::test]
async fn tools_call_install_skill_is_subject_to_the_policy() {
    let client = default_stubs().connect_with_policy(policy(&[], &[])).await;
    let result = client
        .call_tool(call_params(
            "install-skill",
            json!({"source": "owner/repo"}),
        ))
        .await
        .unwrap();
    let text = assert_refused(&result);
    assert!(
        text.contains("Permission denied for 'install-skill'"),
        "{text}"
    );
}
