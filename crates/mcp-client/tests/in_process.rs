//! End-to-end tests for `McpClient` + `McpToolHandler` over an in-process
//! `tokio::io::duplex` transport.
//!
//! These tests construct a real rmcp server with a small set of canned
//! tools, pair it with the client via `tokio::io::duplex`, and exercise the
//! full handshake/list_tools/call_tool flow. This is how we reach the bulk
//! of `client.rs` and `bridge.rs` without spawning a subprocess or speaking
//! HTTP.

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::ToolHandler;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_mcp_client::bridge::{McpToolHandler, namespaced_name};
use assistant_mcp_client::client::McpClient;
use rmcp::handler::server::router::tool::{ToolRoute, ToolRouter};
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::{RoleServer, ServerHandler, ServiceExt};
use serde_json::json;
use tokio::sync::RwLock;
use uuid::Uuid;

// ── Test server ──────────────────────────────────────────────────────────────

#[derive(Clone)]
struct TestServer {
    router: Arc<RwLock<ToolRouter<Self>>>,
    list_changed: bool,
}

impl TestServer {
    fn new(list_changed: bool) -> Self {
        let mut router = ToolRouter::<Self>::new();
        let schema = Arc::new(serde_json::Map::new());

        router.add_route(ToolRoute::new_dyn(
            Tool::new("echo", "Echoes input as text", schema.clone()),
            |ctx: ToolCallContext<Self>| {
                Box::pin(async move {
                    let input = ctx
                        .arguments
                        .as_ref()
                        .and_then(|m| m.get("text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("default")
                        .to_string();
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "echo:{input}"
                    ))]))
                })
            },
        ));

        router.add_route(ToolRoute::new_dyn(
            Tool::new("boom", "Always returns an error", schema.clone()),
            |_ctx| {
                Box::pin(async { Ok(CallToolResult::error(vec![Content::text("intentional")])) })
            },
        ));

        router.add_route(ToolRoute::new_dyn(
            Tool::new("empty", "Returns no content", schema.clone()),
            |_ctx| Box::pin(async { Ok(CallToolResult::success(vec![])) }),
        ));

        router.add_route(ToolRoute::new_dyn(
            Tool::new(
                "with-image",
                "Returns a text part plus a 1-byte image",
                schema,
            ),
            |_ctx| {
                Box::pin(async {
                    // 1-byte payload encoded as base64 = "AA=="
                    Ok(CallToolResult::success(vec![
                        Content::text("textual"),
                        Content::image("AA==", "image/png"),
                    ]))
                })
            },
        ));

        Self {
            router: Arc::new(RwLock::new(router)),
            list_changed,
        }
    }
}

