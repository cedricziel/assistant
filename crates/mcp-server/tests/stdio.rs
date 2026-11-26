//! Stdout carries only protocol frames; logs go to stderr.
//!
//! The harness is off so the test binary can re-run itself as the server
//! process (`--serve`) on its real stdin and stdout. The parent drives a
//! session over pipes and inspects both output streams byte for byte.

#![allow(clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;

use assistant_core::permission::PermissionPolicy;
use assistant_mcp_server::{AssistantMcpServer, ListChangeListeners, serve_stdio};
use assistant_runtime::StubOrchestrationEngine;
use assistant_storage::InMemorySkillCatalog;
use assistant_tool_executor::StubToolDispatcher;
use serde_json::{Value, json};

fn main() {
    if std::env::args().any(|arg| arg == "--serve") {
        serve_child();
    } else {
        logs_do_not_corrupt_the_stdout_stream();
        println!("test stdio ... ok");
    }
}

/// The server process: logs everything to stderr, serves on real stdio.
fn serve_child() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let server = AssistantMcpServer::new(
        Arc::new(StubOrchestrationEngine::new()),
        Arc::new(StubToolDispatcher::new()),
        Arc::new(InMemorySkillCatalog::new()),
        PermissionPolicy::default(),
        std::env::temp_dir(),
        ListChangeListeners::default(),
    );
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(serve_stdio(server)).expect("serve stdio");
}

fn logs_do_not_corrupt_the_stdout_stream() {
    let requests = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "stdio-test", "version": "0"},
        }}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        // The default policy leaves this undecided, so the handler logs a refusal.
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
            "name": "echo", "arguments": {},
        }}),
    ];

    let mut child = Command::new(std::env::current_exe().expect("test binary path"))
        .arg("--serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn server process");
    let mut stdin = child.stdin.take().expect("child stdin");
    for request in &requests {
        writeln!(stdin, "{request}").expect("write request");
    }
    drop(stdin);
    let output = child.wait_with_output().expect("server output");

    assert!(
        output.status.success(),
        "server exited with {}",
        output.status
    );

    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let frames: Vec<Value> = stdout
        .lines()
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("stdout line is not JSON ({e}): {line:?}"))
        })
        .collect();
    assert_eq!(frames.len(), 2, "one response per request: {stdout}");
    assert!(
        frames.iter().all(|frame| frame["jsonrpc"] == "2.0"),
        "{stdout}"
    );
    assert_eq!(frames[1]["result"]["isError"], true, "{stdout}");

    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(
        stderr.contains("MCP tool call refused"),
        "the handler's log line must reach stderr: {stderr}"
    );
}
