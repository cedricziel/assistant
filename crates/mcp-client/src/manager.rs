//! MCP client manager — coordinates all external MCP server connections.
//!
//! Starts configured servers, discovers their tools, and produces
//! `ToolHandler` instances ready for registration with `ToolExecutor`.
//!
//! The manager also handles resilience: a background health-check loop
//! detects dead servers, removes their stale tools from the executor,
//! and attempts reconnection with exponential back-off.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use assistant_core::{McpServerEntry, McpTransportConfig, McpTrustLevel, ToolHandler};

use crate::bridge::{self, McpToolHandler};
use crate::client::McpClient;
use crate::transport::sse::SseTransport;
use crate::transport::stdio::StdioTransport;
use crate::transport::streamable::StreamableHttpTransport;

/// Maximum number of consecutive reconnection attempts before giving up on a
/// server until the next health-check cycle.
const MAX_RECONNECT_ATTEMPTS: u32 = 5;

/// Initial back-off delay between reconnection attempts.
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);

/// Maximum back-off delay.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Default interval between health-check cycles.
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// Per-server state tracked by the manager.
struct ServerState {
    /// The original configuration (needed for reconnection).
    entry: McpServerEntry,
    /// The active client session (if connected).
    client: Option<Arc<McpClient>>,
    /// Number of consecutive failed reconnection attempts since the last
    /// successful connection.
    consecutive_failures: u32,
}

/// Manages all MCP client connections and their bridged tools.
pub struct McpClientManager {
    /// Per-server state keyed by server name.
    servers: RwLock<HashMap<String, ServerState>>,
    /// All tool handlers produced from remote servers.
    handlers: RwLock<Vec<Arc<McpToolHandler>>>,
}

