//! MCP client session — handles the MCP protocol handshake and provides
//! typed methods for `tools/list`, `tools/call`, etc.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use crate::protocol::{
    method, ClientCapabilities, ClientInfo, InitializeParams, InitializeResult, RemoteTool,
    ToolCallParams, ToolCallResult, ToolListResult, PROTOCOL_VERSION,
};
use crate::transport::McpTransport;

/// An active session with a single MCP server.
///
/// Manages the connection lifecycle (initialize → ready) and provides typed
/// methods that map to MCP protocol calls.
pub struct McpClient {
    /// User-assigned name for this server connection.
    name: String,
    /// The underlying transport (stdio, HTTP/SSE, etc.).
    transport: Arc<dyn McpTransport>,
    /// Server capabilities received during initialization.
    server_caps: Option<InitializeResult>,
    /// Monotonically increasing request ID.
    next_id: AtomicU64,
}

impl McpClient {
    /// Create a new client wrapping an already-connected transport.
    ///
    /// Call [`initialize`] before using any other method.
    pub fn new(name: impl Into<String>, transport: Arc<dyn McpTransport>) -> Self {
        Self {
            name: name.into(),
            transport,
            server_caps: None,
            next_id: AtomicU64::new(1),
        }
    }

    /// Allocate the next request ID.
    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Perform the MCP `initialize` handshake.
    ///
    /// Sends `initialize` with our client capabilities, waits for the server
    /// response, then sends the `notifications/initialized` notification.
    pub async fn initialize(&mut self) -> Result<&InitializeResult> {
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities { roots: None },
            client_info: ClientInfo {
                name: "assistant".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let id = self.next_id();
        let req = crate::protocol::JsonRpcRequest::call(
            id,
            method::INITIALIZE,
            Some(serde_json::to_value(&params)?),
        );

        let resp = self
            .transport
            .request(req)
            .await
            .context("MCP initialize request failed")?;

        let result_value = resp
            .into_result()
            .context("MCP initialize returned error")?;
        let init_result: InitializeResult =
            serde_json::from_value(result_value).context("failed to parse InitializeResult")?;

        info!(
            server = %self.name,
            protocol = %init_result.protocol_version,
            server_name = init_result.server_info.as_ref().map(|s| s.name.as_str()).unwrap_or("unknown"),
            "MCP server initialized"
        );

        // Send `notifications/initialized` to signal we're ready.
        let notif = crate::protocol::JsonRpcRequest::notification(method::INITIALIZED, None);
        self.transport.notify(notif).await?;

        self.server_caps = Some(init_result);
        Ok(self.server_caps.as_ref().unwrap())
    }

    /// Fetch the list of tools from the remote server.
    pub async fn list_tools(&self) -> Result<Vec<RemoteTool>> {
        let id = self.next_id();
        let req = crate::protocol::JsonRpcRequest::call(id, method::TOOLS_LIST, None);

        let resp = self
            .transport
            .request(req)
            .await
            .context("tools/list request failed")?;

        let value = resp.into_result().context("tools/list returned error")?;
        let result: ToolListResult =
            serde_json::from_value(value).context("failed to parse ToolListResult")?;

        debug!(
            server = %self.name,
            count = result.tools.len(),
            "discovered remote tools"
        );

        Ok(result.tools)
    }

    /// Call a tool on the remote server.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult> {
        let params = ToolCallParams {
            name: name.to_string(),
            arguments,
        };

        let id = self.next_id();
        let req = crate::protocol::JsonRpcRequest::call(
            id,
            method::TOOLS_CALL,
            Some(serde_json::to_value(&params)?),
        );

        let resp = self
            .transport
            .request(req)
            .await
            .with_context(|| format!("tools/call '{name}' request failed"))?;

        let value = resp
            .into_result()
            .with_context(|| format!("tools/call '{name}' returned error"))?;
        let result: ToolCallResult =
            serde_json::from_value(value).context("failed to parse ToolCallResult")?;

        Ok(result)
    }

    /// Send a `ping` and wait for the response.
    pub async fn ping(&self) -> Result<()> {
        let id = self.next_id();
        let req = crate::protocol::JsonRpcRequest::call(id, method::PING, None);
        let resp = self.transport.request(req).await?;
        let _ = resp.into_result()?;
        Ok(())
    }

    /// The user-assigned name for this server.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the underlying transport is still connected.
    pub fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }

    /// Subscribe to server-initiated notifications.
    pub fn notifications(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::protocol::JsonRpcMessage> {
        self.transport.notifications()
    }

    /// Server capabilities (available after [`initialize`]).
    pub fn server_capabilities(&self) -> Option<&InitializeResult> {
        self.server_caps.as_ref()
    }

    /// Whether the server advertised tool-list-changed notifications.
    pub fn supports_tool_list_changed(&self) -> bool {
        self.server_caps
            .as_ref()
            .and_then(|c| c.capabilities.tools.as_ref())
            .is_some_and(|t| t.list_changed)
    }

    /// Gracefully shut down the transport.
    pub async fn shutdown(&self) -> Result<()> {
        warn!(server = %self.name, "shutting down MCP client");
        self.transport.shutdown().await
    }
}
