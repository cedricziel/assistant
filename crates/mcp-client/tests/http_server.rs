//! Manager behaviour against a scripted MCP server speaking Streamable HTTP.
//!
//! A wiremock responder answers the JSON-RPC methods the client sends
//! (`initialize`, `tools/list`, `tools/call`), so the tests drive real
//! connection, discovery, refresh, reconnect and shutdown paths without a
//! subprocess.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use assistant_core::ToolHandler;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_core::types::skills_mcp::{McpServerEntry, McpTransportConfig, McpTrustLevel};
use assistant_mcp_client::manager::McpClientManager;
use serde_json::{Value, json};
use uuid::Uuid;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// Scripted server state shared between the test and the responder.
#[derive(Clone)]
struct Script {
    tools: Arc<Mutex<Vec<&'static str>>>,
    healthy: Arc<AtomicBool>,
    list_fails: Arc<AtomicBool>,
}

impl Script {
    fn new(tools: &[&'static str]) -> Self {
        Self {
            tools: Arc::new(Mutex::new(tools.to_vec())),
            healthy: Arc::new(AtomicBool::new(true)),
            list_fails: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn rpc_result(id: &Value, result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("mcp-session-id", "session-1")
        .set_body_json(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

fn rpc_error(id: &Value, message: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(
        json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": message}}),
    )
}

impl Respond for Script {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        if !self.healthy.load(Ordering::SeqCst) {
            return ResponseTemplate::new(500);
        }
        let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
        let id = body["id"].clone();
        match body["method"].as_str().unwrap_or_default() {
            "initialize" => rpc_result(
                &id,
                json!({
                    "protocolVersion": body["params"]["protocolVersion"],
                    "capabilities": {"tools": {"listChanged": true}},
                    "serverInfo": {"name": "scripted", "version": "1.0.0"},
                }),
            ),
            "tools/list" if self.list_fails.load(Ordering::SeqCst) => {
                rpc_error(&id, "tool listing unavailable")
            }
            "tools/list" => {
                let tools: Vec<Value> = self
                    .tools
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|name| {
                        json!({
                            "name": name,
                            "description": format!("the {name} tool"),
                            "inputSchema": {"type": "object"},
                        })
                    })
                    .collect();
                rpc_result(&id, json!({"tools": tools}))
            }
            "tools/call" => match body["params"]["name"].as_str() {
                Some("fail") => rpc_error(&id, "tool exploded"),
                Some(name) => rpc_result(
                    &id,
                    json!({"content": [{"type": "text", "text": format!("ran {name}")}]}),
                ),
                None => rpc_error(&id, "missing tool name"),
            },
            _ => ResponseTemplate::new(202),
        }
    }
}

async fn serve(script: &Script) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(script.clone())
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(405))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    server
}

fn entry(name: &str, url: &str, trust: McpTrustLevel) -> McpServerEntry {
    McpServerEntry {
        name: name.to_string(),
        transport: McpTransportConfig::Http {
            url: url.to_string(),
            headers: HashMap::new(),
        },
        env: HashMap::new(),
        enabled: true,
        trust,
    }
}

fn ctx() -> ExecutionContext {
    ExecutionContext {
        conversation_id: Uuid::new_v4(),
        agent_id: "tester".into(),
        turn: 0,
        interface: Interface::Mcp,
        interactive: false,
        allowed_tools: None,
        depth: 0,
        user_id: None,
        org_id: None,
        space_id: None,
    }
}

async fn handler_names(mgr: &McpClientManager) -> Vec<String> {
    let mut names: Vec<String> = mgr
        .tool_handlers()
        .await
        .iter()
        .map(|h| h.name().to_string())
        .collect();
    names.sort();
    names
}

async fn request_methods(server: &MockServer, http_method: &str) -> usize {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| r.method.as_str() == http_method)
        .count()
}

#[tokio::test]
async fn start_bridges_remote_tools_under_namespaced_names() {
    let server = serve(&Script::new(&["alpha", "beta"])).await;

    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Confirm)])
        .await
        .unwrap();

    assert_eq!(
        (handler_names(&mgr).await, mgr.server_count().await),
        (
            vec![
                "mcp__remote__alpha".to_string(),
                "mcp__remote__beta".to_string()
            ],
            1
        )
    );
}

