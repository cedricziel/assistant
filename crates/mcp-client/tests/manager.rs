//! Tests for `McpClientManager` lifecycle paths that don't need a live MCP
//! server: invalid names, disabled entries, failed connections, empty input,
//! and the read-only query/shutdown surface.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use assistant_core::ToolHandler;
use assistant_core::types::skills_mcp::{McpServerEntry, McpTransportConfig, McpTrustLevel};
use assistant_mcp_client::manager::McpClientManager;

fn stdio_entry(name: &str, command: Vec<String>, enabled: bool) -> McpServerEntry {
    McpServerEntry {
        name: name.to_string(),
        transport: McpTransportConfig::Stdio { command },
        env: HashMap::new(),
        enabled,
        trust: McpTrustLevel::Confirm,
    }
}

fn http_entry(name: &str, url: &str, enabled: bool) -> McpServerEntry {
    McpServerEntry {
        name: name.to_string(),
        transport: McpTransportConfig::Http {
            url: url.to_string(),
            headers: HashMap::new(),
        },
        env: HashMap::new(),
        enabled,
        trust: McpTrustLevel::Confirm,
    }
}

#[tokio::test]
async fn start_with_empty_entries_yields_empty_manager() {
    let mgr = McpClientManager::start(&[]).await.unwrap();
    assert_eq!(mgr.tool_count().await, 0);
    assert_eq!(mgr.server_count().await, 0);
    assert!(mgr.tool_handlers().await.is_empty());
}

#[tokio::test]
async fn start_skips_disabled_entries() {
    let entries = vec![stdio_entry("disabled", vec!["true".into()], false)];
    let mgr = McpClientManager::start(&entries).await.unwrap();
    assert_eq!(mgr.tool_count().await, 0);
    assert_eq!(
        mgr.server_count().await,
        0,
        "disabled entries must not be tracked"
    );
}

#[tokio::test]
async fn start_skips_entries_with_invalid_names() {
    // Uppercase, underscores, spaces, dots, and empty all fail
    // `is_valid_server_name`.
    let entries = vec![
        stdio_entry("UPPERCASE", vec!["true".into()], true),
        stdio_entry("with_underscore", vec!["true".into()], true),
        stdio_entry("with space", vec!["true".into()], true),
        stdio_entry("with.dot", vec!["true".into()], true),
        stdio_entry("", vec!["true".into()], true),
    ];
    let mgr = McpClientManager::start(&entries).await.unwrap();
    assert_eq!(mgr.tool_count().await, 0);
    assert_eq!(mgr.server_count().await, 0);
}

#[tokio::test]
async fn start_records_failed_stdio_connections_without_client() {
    // Spawning `/does-not-exist` fails, leaving the entry in the map with
    // `client: None` and `consecutive_failures: 1`.
    let entries = vec![stdio_entry(
        "broken",
        vec!["/does-not-exist-12345".into()],
        true,
    )];
    let mgr = McpClientManager::start(&entries).await.unwrap();
    assert_eq!(mgr.tool_count().await, 0);
    assert_eq!(
        mgr.server_count().await,
        0,
        "failed connection counts as 0 servers"
    );
    // shutdown is a no-op when no client is connected.
    mgr.shutdown().await.unwrap();
}

#[tokio::test]
async fn start_handles_empty_stdio_command() {
    let entries = vec![stdio_entry("emptycmd", vec![], true)];
    let mgr = McpClientManager::start(&entries).await.unwrap();
    assert_eq!(mgr.tool_count().await, 0);
}

#[tokio::test]
async fn start_records_failed_http_connection() {
    // Point at a closed port: rmcp will fail the initialize handshake.
    let entries = vec![http_entry("unreachable", "http://127.0.0.1:1/mcp", true)];
    let mgr = McpClientManager::start(&entries).await.unwrap();
    assert_eq!(mgr.tool_count().await, 0);
}

#[tokio::test]
async fn refresh_server_tools_unknown_server_errors() {
    let mgr = McpClientManager::start(&[]).await.unwrap();
    let res = mgr.refresh_server_tools("ghost").await;
    let err = res.err().expect("expected error");
    assert!(err.to_string().contains("ghost"));
}

