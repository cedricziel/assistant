//! MCP client library — connects to external MCP servers and bridges their
//! tools into the assistant's tool registry.
//!
//! # Architecture
//!
//! ```text
//! McpClientManager
//!   ├─ McpClient("github")  ← session layer (initialize, tools/list, tools/call)
//!   │   └─ StdioTransport   ← raw JSON-RPC over subprocess stdin/stdout
//!   ├─ McpClient("db")
//!   │   └─ HttpSseTransport ← raw JSON-RPC over HTTP + SSE
//!   └─ tool handlers        ← Vec<McpToolHandler> registered as ambient tools
//! ```
//!
//! Each [`McpToolHandler`] implements `assistant_core::ToolHandler` and
//! forwards `run()` calls to the remote MCP server via the client session.

pub mod bridge;
pub mod client;
pub mod manager;
pub mod protocol;
pub mod transport;

pub use bridge::McpToolHandler;
pub use client::McpClient;
pub use manager::McpClientManager;
