//! Tool bridge — wraps a remote MCP tool as a local `ToolHandler`.
//!
//! Each remote tool is exposed under the namespaced name
//! `mcp__{server}__{tool}` to avoid collisions with builtin tools.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tracing::debug;

use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};

use crate::client::McpClient;
use crate::protocol::RemoteTool;

/// Format a namespaced tool name: `mcp__{server}__{tool}`.
pub fn namespaced_name(server: &str, tool: &str) -> String {
    format!("mcp__{server}__{tool}")
}

/// A `ToolHandler` that delegates to a remote MCP server.
pub struct McpToolHandler {
    /// Full namespaced name: `mcp__{server}__{tool}`.
    tool_name: String,
    /// Original tool name on the remote server.
    remote_name: String,
    /// Tool description from the remote server.
    description: String,
    /// JSON Schema for parameters from the remote server.
    input_schema: serde_json::Value,
    /// Whether this tool requires user confirmation.
    requires_confirmation: bool,
    /// Shared client session for the remote server.
    client: Arc<McpClient>,
}

impl McpToolHandler {
    /// Create a handler from a remote tool definition.
    pub fn from_remote(
        server_name: &str,
        remote: &RemoteTool,
        client: Arc<McpClient>,
        requires_confirmation: bool,
    ) -> Self {
        let tool_name = namespaced_name(server_name, &remote.name);
        let description = remote
            .description
            .clone()
            .unwrap_or_else(|| format!("Tool '{}' from MCP server '{}'", remote.name, server_name));

        Self {
            tool_name,
            remote_name: remote.name.clone(),
            description,
            input_schema: remote.input_schema.clone(),
            requires_confirmation,
            client,
        }
    }

    /// The MCP server name this tool belongs to.
    pub fn server_name(&self) -> &str {
        // Extract from "mcp__{server}__{tool}" → server
        self.tool_name
            .strip_prefix("mcp__")
            .and_then(|rest| rest.split("__").next())
            .unwrap_or("unknown")
    }
}

#[async_trait]
impl ToolHandler for McpToolHandler {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn params_schema(&self) -> serde_json::Value {
        self.input_schema.clone()
    }

    /// MCP tools are conservatively treated as mutating since we cannot know
    /// what the remote server does.
    fn is_mutating(&self) -> bool {
        true
    }

    fn requires_confirmation(&self) -> bool {
        self.requires_confirmation
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        debug!(
            tool = %self.tool_name,
            remote = %self.remote_name,
            "forwarding tool call to MCP server"
        );

        let arguments = serde_json::to_value(&params)?;

        match self.client.call_tool(&self.remote_name, arguments).await {
            Ok(result) => {
                let is_error = result.is_error.unwrap_or(false);

                // Collect text content.
                let text: String = result
                    .content
                    .iter()
                    .filter_map(|c| c.text.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n");

                // Collect image attachments.
                let mut attachments = Vec::new();
                for item in &result.content {
                    if item.kind == "image" {
                        if let (Some(data), Some(mime)) = (&item.data, &item.mime_type) {
                            if let Ok(bytes) = base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                data,
                            ) {
                                attachments.push(assistant_core::Attachment::new(
                                    "image",
                                    mime.clone(),
                                    bytes,
                                ));
                            }
                        }
                    }
                }

                if is_error {
                    let output = if text.is_empty() {
                        ToolOutput::error("MCP tool returned an error with no message")
                    } else {
                        ToolOutput::error(text)
                    };
                    Ok(output)
                } else {
                    let display = if text.is_empty() {
                        "(no text output)".to_string()
                    } else {
                        text
                    };
                    let mut output = ToolOutput::success(display);
                    if !attachments.is_empty() {
                        output = output.with_attachments(attachments);
                    }
                    Ok(output)
                }
            }
            Err(e) => Ok(ToolOutput::error(format!(
                "MCP tool call to '{}' failed: {e}",
                self.remote_name
            ))),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaced_name_formats_correctly() {
        assert_eq!(
            namespaced_name("github", "create-issue"),
            "mcp__github__create-issue"
        );
        assert_eq!(namespaced_name("fs", "read"), "mcp__fs__read");
    }
}
