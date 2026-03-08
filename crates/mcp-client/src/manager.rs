//! MCP client manager — coordinates all external MCP server connections.
//!
//! Starts configured servers, discovers their tools, and produces
//! `ToolHandler` instances ready for registration with `ToolExecutor`.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use assistant_core::{McpServerEntry, McpTransportConfig, McpTrustLevel, ToolHandler};

use crate::bridge::{self, McpToolHandler};
use crate::client::McpClient;
use crate::transport::sse::SseTransport;
use crate::transport::stdio::StdioTransport;
use crate::transport::streamable::StreamableHttpTransport;

/// Manages all MCP client connections and their bridged tools.
pub struct McpClientManager {
    /// Active client sessions keyed by server name.
    clients: RwLock<HashMap<String, Arc<McpClient>>>,
    /// All tool handlers produced from remote servers.
    handlers: RwLock<Vec<Arc<McpToolHandler>>>,
}

impl McpClientManager {
    /// Connect to all configured MCP servers and discover their tools.
    ///
    /// Servers that fail to connect are logged and skipped — a single broken
    /// server should not prevent the assistant from starting.
    pub async fn start(entries: &[McpServerEntry]) -> Result<Self> {
        let mut clients = HashMap::new();
        let mut handlers = Vec::new();

        for entry in entries {
            if !entry.enabled {
                info!(server = %entry.name, "MCP server disabled, skipping");
                continue;
            }

            match Self::connect_one(entry).await {
                Ok((client, tools)) => {
                    let client = Arc::new(client);
                    let requires_confirmation = entry.trust != McpTrustLevel::Trust;

                    for tool in &tools {
                        let handler = McpToolHandler::from_remote(
                            &entry.name,
                            tool,
                            client.clone(),
                            requires_confirmation,
                        );
                        handlers.push(Arc::new(handler));
                    }

                    info!(
                        server = %entry.name,
                        tools = tools.len(),
                        "MCP client connected"
                    );
                    clients.insert(entry.name.clone(), client);
                }
                Err(e) => {
                    error!(
                        server = %entry.name,
                        error = %e,
                        "failed to connect to MCP server, skipping"
                    );
                }
            }
        }

        Ok(Self {
            clients: RwLock::new(clients),
            handlers: RwLock::new(handlers),
        })
    }

    /// Connect to a single MCP server and perform the initialize handshake.
    async fn connect_one(
        entry: &McpServerEntry,
    ) -> Result<(McpClient, Vec<crate::protocol::RemoteTool>)> {
        let transport: Arc<dyn crate::transport::McpTransport> = match &entry.transport {
            McpTransportConfig::Stdio { command } => {
                let resolved_env = resolve_env_vars(&entry.env);
                let transport = StdioTransport::spawn(command, &resolved_env)
                    .await
                    .with_context(|| {
                        format!("failed to spawn MCP server '{}': {:?}", entry.name, command)
                    })?;
                Arc::new(transport)
            }
            McpTransportConfig::Http { url, headers } => {
                let resolved_headers = resolve_env_vars(headers);
                let resolved_env_headers = resolve_env_vars(&entry.env);
                let mut all_headers = resolved_env_headers;
                all_headers.extend(resolved_headers);

                // Try Streamable HTTP first (newer protocol), fall back to SSE.
                match StreamableHttpTransport::connect(url, &all_headers).await {
                    Ok(transport) => {
                        debug!(
                            server = %entry.name,
                            url = %url,
                            "connected via streamable HTTP"
                        );
                        Arc::new(transport)
                    }
                    Err(streamable_err) => {
                        debug!(
                            server = %entry.name,
                            error = %streamable_err,
                            "streamable HTTP failed, trying SSE"
                        );
                        let transport = SseTransport::connect(url, &all_headers)
                            .await
                            .with_context(|| {
                                format!(
                                    "failed to connect to MCP server '{}' at {} \
                                     (tried streamable HTTP and SSE)",
                                    entry.name, url
                                )
                            })?;
                        Arc::new(transport)
                    }
                }
            }
        };

        let mut client = McpClient::new(&entry.name, transport);
        client
            .initialize()
            .await
            .with_context(|| format!("MCP initialize handshake failed for '{}'", entry.name))?;

        let tools = client
            .list_tools()
            .await
            .with_context(|| format!("tools/list failed for MCP server '{}'", entry.name))?;

        Ok((client, tools))
    }

