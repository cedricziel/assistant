//! Lifecycle tests for serving [`AssistantMcpServer`] over a byte pipe.
//!
//! An rmcp client talks to `serve_transport` over `tokio::io::duplex`, the
//! same transport shape stdio uses. Together with `dispatch.rs` this lifts
//! `crates/mcp-server` above the 80% line floor.

mod common;

use std::sync::Arc;

use assistant_mcp_server::serve_transport;
use assistant_runtime::StubOrchestrationEngine;
use common::*;
use rmcp::ServiceExt;
use serde_json::json;

#[tokio::test]
async fn serve_transport_returns_cleanly_when_the_peer_hangs_up_immediately() {
    let (server_io, client_io) = tokio::io::duplex(4096);
    drop(client_io);
    let server = default_stubs().server(allow_everything());
    serve_transport(server, server_io)
        .await
        .expect("EOF before the handshake is a clean shutdown");
}

#[tokio::test]
async fn serve_transport_returns_when_the_client_disconnects_after_a_session() {
    let (server_io, client_io) = tokio::io::duplex(4096);
    let server = default_stubs().server(allow_everything());
    let serving = tokio::spawn(serve_transport(server, server_io));

    let client = ().serve(client_io).await.unwrap();
    client.list_all_tools().await.unwrap();
    client.cancel().await.unwrap();

    serving.await.unwrap().expect("server shuts down cleanly");
}

#[tokio::test]
async fn initialize_negotiates_a_revision_and_names_the_server() {
    let client = default_stubs().connect().await;
    let info = client.peer_info().expect("peer info");
    assert_eq!(info.protocol_version.as_str(), "2025-11-25");
    assert_eq!(
        info.server_info.as_ref().map(|i| i.name.as_str()),
        Some("assistant-mcp-server")
    );
}

#[tokio::test]
async fn processes_multiple_requests_in_sequence() {
    let orch = Arc::new(
        StubOrchestrationEngine::new()
            .queue_turn(Ok(turn("first answer")))
            .queue_turn(Ok(turn("second answer"))),
    );
    let client = default_stubs().orchestrator(orch).connect().await;

    let call = |prompt: &'static str| {
        client.call_tool(call_params("run-prompt", json!({ "prompt": prompt })))
    };
    let a = call("a").await.unwrap();
    let b = call("b").await.unwrap();

    assert!(text_of(&a).contains("first"));
    assert!(text_of(&b).contains("second"));
}
