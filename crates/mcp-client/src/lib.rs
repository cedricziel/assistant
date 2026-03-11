//! MCP client library — connects to external MCP servers and bridges their
//! tools into the assistant's tool registry.
//!
//! # Architecture
//!
//! ```text
//! McpClientManager
//!   ├─ McpClient("github")  ← thin wrapper around rmcp RunningService/Peer
//!   │   └─ rmcp stdio transport (TokioChildProcess)
//!   ├─ McpClient("db")
//!   │   └─ rmcp Streamable HTTP transport
//!   └─ tool handlers        ← Vec<McpToolHandler> registered as ambient tools
//! ```
//!
//! Each [`McpToolHandler`] implements `assistant_core::ToolHandler` and
//! forwards `run()` calls to the remote MCP server via the rmcp `Peer`.

pub mod bridge;
pub mod client;
pub mod manager;

pub use bridge::McpToolHandler;
pub use client::McpClient;
pub use manager::McpClientManager;
