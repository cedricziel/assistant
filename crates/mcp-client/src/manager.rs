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
use rmcp::model::Tool;
use rmcp::ServiceExt;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use assistant_core::{McpServerEntry, McpTransportConfig, McpTrustLevel, ToolHandler};

use crate::bridge::{self, McpToolHandler};
use crate::client::McpClient;

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

            // Validate server name: must be non-empty, lowercase alphanumeric + hyphens.
            // Invalid names would create ambiguous tool namespaces and break
            // prefix-based tool unregistration.
            if !is_valid_server_name(&entry.name) {
                error!(
                    server = %entry.name,
                    "invalid MCP server name: must be non-empty, lowercase alphanumeric + hyphens only; skipping"
                );
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

    /// Connect to a single MCP server using rmcp and perform the initialize
    /// handshake, then list tools.
    async fn connect_one(entry: &McpServerEntry) -> Result<(McpClient, Vec<Tool>)> {
        let service = match &entry.transport {
            McpTransportConfig::Stdio { command } => {
                let resolved_env = resolve_env_vars(&entry.env);

                // Build a tokio::process::Command from the command parts.
                let (program, args) = command
                    .split_first()
                    .ok_or_else(|| anyhow::anyhow!("MCP server '{}': empty command", entry.name))?;
                let mut cmd = tokio::process::Command::new(program);
                cmd.args(args);
                for (k, v) in &resolved_env {
                    cmd.env(k, v);
                }

                let transport = rmcp::transport::child_process::TokioChildProcess::new(cmd)
                    .with_context(|| {
                        format!("failed to spawn MCP server '{}': {:?}", entry.name, command)
                    })?;

                ().serve(transport).await.map_err(|e| {
                    anyhow::anyhow!("MCP initialize handshake failed for '{}': {e}", entry.name)
                })?
            }
            McpTransportConfig::Http { url, headers } => {
                // Only explicitly configured `headers` are sent over HTTP.
                // `entry.env` is for stdio process environment only — promoting
                // it to HTTP headers would leak sensitive values.
                let resolved_headers = resolve_env_vars(headers);

                // Build rmcp StreamableHttpClientTransportConfig.
                let mut config = rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(url.as_str());

                // Convert string headers to http types.
                let mut http_headers = HashMap::new();
                for (k, v) in &resolved_headers {
                    if let (Ok(name), Ok(value)) = (
                        http::HeaderName::try_from(k.as_str()),
                        http::HeaderValue::try_from(v.as_str()),
                    ) {
                        http_headers.insert(name, value);
                    } else {
                        warn!(
                            server = %entry.name,
                            header = %k,
                            "skipping invalid HTTP header"
                        );
                    }
                }
                if !http_headers.is_empty() {
                    config = config.custom_headers(http_headers);
                }

                let transport =
                    rmcp::transport::StreamableHttpClientTransport::<reqwest::Client>::from_config(
                        config,
                    );
                ().serve(transport).await.map_err(|e| {
                    anyhow::anyhow!(
                        "MCP initialize handshake failed for '{}' at {}: {e}",
                        entry.name,
                        url
                    )
                })?
            }
        };

        let client = McpClient::new(&entry.name, service);

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
    ///
    /// The confirmation requirement is derived from the server's configured
    /// trust level — callers cannot override it to prevent accidental or
    /// malicious downgrades for untrusted servers.
    pub async fn refresh_server_tools(
        &self,
        server_name: &str,
    ) -> Result<Vec<Arc<dyn ToolHandler>>> {
        let servers = self.servers.read().await;
        let state = servers
            .get(server_name)
            .ok_or_else(|| anyhow::anyhow!("no active connection for server: {server_name}"))?;
        let client = state
            .client
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no active connection for server: {server_name}"))?;
        let requires_confirmation = state.entry.trust != McpTrustLevel::Trust;
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
    /// **Phase 1** (sequential): detect dead servers, remove stale tools.
    /// **Phase 2** (parallel): reconnect all disconnected servers concurrently
    /// so one slow server doesn't block recovery of others.
    ///
    /// `register_tool` and `unregister_prefix` are callbacks so the manager
    /// doesn't need to depend on `ToolExecutor` directly.
    pub async fn health_check(
        &self,
        register_tool: &Arc<dyn Fn(Arc<dyn ToolHandler>) + Send + Sync>,
        unregister_prefix: &Arc<dyn Fn(&str) + Send + Sync>,
    ) {
        // -- Phase 1: detect dead connections (needs write lock) ---------------
        let mut reconnect_work: Vec<(McpServerEntry, Duration)> = Vec::new();

        {
            let mut servers = self.servers.write().await;
            let names: Vec<String> = servers.keys().cloned().collect();

            for name in names {
                let Some(state) = servers.get_mut(&name) else {
                    continue;
                };

                // Check if a connected server has died.
                if let Some(ref client) = state.client {
                    if !client.is_connected() {
                        warn!(server = %name, "MCP server disconnected, removing tools");
                        let prefix = bridge::namespaced_name(&name, "");
                        unregister_prefix(&prefix);

                        let mut handlers = self.handlers.write().await;
                        handlers.retain(|h| !h.name().starts_with(&prefix));
                        drop(handlers);

                        state.client = None;
                        state.consecutive_failures = 0;
                    }
                }

                // Collect disconnected servers for parallel reconnection.
                if state.client.is_none() {
                    if state.consecutive_failures >= MAX_RECONNECT_ATTEMPTS {
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
                        "scheduling MCP reconnection"
                    );
                    reconnect_work.push((state.entry.clone(), backoff));
                }
            }
        } // write lock released

        if reconnect_work.is_empty() {
            return;
        }

        // -- Phase 2: reconnect concurrently (no lock held) -------------------
        type ReconnectResult = (McpServerEntry, Result<(McpClient, Vec<Tool>)>);
        let results: Vec<ReconnectResult> =
            futures::future::join_all(reconnect_work.into_iter().map(
                |(entry, backoff)| async move {
                    tokio::time::sleep(backoff).await;
                    let result = Self::connect_one(&entry).await;
                    (entry, result)
                },
            ))
            .await;

        // -- Phase 3: apply results (needs write lock) ------------------------
        for (entry, result) in results {
            match result {
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

/// Check that a server name is valid: non-empty, lowercase ASCII alphanumeric + hyphens.
fn is_valid_server_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
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
///
/// Uses a single-pass scan so that replacement text is never re-scanned.
/// This prevents infinite loops when an env var's value contains `${…}`
/// syntax (including self-referential values like `FOO=${FOO}`).
fn resolve_env_value(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut remaining = value;

    while let Some(start) = remaining.find("${") {
        // Append everything before the `${`.
        result.push_str(&remaining[..start]);
        let after_dollar_brace = &remaining[start + 2..];

        if let Some(end) = after_dollar_brace.find('}') {
            let var_name = &after_dollar_brace[..end];
            let replacement = std::env::var(var_name).unwrap_or_default();
            result.push_str(&replacement);
            // Advance past the closing `}` — never re-scan the replacement.
            remaining = &after_dollar_brace[end + 1..];
        } else {
            // Unterminated `${…` — keep the rest as-is and stop.
            result.push_str(&remaining[start..]);
            remaining = "";
            break;
        }
    }

    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- resolve_env_value tests --

    #[test]
    fn resolve_env_value_no_vars() {
        assert_eq!(
            resolve_env_value("hello"),
            "hello",
            "plain text without variables should be unchanged"
        );
    }

    #[test]
    fn resolve_env_value_with_var() {
        // SAFETY: test-only, single-threaded test runner.
        unsafe { std::env::set_var("TEST_MCP_VAR", "world") };
        assert_eq!(
            resolve_env_value("hello ${TEST_MCP_VAR}"),
            "hello world",
            "should substitute env var value"
        );
        unsafe { std::env::remove_var("TEST_MCP_VAR") };
    }

    #[test]
    fn resolve_env_value_missing_var() {
        assert_eq!(
            resolve_env_value("token=${DEFINITELY_NOT_SET_12345}"),
            "token=",
            "missing env var should resolve to empty string"
        );
    }

    #[test]
    fn resolve_env_value_multiple_vars() {
        // SAFETY: test-only, single-threaded test runner.
        unsafe { std::env::set_var("TEST_A", "foo") };
        unsafe { std::env::set_var("TEST_B", "bar") };
        assert_eq!(
            resolve_env_value("${TEST_A}-${TEST_B}"),
            "foo-bar",
            "multiple variables should all be resolved"
        );
        unsafe { std::env::remove_var("TEST_A") };
        unsafe { std::env::remove_var("TEST_B") };
    }

    #[test]
    fn resolve_env_value_self_referential_does_not_loop() {
        // A self-referential env var must not cause an infinite loop.
        // SAFETY: test-only, single-threaded test runner.
        unsafe { std::env::set_var("TEST_SELF_REF", "${TEST_SELF_REF}") };
        let result = resolve_env_value("${TEST_SELF_REF}");
        // The single-pass scanner resolves TEST_SELF_REF to its literal
        // value "${TEST_SELF_REF}" and does not re-scan that output.
        assert_eq!(
            result, "${TEST_SELF_REF}",
            "self-referential env var should not cause infinite loop"
        );
        unsafe { std::env::remove_var("TEST_SELF_REF") };
    }

    #[test]
    fn resolve_env_value_unterminated_kept_as_is() {
        assert_eq!(
            resolve_env_value("prefix ${UNCLOSED"),
            "prefix ${UNCLOSED",
            "unterminated ${{ pattern should be preserved verbatim"
        );
    }

    // -- backoff_duration tests --

    #[test]
    fn backoff_duration_exponential() {
        assert_eq!(
            backoff_duration(0),
            Duration::from_secs(2),
            "attempt 0 should use initial backoff"
        );
        assert_eq!(
            backoff_duration(1),
            Duration::from_secs(4),
            "attempt 1 should double"
        );
        assert_eq!(
            backoff_duration(2),
            Duration::from_secs(8),
            "attempt 2 should quadruple"
        );
        assert_eq!(backoff_duration(3), Duration::from_secs(16), "attempt 3");
        assert_eq!(backoff_duration(4), Duration::from_secs(32), "attempt 4");
    }

    #[test]
    fn backoff_duration_capped() {
        assert_eq!(
            backoff_duration(5),
            MAX_BACKOFF,
            "should cap at MAX_BACKOFF"
        );
        assert_eq!(
            backoff_duration(10),
            MAX_BACKOFF,
            "large attempt should still cap"
        );
        assert_eq!(
            backoff_duration(100),
            MAX_BACKOFF,
            "very large attempt should still cap"
        );
    }

    // -- is_valid_server_name tests --

    #[test]
    fn valid_server_names() {
        assert!(is_valid_server_name("github"), "lowercase alpha is valid");
        assert!(is_valid_server_name("my-server"), "hyphens are valid");
        assert!(is_valid_server_name("server-2"), "digits are valid");
        assert!(is_valid_server_name("a"), "single character is valid");
    }

    #[test]
    fn invalid_server_names() {
        assert!(!is_valid_server_name(""), "empty string is invalid");
        assert!(!is_valid_server_name("MyServer"), "uppercase is invalid");
        assert!(
            !is_valid_server_name("my_server"),
            "underscores are invalid"
        );
        assert!(!is_valid_server_name("my server"), "spaces are invalid");
        assert!(!is_valid_server_name("server.name"), "dots are invalid");
    }
}
