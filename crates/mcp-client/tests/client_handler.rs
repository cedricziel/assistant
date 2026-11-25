//! The client handler's behaviour against an in-process rmcp server over
//! `tokio::io::duplex`: negotiated protocol, declared capabilities and
//! tool-list freshness.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use assistant_mcp_client::StaleTools;
use assistant_mcp_client::client::McpClient;
use assistant_mcp_client::handler::{APPS_EXTENSION_ID, APPS_MIME_TYPE};
use rmcp::model::{
    ClientCapabilities, DiscoverResult, ErrorCode, ErrorData, ListToolsResult,
    PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerConfig, SubscriptionFilter,
    Tool,
};
use rmcp::service::{MaybeSendFuture, Peer, RequestContext, SubscriptionContext};
use rmcp::{RoleServer, ServerHandler, ServiceExt};
use tokio::sync::Notify;

/// What the server saw of the client on a request.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    protocol_version: Option<ProtocolVersion>,
    capabilities: Option<ClientCapabilities>,
}

#[derive(Clone)]
struct Server {
    /// Only speaks the `initialize` era: `server/discover` is unknown.
    legacy_only: bool,
    tools: Arc<Mutex<Vec<&'static str>>>,
    seen: Arc<Mutex<Option<Seen>>>,
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    /// Fires a list-changed notification on the open subscription.
    fire: Arc<Notify>,
}

impl Server {
    fn new(legacy_only: bool) -> Self {
        Self {
            legacy_only,
            tools: Arc::new(Mutex::new(vec!["first"])),
            seen: Arc::default(),
            peer: Arc::default(),
            fire: Arc::default(),
        }
    }

    fn seen(&self) -> Seen {
        self.seen
            .lock()
            .unwrap()
            .clone()
            .expect("the server has seen a request")
    }

    fn set_tools(&self, tools: &[&'static str]) {
        *self.tools.lock().unwrap() = tools.to_vec();
    }

    /// Announce a change the way the negotiated protocol allows.
    async fn announce_tool_change(&self) {
        let peer = self.peer.lock().unwrap().clone();
        match peer {
            // `initialize` era: an unsolicited notification.
            Some(peer) if self.legacy_only => peer.notify_tool_list_changed().await.unwrap(),
            // `2026-07-28`: only on the client's subscription.
            _ => self.fire.notify_one(),
        }
    }
}

impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
    }

    fn discover(
        &self,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<DiscoverResult, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(if self.legacy_only {
            Err(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                "Method not found",
                None,
            ))
        } else {
            Ok(DiscoverResult::from_server_info(
                ProtocolVersion::KNOWN_VERSIONS.to_vec(),
                self.get_info(),
            ))
        })
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + MaybeSendFuture + '_ {
        *self.seen.lock().unwrap() = Some(Seen {
            protocol_version: context.protocol_version(),
            capabilities: context.client_capabilities(),
        });
        *self.peer.lock().unwrap() = Some(context.peer.clone());
        let tools = self
            .tools
            .lock()
            .unwrap()
            .iter()
            .map(|name| Tool::new(*name, "a tool", Arc::new(serde_json::Map::new())))
            .collect();
        std::future::ready(Ok(ListToolsResult {
            tools,
            ..Default::default()
        }))
    }

    fn accepted_subscription_filter(
        &self,
        requested: &SubscriptionFilter,
    ) -> Option<SubscriptionFilter> {
        Some(requested.supported_by(&self.get_info().capabilities))
    }

    async fn listen(&self, context: SubscriptionContext) -> Result<(), ErrorData> {
        self.fire.notified().await;
        context.sink().notify_tool_list_changed().await.ok();
        context.cancelled().await;
        Ok(())
    }
}

async fn connect(server: &Server) -> McpClient {
    let (server_io, client_io) = tokio::io::duplex(8192);
    let server = server.clone();
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    McpClient::connect("probe", client_io, StaleTools::default())
        .await
        .expect("the client connects")
}

/// Poll until the client reports a stale tool list, or give up.
async fn becomes_stale(client: &McpClient) -> bool {
    for _ in 0..100 {
        if client.take_tools_stale() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    false
}

fn extension_ids(capabilities: &ClientCapabilities) -> Vec<String> {
    capabilities
        .extensions
        .as_ref()
        .map(|e| e.keys().cloned().collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn a_modern_server_is_spoken_to_in_2026_07_28() {
    let server = Server::new(false);
    let client = connect(&server).await;
    client.list_tools().await.unwrap();

    assert_eq!(
        server.seen().protocol_version,
        Some(ProtocolVersion::V_2026_07_28)
    );
}

#[tokio::test]
async fn declares_tasks_skills_and_apps_and_nothing_interactive() {
    let server = Server::new(false);
    let client = connect(&server).await;
    client.list_tools().await.unwrap();

    let capabilities = server.seen().capabilities.expect("capabilities sent");
    assert_eq!(
        (
            extension_ids(&capabilities),
            capabilities.extensions.as_ref().and_then(|e| e
                .get(APPS_EXTENSION_ID)
                .and_then(|ui| ui.get("mimeTypes"))
                .cloned()),
            capabilities.sampling.is_some(),
            capabilities.roots.is_some(),
            capabilities.elicitation.is_some(),
        ),
        (
            vec![
                "io.modelcontextprotocol/skills".to_string(),
                "io.modelcontextprotocol/tasks".to_string(),
                APPS_EXTENSION_ID.to_string(),
            ],
            Some(serde_json::json!([APPS_MIME_TYPE])),
            false,
            false,
            false,
        ),
        "the three extensions are declared, sampling, roots and elicitation are not"
    );
}

#[tokio::test]
async fn a_2025_11_25_server_connects_and_lists_its_tools() {
    let server = Server::new(true);
    let client = connect(&server).await;
    let tools = client.list_tools().await.unwrap();

    assert_eq!(
        (
            tools.iter().map(|t| t.name.to_string()).collect::<Vec<_>>(),
            server.seen().protocol_version,
            extension_ids(&server.seen().capabilities.unwrap()).len(),
        ),
        (
            vec!["first".to_string()],
            Some(ProtocolVersion::V_2025_11_25),
            3
        ),
        "the fallback uses initialize and still declares the extensions"
    );
}

#[tokio::test]
async fn a_tool_list_change_on_a_modern_connection_marks_the_tools_stale() {
    let server = Server::new(false);
    let client = connect(&server).await;
    client.list_tools().await.unwrap();
    assert!(!client.take_tools_stale(), "fresh after the first listing");

    server.set_tools(&["first", "second"]);
    server.announce_tool_change().await;

    assert!(
        becomes_stale(&client).await,
        "the subscription's notification should mark the tools stale"
    );
    assert!(
        !client.take_tools_stale(),
        "taking the flag clears it until the next change"
    );
    let names: Vec<String> = client
        .list_tools()
        .await
        .unwrap()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    assert_eq!(
        names,
        ["first", "second"],
        "the next listing sees the change"
    );
}

#[tokio::test]
async fn a_tool_list_change_on_a_legacy_connection_marks_the_tools_stale() {
    let server = Server::new(true);
    let client = connect(&server).await;
    client.list_tools().await.unwrap();
    assert!(!client.take_tools_stale(), "fresh after the first listing");

    server.set_tools(&["first", "second"]);
    server.announce_tool_change().await;

    assert!(
        becomes_stale(&client).await,
        "the unsolicited notification should mark the tools stale"
    );
}
