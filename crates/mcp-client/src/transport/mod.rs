//! Transport layer for MCP client connections.
//!
//! Each transport handles the physical communication with an MCP server
//! (stdio subprocess, HTTP/SSE, Streamable HTTP) while the [`McpClient`]
//! session layer handles the MCP protocol logic on top.

pub mod sse;
pub mod stdio;
pub mod streamable;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::protocol::{JsonRpcMessage, JsonRpcRequest};

/// A transport connects to a single MCP server and handles the raw JSON-RPC
/// message exchange.
///
/// Implementations manage the underlying I/O (subprocess pipes, HTTP
/// connections, etc.) and provide a request/response interface plus a
/// notification stream.
#[async_trait]
pub trait McpTransport: Send + Sync + 'static {
    /// Send a JSON-RPC request and wait for the matching response.
    ///
    /// The transport matches responses to requests by `id`.
    async fn request(&self, req: JsonRpcRequest) -> Result<JsonRpcMessage>;

    /// Send a JSON-RPC notification (no response expected).
    async fn notify(&self, req: JsonRpcRequest) -> Result<()>;

    /// Subscribe to server-initiated notifications.
    ///
    /// The returned receiver yields notifications like
    /// `notifications/tools/list_changed`.
    fn notifications(&self) -> broadcast::Receiver<JsonRpcMessage>;

    /// Check whether the transport is still connected.
    fn is_connected(&self) -> bool;

    /// Gracefully shut down the transport.
    async fn shutdown(&self) -> Result<()>;
}