impl McpClientManager {
    /// Connect to all configured MCP servers and discover their tools.
    ///
    /// Servers that fail to connect are logged and skipped — a single broken
    /// server should not prevent the assistant from starting. The health-check
    /// loop will retry them later.
    pub async fn start(entries: &[McpServerEntry]) -> Result<Self> {
        let mut servers = HashMap::new();
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
                    servers.insert(
                        entry.name.clone(),
                        ServerState {
                            entry: entry.clone(),
                            client: Some(client),
                            consecutive_failures: 0,
                        },
                    );
                }
                Err(e) => {
                    error!(
                        server = %entry.name,
                        error = %e,
                        "failed to connect to MCP server, skipping (will retry)"
                    );
                    servers.insert(
                        entry.name.clone(),
                        ServerState {
                            entry: entry.clone(),
                            client: None,
                            consecutive_failures: 1,
                        },
                    );
                }
            }
        }

        Ok(Self {
            servers: RwLock::new(servers),
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
        self.servers
            .read()
            .await
            .values()
            .filter(|s| s.client.is_some())
            .count()
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
        let servers = self.servers.read().await;
        let client = servers
            .get(server_name)
            .and_then(|s| s.client.clone())
            .ok_or_else(|| anyhow::anyhow!("no active connection for server: {server_name}"))?;
        drop(servers);

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

    /// Run a single health-check cycle.
    ///
    /// For each server:
    /// - If connected, verify with `is_connected()`. If dead, remove stale
    ///   tools and mark for reconnection.
    /// - If disconnected, attempt reconnection with back-off. On success,
    ///   re-register tools on the executor.
    ///
    /// `register_tool` and `unregister_prefix` are callbacks so the manager
    /// doesn't need to depend on `ToolExecutor` directly.
    pub async fn health_check(
        &self,
        register_tool: &Arc<dyn Fn(Arc<dyn ToolHandler>) + Send + Sync>,
        unregister_prefix: &Arc<dyn Fn(&str) + Send + Sync>,
    ) {
        let server_names: Vec<String> = { self.servers.read().await.keys().cloned().collect() };

        for name in server_names {
            let mut servers = self.servers.write().await;
            let Some(state) = servers.get_mut(&name) else {
                continue;
            };

            // Check if a connected server has died.
            if let Some(ref client) = state.client {
                if !client.is_connected() {
                    warn!(server = %name, "MCP server disconnected, removing tools");
                    let prefix = bridge::namespaced_name(&name, "");
                    unregister_prefix(&prefix);

                    // Remove handlers from our internal list.
                    let mut handlers = self.handlers.write().await;
                    handlers.retain(|h| !h.name().starts_with(&prefix));
                    drop(handlers);

                    state.client = None;
                    // Don't count this as a failure — the server was working.
                    // Reset so we retry immediately.
                    state.consecutive_failures = 0;
                }
            }

            // Attempt reconnection for disconnected servers.
            if state.client.is_none() {
                if state.consecutive_failures >= MAX_RECONNECT_ATTEMPTS {
                    // Back off: only retry every MAX_RECONNECT_ATTEMPTS cycles.
                    // After hitting the cap, reset to allow another burst.
                    debug!(
                        server = %name,
                        failures = state.consecutive_failures,
                        "exceeded max reconnect attempts, will retry next cycle"
                    );
                    state.consecutive_failures = 0;
                    continue;
                }

                let backoff = backoff_duration(state.consecutive_failures);
                debug!(
                    server = %name,
                    attempt = state.consecutive_failures + 1,
                    backoff_ms = backoff.as_millis(),
                    "attempting MCP reconnection"
                );

                // Drop the lock during the potentially slow connect.
                let entry = state.entry.clone();
                drop(servers);

                // Small back-off before reconnecting.
                tokio::time::sleep(backoff).await;

                match Self::connect_one(&entry).await {
                    Ok((client, tools)) => {
                        let client = Arc::new(client);
                        let requires_confirmation = entry.trust != McpTrustLevel::Trust;

                        let mut new_tool_handlers = Vec::new();
                        for tool in &tools {
                            let handler = McpToolHandler::from_remote(
                                &entry.name,
                                tool,
                                client.clone(),
                                requires_confirmation,
                            );
                            let handler = Arc::new(handler);
                            register_tool(Arc::clone(&handler) as Arc<dyn ToolHandler>);
                            new_tool_handlers.push(handler);
                        }

                        // Update internal state.
                        let mut handlers = self.handlers.write().await;
                        handlers.extend(new_tool_handlers);
                        drop(handlers);

                        let mut servers = self.servers.write().await;
                        if let Some(state) = servers.get_mut(&entry.name) {
                            state.client = Some(client);
                            state.consecutive_failures = 0;
                        }

                        info!(
                            server = %entry.name,
                            tools = tools.len(),
                            "MCP server reconnected"
                        );
                    }
                    Err(e) => {
                        let mut servers = self.servers.write().await;
                        if let Some(state) = servers.get_mut(&entry.name) {
                            state.consecutive_failures += 1;
                        }
                        warn!(
                            server = %entry.name,
                            error = %e,
                            "MCP reconnection failed"
                        );
                    }
                }
            }
        }
    }

    /// Spawn a background task that periodically health-checks all MCP server
    /// connections and reconnects dead ones.
    ///
    /// The task runs until the returned [`tokio::task::JoinHandle`] is aborted
    /// or the process exits.
    pub fn spawn_health_loop(
        self: &Arc<Self>,
        register_tool: Arc<dyn Fn(Arc<dyn ToolHandler>) + Send + Sync>,
        unregister_prefix: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> tokio::task::JoinHandle<()> {
        let manager = Arc::clone(self);
        let reg = register_tool;
        let unreg = unregister_prefix;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(HEALTH_CHECK_INTERVAL).await;
                manager.health_check(&reg, &unreg).await;
            }
        })
    }

    /// Gracefully shut down all MCP client connections.
    pub async fn shutdown(&self) -> Result<()> {
        let servers = self.servers.read().await;
        for (name, state) in servers.iter() {
            if let Some(ref client) = state.client {
                if let Err(e) = client.shutdown().await {
                    warn!(server = %name, error = %e, "error shutting down MCP client");
                }
            }
        }
        Ok(())
    }
}

/// Compute exponential back-off duration for the given attempt number.
fn backoff_duration(attempt: u32) -> Duration {
    let multiplier = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
    let secs = INITIAL_BACKOFF.as_secs().saturating_mul(multiplier);
    Duration::from_secs(secs).min(MAX_BACKOFF)
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

    #[test]
    fn backoff_duration_exponential() {
        assert_eq!(backoff_duration(0), Duration::from_secs(2));
        assert_eq!(backoff_duration(1), Duration::from_secs(4));
        assert_eq!(backoff_duration(2), Duration::from_secs(8));
        assert_eq!(backoff_duration(3), Duration::from_secs(16));
        assert_eq!(backoff_duration(4), Duration::from_secs(32));
    }

    #[test]
    fn backoff_duration_capped() {
        assert_eq!(backoff_duration(5), MAX_BACKOFF);
        assert_eq!(backoff_duration(10), MAX_BACKOFF);
        assert_eq!(backoff_duration(100), MAX_BACKOFF);
    }
}
