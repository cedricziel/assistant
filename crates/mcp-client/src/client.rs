//! MCP client session — thin wrapper around the `rmcp` SDK.
//!
//! Stores the `Peer<RoleClient>` for sending requests and the
//! `RunningService` for lifecycle management.

use std::sync::Arc;

use anyhow::{Context, Result};
use rmcp::model::{CallToolRequestParams, CallToolResult, ServerPeerInfo, Tool};
use rmcp::service::{Peer, RoleClient, RunningService};
use tokio::sync::Mutex;
use tracing::debug;

/// An active session with a single MCP server.
///
/// The `Peer` is cheaply clonable (Arc internally) and used for all
/// request/response interactions.  The `RunningService` owns the
/// background I/O task and is kept alive for the session duration.
pub struct McpClient {
    /// User-assigned name for this server connection.
    name: String,
    /// The rmcp peer used for sending requests.
    peer: Peer<RoleClient>,
    /// The running service that owns the background I/O task.
    /// Wrapped in `Mutex<Option<...>>` so we can `close()` it on shutdown.
    _service: Mutex<Option<RunningService<RoleClient, ()>>>,
}

impl McpClient {
    /// Create a new client wrapping an already-initialized rmcp service.
    pub fn new(name: impl Into<String>, service: RunningService<RoleClient, ()>) -> Self {
        let peer = service.peer().clone();
        Self {
            name: name.into(),
            peer,
            _service: Mutex::new(Some(service)),
        }
    }

    /// Fetch the list of tools from the remote server (handles pagination).
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let tools = self
            .peer
            .list_all_tools()
            .await
            .context("tools/list request failed")?;

        debug!(
            server = %self.name,
            count = tools.len(),
            "discovered remote tools"
        );

        Ok(tools)
    }

    /// Call a tool on the remote server.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult> {
        let mut params = CallToolRequestParams::new(name.to_string());
        if let Some(args) = arguments {
            params = params.with_arguments(args);
        }

        let result = self
            .peer
            .call_tool(params)
            .await
            .with_context(|| format!("tools/call '{name}' request failed"))?;

        Ok(result)
    }

    /// The user-assigned name for this server.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the underlying transport is still connected.
    pub fn is_connected(&self) -> bool {
        !self.peer.is_transport_closed()
    }

    /// Snapshot of the server's handshake info (available after initialization).
    ///
    /// rmcp hands out an owned `Arc` snapshot rather than a borrow, so the
    /// caller keeps the info alive independently of this session.
    pub fn server_info(&self) -> Option<Arc<ServerPeerInfo>> {
        self.peer.peer_info()
    }

    /// Whether the server advertised tool-list-changed notifications.
    pub fn supports_tool_list_changed(&self) -> bool {
        self.peer.peer_info().is_some_and(|info| {
            info.capabilities
                .tools
                .as_ref()
                .is_some_and(|t| t.list_changed.unwrap_or(false))
        })
    }

    /// Gracefully shut down the MCP session.
    pub async fn shutdown(&self) -> Result<()> {
        debug!(server = %self.name, "shutting down MCP client");
        let mut service = self._service.lock().await;
        if let Some(mut svc) = service.take() {
            // RunningService::close() waits for cleanup.  Ignore its result —
            // it only fails if the transport is already gone, which is fine
            // during shutdown.
            let _ = svc.close().await;
            debug!(server = %self.name, "MCP client session closed");
        }
        Ok(())
    }
}
