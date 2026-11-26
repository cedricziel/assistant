//! MCP Apps resolution: after a call to a tool that declares a `ui://`
//! resource, `McpToolHandler` reads that resource from the same server and
//! attaches an `McpApp` to the output's data.

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::ToolHandler;
use assistant_core::mcp_app::{McpApp, McpAppCsp, McpAppPermissions};
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_mcp_client::StaleTools;
use assistant_mcp_client::bridge::McpToolHandler;
use assistant_mcp_client::client::McpClient;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListResourcesResult,
    ListToolsResult, MetaObject, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::json;
use uuid::Uuid;

const APP_MIME: &str = "text/html;profile=mcp-app";

fn meta(value: serde_json::Value) -> MetaObject {
    match value {
        serde_json::Value::Object(map) => MetaObject(map),
        _ => MetaObject::default(),
    }
}

fn tool(name: &str, tool_meta: serde_json::Value) -> Tool {
    Tool::new(name.to_string(), "d", Arc::new(serde_json::Map::new())).with_meta(meta(tool_meta))
}

fn ui_meta() -> serde_json::Value {
    json!({"ui": {
        "csp": {"connectDomains": ["https://api.weather.test"], "resourceDomains": ["https://cdn.test"]},
        "permissions": {"geolocation": {}, "clipboardWrite": {}},
        "prefersBorder": true,
    }})
}

#[derive(Clone)]
struct AppServer;

impl AppServer {
    fn tools() -> Vec<Tool> {
        vec![
            tool(
                "forecast",
                json!({"ui": {"resourceUri": "ui://weather/forecast"}}),
            ),
            tool("legacy", json!({"ui/resourceUri": "ui://weather/legacy"})),
            tool(
                "plain",
                json!({"ui": {"resourceUri": "ui://weather/plain"}}),
            ),
            tool("blob", json!({"ui": {"resourceUri": "ui://weather/blob"}})),
            tool(
                "listed",
                json!({"ui": {"resourceUri": "ui://weather/listed"}}),
            ),
            tool(
                "missing",
                json!({"ui": {"resourceUri": "ui://weather/missing"}}),
            ),
            tool(
                "web",
                json!({"ui": {"resourceUri": "https://weather.test/app"}}),
            ),
            Tool::new("bare", "d", Arc::new(serde_json::Map::new())),
        ]
    }

    fn contents(uri: &str) -> Option<ResourceContents> {
        let html = "<p>sunny</p>";
        let with_meta = |contents: ResourceContents, value| match contents {
            ResourceContents::TextResourceContents {
                uri,
                mime_type,
                text,
                ..
            } => ResourceContents::TextResourceContents {
                uri,
                mime_type,
                text,
                meta: Some(meta(value)),
            },
            other => other,
        };
        Some(match uri {
            "ui://weather/forecast" | "ui://weather/legacy" => with_meta(
                ResourceContents::text(html, uri).with_mime_type(APP_MIME),
                ui_meta(),
            ),
            "ui://weather/plain" => ResourceContents::text(html, uri).with_mime_type("text/html"),
            "ui://weather/blob" => {
                ResourceContents::blob("PHA+c3Vubnk8L3A+", uri).with_mime_type(APP_MIME)
            }
            "ui://weather/listed" => ResourceContents::text(html, uri).with_mime_type(APP_MIME),
            _ => return None,
        })
    }
}

impl ServerHandler for AppServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(Self::tools())))
    }

    fn call_tool(
        &self,
        _request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(Ok(CallToolResponse::from(CallToolResult::success(vec![
            ContentBlock::text("21 degrees"),
        ]))))
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, ErrorData>> + MaybeSendFuture + '_ {
        let listed = Resource::new("ui://weather/listed", "listed")
            .with_mime_type(APP_MIME)
            .with_meta(meta(
                json!({"ui": {"csp": {"frameDomains": ["https://maps.test"]}}}),
            ));
        std::future::ready(Ok(ListResourcesResult::with_all_items(vec![listed])))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResponse, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(match Self::contents(&request.uri) {
            Some(contents) => Ok(ReadResourceResponse::Complete(ReadResourceResult::new(
                vec![contents],
            ))),
            None => Err(ErrorData::resource_not_found("no such resource", None)),
        })
    }
}

async fn handler(name: &str) -> McpToolHandler {
    let (server_io, client_io) = tokio::io::duplex(65536);
    tokio::spawn(async move {
        if let Ok(running) = AppServer.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    let client = Arc::new(
        McpClient::connect("weather", client_io, StaleTools::default())
            .await
            .expect("the client connects"),
    );
    let tool = client
        .list_tools()
        .await
        .unwrap()
        .into_iter()
        .find(|t| t.name == name)
        .unwrap();
    McpToolHandler::from_remote("weather", &tool, client, false)
}

fn ctx() -> ExecutionContext {
    ExecutionContext {
        conversation_id: Uuid::new_v4(),
        agent_id: "tester".into(),
        turn: 0,
        interface: Interface::Web,
        interactive: true,
        run_id: None,
        allowed_tools: None,
        depth: 0,
        user_id: None,
        org_id: None,
        space_id: None,
    }
}

async fn app_after_calling(name: &str) -> Option<McpApp> {
    let mut output = handler(name)
        .await
        .run(HashMap::new(), &ctx())
        .await
        .unwrap();
    assert!(
        output.success,
        "the call itself succeeds: {}",
        output.content
    );
    McpApp::take_from(&mut output.data)
}

fn expected(resource_uri: &str) -> McpApp {
    McpApp {
        server: "weather".into(),
        resource_uri: resource_uri.into(),
        html: "<p>sunny</p>".into(),
        csp: McpAppCsp {
            connect_domains: vec!["https://api.weather.test".into()],
            resource_domains: vec!["https://cdn.test".into()],
            ..McpAppCsp::default()
        },
        permissions: McpAppPermissions {
            geolocation: true,
            clipboard_write: true,
            ..McpAppPermissions::default()
        },
        prefers_border: Some(true),
    }
}

#[tokio::test]
async fn tool_with_a_ui_resource_attaches_the_app() {
    assert_eq!(
        app_after_calling("forecast").await,
        Some(expected("ui://weather/forecast")),
        "the app carries the HTML, CSP and permissions read from the resource"
    );
}

#[tokio::test]
async fn deprecated_flat_resource_uri_key_is_honoured() {
    assert_eq!(
        app_after_calling("legacy").await,
        Some(expected("ui://weather/legacy"))
    );
}

#[tokio::test]
async fn resource_without_the_app_profile_attaches_nothing() {
    assert_eq!(app_after_calling("plain").await, None);
}

#[tokio::test]
async fn base64_blob_resource_is_decoded() {
    let app = app_after_calling("blob").await.expect("an app");
    assert_eq!(app.html, "<p>sunny</p>");
}

#[tokio::test]
async fn list_time_metadata_applies_when_the_read_has_none() {
    let app = app_after_calling("listed").await.expect("an app");
    assert_eq!(
        app.csp.frame_domains,
        vec!["https://maps.test".to_string()],
        "the resource's listed _meta.ui fills in for the read"
    );
}

#[tokio::test]
async fn unreadable_resource_attaches_nothing() {
    assert_eq!(app_after_calling("missing").await, None);
}

#[tokio::test]
async fn non_ui_scheme_attaches_nothing() {
    assert_eq!(app_after_calling("web").await, None);
}

#[tokio::test]
async fn tool_without_a_ui_resource_attaches_nothing() {
    assert_eq!(app_after_calling("bare").await, None);
}