    /// Return all tool handlers for registration with `ToolExecutor`.
    pub async fn tool_handlers(&self) -> Vec<Arc<dyn ToolHandler>> {
        self.handlers
            .read()
            .await
            .iter()
            .map(|h| Arc::clone(h) as Arc<dyn ToolHandler>)
            .collect()
    }

    /// Number of tools across all connected servers.
    pub async fn tool_count(&self) -> usize {
        self.handlers.read().await.len()
    }

    /// Number of connected servers.
    pub async fn server_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Refresh tools from a specific server.
    ///
    /// Called when we receive a `notifications/tools/list_changed` from the
    /// server. Returns the new tool handlers so the caller can re-register them.
    pub async fn refresh_server_tools(
        &self,
        server_name: &str,
        requires_confirmation: bool,
    ) -> Result<Vec<Arc<dyn ToolHandler>>> {
        let clients = self.clients.read().await;
        let client = clients
            .get(server_name)
            .ok_or_else(|| anyhow::anyhow!("unknown MCP server: {server_name}"))?
            .clone();
        drop(clients);

        let tools = client.list_tools().await?;
        let prefix = bridge::namespaced_name(server_name, "");

        // Remove old handlers for this server.
        let mut handlers = self.handlers.write().await;
        handlers.retain(|h| !h.name().starts_with(&prefix));

        // Add new handlers.
        let mut new_handlers = Vec::new();
        for tool in &tools {
            let handler = McpToolHandler::from_remote(
                server_name,
                tool,
                client.clone(),
                requires_confirmation,
            );
            let handler = Arc::new(handler);
            new_handlers.push(Arc::clone(&handler) as Arc<dyn ToolHandler>);
            handlers.push(handler);
        }

        info!(
            server = %server_name,
            tools = tools.len(),
            "refreshed MCP server tools"
        );

        Ok(new_handlers)
    }

    /// Run a health check on all connections. Reconnect is not yet implemented.
    pub async fn health_check(&self) -> Result<()> {
        let clients = self.clients.read().await;
        for (name, client) in clients.iter() {
            if !client.is_connected() {
                warn!(server = %name, "MCP server disconnected");
                // TODO: reconnect logic
            }
        }
        Ok(())
    }

    /// Gracefully shut down all MCP client connections.
    pub async fn shutdown(&self) -> Result<()> {
        let clients = self.clients.read().await;
        for (name, client) in clients.iter() {
            if let Err(e) = client.shutdown().await {
                warn!(server = %name, error = %e, "error shutting down MCP client");
            }
        }
        Ok(())
    }
}

/// Resolve `${VAR}` references in environment variable values.
fn resolve_env_vars(env: &HashMap<String, String>) -> HashMap<String, String> {
    env.iter()
        .map(|(k, v)| {
            let resolved = resolve_env_value(v);
            (k.clone(), resolved)
        })
        .collect()
}

/// Replace `${VAR}` in a value with the corresponding environment variable.
fn resolve_env_value(value: &str) -> String {
    let mut result = value.to_string();
    // Simple regex-free approach: find ${...} patterns
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];
            let replacement = std::env::var(var_name).unwrap_or_default();
            result = format!(
                "{}{}{}",
                &result[..start],
                replacement,
                &result[start + end + 1..]
            );
        } else {
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_env_value_no_vars() {
        assert_eq!(resolve_env_value("hello"), "hello");
    }

    #[test]
    fn resolve_env_value_with_var() {
        // SAFETY: test-only, single-threaded test runner.
        unsafe { std::env::set_var("TEST_MCP_VAR", "world") };
        assert_eq!(resolve_env_value("hello ${TEST_MCP_VAR}"), "hello world");
        unsafe { std::env::remove_var("TEST_MCP_VAR") };
    }

    #[test]
    fn resolve_env_value_missing_var() {
        assert_eq!(
            resolve_env_value("token=${DEFINITELY_NOT_SET_12345}"),
            "token="
        );
    }

    #[test]
    fn resolve_env_value_multiple_vars() {
        // SAFETY: test-only, single-threaded test runner.
        unsafe { std::env::set_var("TEST_A", "foo") };
        unsafe { std::env::set_var("TEST_B", "bar") };
        assert_eq!(resolve_env_value("${TEST_A}-${TEST_B}"), "foo-bar");
        unsafe { std::env::remove_var("TEST_A") };
        unsafe { std::env::remove_var("TEST_B") };
    }
}