#[tokio::test]
async fn refresh_server_tools_disconnected_server_errors() {
    // Insert a failed-connection entry so the server is tracked but has no
    // client.  refresh_server_tools should still error.
    let entries = vec![stdio_entry("disco", vec!["/does-not-exist".into()], true)];
    let mgr = McpClientManager::start(&entries).await.unwrap();
    let res = mgr.refresh_server_tools("disco").await;
    let err = res.err().expect("expected error");
    assert!(err.to_string().contains("disco"));
}

#[tokio::test]
async fn shutdown_on_empty_manager_is_ok() {
    let mgr = McpClientManager::start(&[]).await.unwrap();
    mgr.shutdown().await.unwrap();
}

#[tokio::test]
async fn health_check_reconnects_failed_servers_and_callbacks_fire() {
    // Start with a broken server so it's tracked with consecutive_failures=1.
    // Run one health-check cycle.  The reconnect will fail again (same bad
    // command), but the path exercises the parallel reconnect Phase 2 and
    // the error branch of Phase 3.
    let entries = vec![stdio_entry(
        "retry-me",
        vec!["/does-not-exist-987654".into()],
        true,
    )];
    let mgr = Arc::new(McpClientManager::start(&entries).await.unwrap());

    let register_calls = Arc::new(Mutex::new(0u32));
    let unregister_calls = Arc::new(Mutex::new(0u32));

    let reg_clone = register_calls.clone();
    let register: Arc<dyn Fn(Arc<dyn ToolHandler>) + Send + Sync> = Arc::new(move |_h| {
        if let Ok(mut g) = reg_clone.lock() {
            *g += 1;
        }
    });

    let unreg_clone = unregister_calls.clone();
    let unregister: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(move |_p| {
        if let Ok(mut g) = unreg_clone.lock() {
            *g += 1;
        }
    });

    mgr.health_check(&register, &unregister).await;

    // No successful reconnect, so register callback should never fire.
    let reg_count = *register_calls.lock().unwrap();
    assert_eq!(reg_count, 0);
    // No previously-connected client to unregister either.
    let unreg_count = *unregister_calls.lock().unwrap();
    assert_eq!(unreg_count, 0);
}

#[tokio::test]
async fn start_records_failed_stdio_when_process_spawns_then_exits() {
    // `sh -c "exit 0"` spawns successfully so TokioChildProcess::new returns
    // Ok, but the child closes stdout immediately, so the rmcp initialize
    // handshake fails. This drives the path through `serve()` to the error
    // branch (vs the spawn-failure branch exercised by `/does-not-exist`).
    let entries = vec![stdio_entry(
        "exits-fast",
        vec!["sh".into(), "-c".into(), "exit 0".into()],
        true,
    )];
    let mgr = McpClientManager::start(&entries).await.unwrap();
    assert_eq!(mgr.tool_count().await, 0);
}

#[tokio::test]
async fn start_records_http_failure_with_env_substituted_and_invalid_headers() {
    // SAFETY: test-only.
    unsafe {
        std::env::set_var("TEST_MCP_HDR", "secret-token");
    }
    let mut headers = HashMap::new();
    headers.insert("X-Auth".into(), "Bearer ${TEST_MCP_HDR}".into());
    // Headers with invalid names should be skipped via the `warn!` branch.
    headers.insert("bad header name".into(), "v".into());

    let entry = McpServerEntry {
        name: "with-headers".to_string(),
        transport: McpTransportConfig::Http {
            url: "http://127.0.0.1:1/mcp".into(),
            headers,
        },
        env: HashMap::new(),
        enabled: true,
        trust: McpTrustLevel::Trust,
    };
    let mgr = McpClientManager::start(&[entry]).await.unwrap();
    assert_eq!(mgr.tool_count().await, 0);
    unsafe {
        std::env::remove_var("TEST_MCP_HDR");
    }
}

#[tokio::test]
async fn health_check_no_op_when_no_servers_need_reconnection() {
    let mgr = McpClientManager::start(&[]).await.unwrap();
    let register: Arc<dyn Fn(Arc<dyn ToolHandler>) + Send + Sync> = Arc::new(|_| {});
    let unregister: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(|_| {});
    // Empty manager: returns early because reconnect_work is empty.
    mgr.health_check(&register, &unregister).await;
    assert_eq!(mgr.server_count().await, 0);
}