#[tokio::test]
async fn trust_level_decides_whether_tools_require_confirmation() {
    let server = serve(&Script::new(&["alpha"])).await;
    let trusted = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Trust)])
        .await
        .unwrap();
    let confirmed =
        McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Confirm)])
            .await
            .unwrap();

    let trusted_flag = trusted.tool_handlers().await[0].requires_confirmation();
    let confirmed_flag = confirmed.tool_handlers().await[0].requires_confirmation();

    assert_eq!((trusted_flag, confirmed_flag), (false, true));
}

#[tokio::test]
async fn bridged_tool_call_reaches_the_server_and_returns_its_text() {
    let server = serve(&Script::new(&["alpha"])).await;
    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Trust)])
        .await
        .unwrap();
    let handler = mgr.tool_handlers().await.remove(0);

    let out = handler.run(HashMap::new(), &ctx()).await.unwrap();

    assert_eq!((out.success, out.content.as_str()), (true, "ran alpha"));
}

#[tokio::test]
async fn server_side_call_error_becomes_error_output_naming_the_tool() {
    let server = serve(&Script::new(&["fail"])).await;
    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Trust)])
        .await
        .unwrap();
    let handler = mgr.tool_handlers().await.remove(0);

    let out = handler.run(HashMap::new(), &ctx()).await.unwrap();

    assert!(!out.success, "an RPC error must not be reported as success");
    assert!(
        out.content.contains("'fail'") && out.content.contains("tool exploded"),
        "content was: {}",
        out.content
    );
}

#[tokio::test]
async fn tool_listing_error_leaves_server_disconnected_with_no_tools() {
    let script = Script::new(&["alpha"]);
    script.list_fails.store(true, Ordering::SeqCst);
    let server = serve(&script).await;

    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Trust)])
        .await
        .unwrap();

    assert_eq!((mgr.tool_count().await, mgr.server_count().await), (0, 0));
}

#[tokio::test]
async fn http_server_error_on_initialize_leaves_server_disconnected() {
    let script = Script::new(&["alpha"]);
    script.healthy.store(false, Ordering::SeqCst);
    let server = serve(&script).await;

    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Trust)])
        .await
        .unwrap();

    assert_eq!((mgr.tool_count().await, mgr.server_count().await), (0, 0));
}

#[tokio::test]
async fn configured_headers_are_sent_with_env_references_expanded() {
    // SAFETY: variable name is unique to this test.
    unsafe { std::env::set_var("MCP_CLIENT_TEST_TOKEN", "s3cret") };
    let server = serve(&Script::new(&["alpha"])).await;
    let mut e = entry("remote", &server.uri(), McpTrustLevel::Trust);
    e.transport = McpTransportConfig::Http {
        url: server.uri(),
        headers: HashMap::from([
            (
                "x-api-key".to_string(),
                "Bearer ${MCP_CLIENT_TEST_TOKEN}".to_string(),
            ),
            ("bad header name".to_string(), "ignored".to_string()),
        ]),
    };

    let mgr = McpClientManager::start(&[e]).await.unwrap();

    let received = server.received_requests().await.unwrap_or_default();
    let sent: Vec<Option<String>> = received
        .iter()
        .filter(|r| r.method.as_str() == "POST")
        .map(|r| {
            r.headers
                .get("x-api-key")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        })
        .collect();
    unsafe { std::env::remove_var("MCP_CLIENT_TEST_TOKEN") };
    assert_eq!(
        (mgr.server_count().await, sent.first().cloned().flatten()),
        (1, Some("Bearer s3cret".to_string())),
        "an invalid header must be skipped without blocking the connection"
    );
}

#[tokio::test]
async fn env_is_not_promoted_to_http_headers() {
    let server = serve(&Script::new(&["alpha"])).await;
    let mut e = entry("remote", &server.uri(), McpTrustLevel::Trust);
    e.env = HashMap::from([("SECRET_ENV".to_string(), "do-not-leak".to_string())]);

    let _mgr = McpClientManager::start(&[e]).await.unwrap();

    let leaked = server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .any(|r| {
            r.headers
                .iter()
                .any(|(_, v)| v.to_str().ok() == Some("do-not-leak"))
        });
    assert!(!leaked, "env values must never be sent as HTTP headers");
}