impl ServerHandler for TestServer {
    fn get_info(&self) -> ServerInfo {
        let mut builder = ServerCapabilities::builder().enable_tools();
        if self.list_changed {
            builder = builder.enable_tool_list_changed();
        }
        ServerInfo::new(builder.build())
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, rmcp::ErrorData>> + MaybeSendFuture + '_
    {
        async move {
            let router = self.router.read().await;
            Ok(ListToolsResult {
                tools: router.list_all(),
                ..Default::default()
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, rmcp::ErrorData>> + MaybeSendFuture + '_
    {
        async move {
            let router = self.router.read().await;
            let tcc = ToolCallContext::new(self, request, context);
            router.call(tcc).await
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Keep the server's RunningService alive for the lifetime of the spawned
/// task by awaiting `.waiting()` — otherwise the service is dropped as soon
/// as the handshake returns and the client sees `BrokenPipe`.
async fn connect(list_changed: bool) -> (McpClient, tokio::task::JoinHandle<()>) {
    let (server_io, client_io) = tokio::io::duplex(4096);
    let server = TestServer::new(list_changed);
    let server_handle = tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    let service = ().serve(client_io).await.expect("client handshake succeeds");
    (McpClient::new("test-srv", service), server_handle)
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

// ── McpClient surface tests ──────────────────────────────────────────────────

#[tokio::test]
async fn name_returns_configured_value() {
    let (client, _h) = connect(false).await;
    assert_eq!(client.name(), "test-srv");
}

#[tokio::test]
async fn is_connected_true_after_handshake() {
    let (client, _h) = connect(false).await;
    assert!(client.is_connected());
}

#[tokio::test]
async fn server_info_populated_after_initialize() {
    let (client, _h) = connect(false).await;
    let info = client.server_info().expect("server info present");
    assert!(info.capabilities.tools.is_some());
}

#[tokio::test]
async fn supports_tool_list_changed_reflects_server_capability() {
    let (off, _h1) = connect(false).await;
    let (on, _h2) = connect(true).await;
    assert!(!off.supports_tool_list_changed());
    assert!(on.supports_tool_list_changed());
}

#[tokio::test]
async fn list_tools_returns_all_server_tools() {
    let (client, _h) = connect(false).await;
    let tools = client.list_tools().await.unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains(&"echo"));
    assert!(names.contains(&"boom"));
    assert!(names.contains(&"empty"));
    assert!(names.contains(&"with-image"));
}

#[tokio::test]
async fn call_tool_succeeds_and_returns_text_content() {
    let (client, _h) = connect(false).await;
    let mut args = serde_json::Map::new();
    args.insert("text".into(), json!("hi"));
    let result = client.call_tool("echo", Some(args)).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
    let text = result
        .content
        .iter()
        .find_map(|c| match &c.raw {
            rmcp::model::RawContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(text, "echo:hi");
}

#[tokio::test]
async fn call_tool_no_arguments_dispatches_without_args() {
    let (client, _h) = connect(false).await;
    let result = client.call_tool("echo", None).await.unwrap();
    let text = result
        .content
        .iter()
        .find_map(|c| match &c.raw {
            rmcp::model::RawContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(text, "echo:default");
}

#[tokio::test]
async fn call_tool_unknown_returns_error() {
    let (client, _h) = connect(false).await;
    let err = client.call_tool("nonexistent", None).await;
    assert!(err.is_err(), "calling unknown tool must return Err");
}

#[tokio::test]
async fn shutdown_is_idempotent() {
    let (client, _h) = connect(false).await;
    client.shutdown().await.unwrap();
    // Second shutdown is a no-op because the service was taken.
    client.shutdown().await.unwrap();
}

// ── McpToolHandler bridge tests ──────────────────────────────────────────────

async fn fetch_tool(client: &McpClient, name: &str) -> Tool {
    let tools = client.list_tools().await.unwrap();
    tools.into_iter().find(|t| t.name.as_ref() == name).unwrap()
}

#[tokio::test]
async fn from_remote_namespaces_and_clones_schema() {
    let (raw, _h) = connect(false).await;
    let client = Arc::new(raw);
    let tool = fetch_tool(&client, "echo").await;
    let handler = McpToolHandler::from_remote("srv", &tool, client.clone(), true);
    assert_eq!(handler.name(), "mcp__srv__echo");
    assert!(handler.is_mutating());
    assert!(handler.requires_confirmation());
    assert_eq!(handler.server_name(), "srv");
    assert!(handler.params_schema().is_object());
}

#[tokio::test]
async fn from_remote_synthesises_description_when_remote_empty() {
    let (raw, _h) = connect(false).await;
    let client = Arc::new(raw);
    let mut tool = fetch_tool(&client, "echo").await;
    tool.description = Some(std::borrow::Cow::Borrowed(""));
    let handler = McpToolHandler::from_remote("srv", &tool, client.clone(), false);
    assert!(handler.description().contains("MCP server"));
    assert!(!handler.requires_confirmation());
}

#[tokio::test]
async fn run_forwards_text_response_and_marks_success() {
    let (raw, _h) = connect(false).await;
    let client = Arc::new(raw);
    let tool = fetch_tool(&client, "echo").await;
    let handler = McpToolHandler::from_remote("srv", &tool, client.clone(), false);
    let mut params = HashMap::new();
    params.insert("text".to_string(), json!("greetings"));
    let out = handler.run(params, &ctx()).await.unwrap();
    assert!(out.success);
    assert!(out.content.contains("echo:greetings"));
}

#[tokio::test]
async fn run_returns_error_output_when_server_marks_is_error() {
    let (raw, _h) = connect(false).await;
    let client = Arc::new(raw);
    let tool = fetch_tool(&client, "boom").await;
    let handler = McpToolHandler::from_remote("srv", &tool, client.clone(), false);
    let out = handler.run(HashMap::new(), &ctx()).await.unwrap();
    assert!(!out.success);
    assert!(out.content.contains("intentional"));
}

#[tokio::test]
async fn run_handles_empty_content_with_placeholder() {
    let (raw, _h) = connect(false).await;
    let client = Arc::new(raw);
    let tool = fetch_tool(&client, "empty").await;
    let handler = McpToolHandler::from_remote("srv", &tool, client.clone(), false);
    let out = handler.run(HashMap::new(), &ctx()).await.unwrap();
    assert!(out.success);
    assert!(out.content.contains("(no text output)"));
}

#[tokio::test]
async fn run_extracts_image_attachments() {
    let (raw, _h) = connect(false).await;
    let client = Arc::new(raw);
    let tool = fetch_tool(&client, "with-image").await;
    let handler = McpToolHandler::from_remote("srv", &tool, client.clone(), false);
    let out = handler.run(HashMap::new(), &ctx()).await.unwrap();
    assert!(out.success);
    assert!(out.content.contains("textual"));
    assert_eq!(out.attachments.len(), 1);
    assert_eq!(out.attachments[0].mime_type, "image/png");
}

#[tokio::test]
async fn run_reports_error_when_underlying_call_fails() {
    let (raw, _h) = connect(false).await;
    let client = Arc::new(raw);
    let mut tool = fetch_tool(&client, "echo").await;
    tool.name = std::borrow::Cow::Owned("does-not-exist".to_string());
    let handler = McpToolHandler::from_remote("srv", &tool, client.clone(), false);
    let out = handler.run(HashMap::new(), &ctx()).await.unwrap();
    assert!(!out.success);
    assert!(out.content.contains("does-not-exist"));
}

// ── Stand-alone helpers ──────────────────────────────────────────────────────

#[test]
fn namespaced_name_is_stable() {
    assert_eq!(namespaced_name("a", "b"), "mcp__a__b");
}
