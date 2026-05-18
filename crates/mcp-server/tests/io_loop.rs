//! Integration tests for the stdio JSON-RPC loop (`run_io`).
//!
//! Pipes canned newline-delimited JSON-RPC requests through an in-memory
//! reader and captures responses from an in-memory writer. Together with
//! `dispatch.rs` this lifts `crates/mcp-server` above the 80% line floor.

use std::sync::Arc;

use assistant_core::tool::ToolDispatcher;
use assistant_mcp_server::run_io;
use assistant_runtime::{OrchestrationEngine, StubOrchestrationEngine, TurnResult};
use assistant_storage::{InMemorySkillCatalog, SkillCatalog, SkillRegistry, StorageLayer};
use assistant_tool_executor::StubToolDispatcher;
use tokio::io::BufReader;

async fn make_install_registry() -> Arc<SkillRegistry> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap())
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
async fn run_io_handles_eof_immediately() {
    // Empty input — loop should exit on the very first read.
    let (o, e, c) = default_stubs();
    let install = make_install_registry().await;
    let input: &[u8] = b"";
    let mut output: Vec<u8> = Vec::new();
    run_io(
        BufReader::new(input),
        &mut output,
        o,
        e,
        c,
        install,
        std::path::PathBuf::from("/tmp"),
    )
    .await
    .unwrap();
    assert!(output.is_empty(), "no responses expected, got {output:?}");
}

#[tokio::test]
async fn run_io_handles_initialize_and_returns_newline_delimited_response() {
    let (o, e, c) = default_stubs();
    let install = make_install_registry().await;

    let request = br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(request);
    input.push(b'\n');

    let mut output: Vec<u8> = Vec::new();
    run_io(
        BufReader::new(input.as_slice()),
        &mut output,
        o,
        e,
        c,
        install,
        std::path::PathBuf::from("/tmp"),
    )
    .await
    .unwrap();

    let text = String::from_utf8(output).unwrap();
    assert!(text.ends_with('\n'));
    let parsed: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(parsed["id"], 1);
    assert_eq!(parsed["result"]["protocolVersion"], "2024-11-05");
}

#[tokio::test]
async fn run_io_skips_blank_lines_between_requests() {
    let (o, e, c) = default_stubs();
    let install = make_install_registry().await;

    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"\n\n");
    input.extend_from_slice(br#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#);
    input.push(b'\n');
    input.extend_from_slice(b"\n");

    let mut output: Vec<u8> = Vec::new();
    run_io(
        BufReader::new(input.as_slice()),
        &mut output,
        o,
        e,
        c,
        install,
        std::path::PathBuf::from("/tmp"),
    )
    .await
    .unwrap();

    let lines: Vec<&str> = std::str::from_utf8(&output)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(lines.len(), 1, "expected exactly one response: {output:?}");
    let parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(parsed["id"], 1);
}

#[tokio::test]
async fn run_io_garbage_input_returns_parse_error() {
    let (o, e, c) = default_stubs();
    let install = make_install_registry().await;

    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"this-is-not-json\n");

    let mut output: Vec<u8> = Vec::new();
    run_io(
        BufReader::new(input.as_slice()),
        &mut output,
        o,
        e,
        c,
        install,
        std::path::PathBuf::from("/tmp"),
    )
    .await
    .unwrap();

    let text = std::str::from_utf8(&output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(parsed["error"]["code"], -32700);
    assert_eq!(parsed["error"]["message"], "Parse error");
}

#[tokio::test]
async fn run_io_processes_multiple_requests_in_sequence() {
    let orch = Arc::new(
        StubOrchestrationEngine::new()
            .queue_turn(Ok(TurnResult {
                answer: "first answer".into(),
                attachments: Vec::new(),
                attachment_ids: Vec::new(),
                message_id: None,
                had_errors: false,
            }))
            .queue_turn(Ok(TurnResult {
                answer: "second answer".into(),
                attachments: Vec::new(),
                attachment_ids: Vec::new(),
                message_id: None,
                had_errors: false,
            })),
    ) as Arc<dyn OrchestrationEngine>;
    let exec = Arc::new(StubToolDispatcher::new()) as Arc<dyn ToolDispatcher>;
    let cat = Arc::new(InMemorySkillCatalog::new()) as Arc<dyn SkillCatalog>;
    let install = make_install_registry().await;

    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(
        br#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"run-prompt","arguments":{"prompt":"a"}}}"#,
    );
    input.push(b'\n');
    input.extend_from_slice(
        br#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"run-prompt","arguments":{"prompt":"b"}}}"#,
    );
    input.push(b'\n');

    let mut output: Vec<u8> = Vec::new();
    run_io(
        BufReader::new(input.as_slice()),
        &mut output,
        orch,
        exec,
        cat,
        install,
        std::path::PathBuf::from("/tmp"),
    )
    .await
    .unwrap();

    let lines: Vec<&str> = std::str::from_utf8(&output)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(lines.len(), 2);
    let a: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let b: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(a["id"], 1);
    assert_eq!(b["id"], 2);
    assert!(
        a["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("first")
    );
    assert!(
        b["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("second")
    );
}