#[tokio::test]
async fn refresh_replaces_only_the_refreshed_servers_tools() {
    let one = Script::new(&["a1", "a2"]);
    let two = Script::new(&["b1"]);
    let server_one = serve(&one).await;
    let server_two = serve(&two).await;
    let mgr = McpClientManager::start(&[
        entry("one", &server_one.uri(), McpTrustLevel::Trust),
        entry("two", &server_two.uri(), McpTrustLevel::Trust),
    ])
    .await
    .unwrap();
    *one.tools.lock().unwrap() = vec!["a2", "a3"];

    let refreshed = mgr.refresh_server_tools("one").await.unwrap();

    let mut refreshed_names: Vec<String> = refreshed.iter().map(|h| h.name().to_string()).collect();
    refreshed_names.sort();
    assert_eq!(
        (refreshed_names, handler_names(&mgr).await),
        (
            vec!["mcp__one__a2".to_string(), "mcp__one__a3".to_string()],
            vec![
                "mcp__one__a2".to_string(),
                "mcp__one__a3".to_string(),
                "mcp__two__b1".to_string()
            ]
        )
    );
}

#[tokio::test]
async fn refresh_keeps_trust_derived_confirmation_requirement() {
    let script = Script::new(&["alpha"]);
    let server = serve(&script).await;
    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Confirm)])
        .await
        .unwrap();

    let refreshed = mgr.refresh_server_tools("remote").await.unwrap();

    assert!(refreshed[0].requires_confirmation());
}

#[tokio::test]
async fn refresh_failure_keeps_existing_tools() {
    let script = Script::new(&["alpha"]);
    let server = serve(&script).await;
    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Trust)])
        .await
        .unwrap();
    script.list_fails.store(true, Ordering::SeqCst);

    let result = mgr.refresh_server_tools("remote").await;

    assert_eq!(
        (result.is_err(), handler_names(&mgr).await),
        (true, vec!["mcp__remote__alpha".to_string()])
    );
}

#[tokio::test]
async fn shutdown_terminates_the_http_session() {
    let server = serve(&Script::new(&["alpha"])).await;
    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Trust)])
        .await
        .unwrap();

    mgr.shutdown().await.unwrap();

    assert_eq!(request_methods(&server, "DELETE").await, 1);
}

#[tokio::test(start_paused = true)]
async fn health_check_reconnects_a_recovered_server_and_registers_its_tools() {
    let script = Script::new(&["alpha", "beta"]);
    script.healthy.store(false, Ordering::SeqCst);
    let server = serve(&script).await;
    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Trust)])
        .await
        .unwrap();
    script.healthy.store(true, Ordering::SeqCst);
    let registered = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = registered.clone();
    let register: Arc<dyn Fn(Arc<dyn ToolHandler>) + Send + Sync> = Arc::new(move |h| {
        sink.lock().unwrap().push(h.name().to_string());
    });
    let unregister: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(|_| {});

    mgr.health_check(&register, &unregister).await;

    let mut names = registered.lock().unwrap().clone();
    names.sort();
    assert_eq!(
        (
            names,
            mgr.server_count().await,
            handler_names(&mgr).await.len()
        ),
        (
            vec![
                "mcp__remote__alpha".to_string(),
                "mcp__remote__beta".to_string()
            ],
            1,
            2
        )
    );
}

#[tokio::test(start_paused = true)]
async fn health_check_pauses_reconnecting_after_five_consecutive_failures() {
    let script = Script::new(&["alpha"]);
    script.healthy.store(false, Ordering::SeqCst);
    let server = serve(&script).await;
    let mgr = McpClientManager::start(&[entry("remote", &server.uri(), McpTrustLevel::Trust)])
        .await
        .unwrap();
    let register: Arc<dyn Fn(Arc<dyn ToolHandler>) + Send + Sync> = Arc::new(|_| {});
    let unregister: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(|_| {});
    // `start` counted failure 1; four more cycles reach the limit of five.
    for _ in 0..4 {
        mgr.health_check(&register, &unregister).await;
    }
    let attempts_before = request_methods(&server, "POST").await;

    mgr.health_check(&register, &unregister).await;
    let attempts_during_pause = request_methods(&server, "POST").await;
    mgr.health_check(&register, &unregister).await;
    let attempts_after_pause = request_methods(&server, "POST").await;

    assert_eq!(
        (attempts_during_pause, attempts_after_pause),
        (attempts_before, attempts_before + 1),
        "the cycle at the failure limit must skip the server, the next must retry"
    );
}
