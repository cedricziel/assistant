//! Tool bridge — wraps a remote MCP tool as a local `ToolHandler`.
//!
//! Each remote tool is exposed under the namespaced name
//! `mcp__{server}__{tool}` to avoid collisions with builtin tools.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{ContentBlock, Tool};
use tracing::debug;

use assistant_core::types::conversation::ExecutionContext;
use assistant_core::{ToolHandler, ToolOutput};

use crate::client::McpClient;

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
    /// Create a handler from an rmcp `Tool` definition.
    pub fn from_remote(
        server_name: &str,
        remote: &Tool,
        client: Arc<McpClient>,
        requires_confirmation: bool,
    ) -> Self {
        let tool_name = namespaced_name(server_name, &remote.name);
        let description = remote.description.as_deref().unwrap_or("").to_string();
        let description = if description.is_empty() {
            format!("Tool '{}' from MCP server '{}'", remote.name, server_name)
        } else {
            description
        };

        // Convert Arc<JsonObject> → serde_json::Value for the ToolHandler trait.
        let input_schema = serde_json::Value::Object(remote.input_schema.as_ref().clone());

        Self {
            tool_name,
            remote_name: remote.name.to_string(),
            description,
            input_schema,
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

        // Convert HashMap → JsonObject (serde_json::Map).
        let arguments: serde_json::Map<String, serde_json::Value> = params.into_iter().collect();
        let arguments = if arguments.is_empty() {
            None
        } else {
            Some(arguments)
        };

        match self.client.call_tool(&self.remote_name, arguments).await {
            Ok(result) => {
                let is_error = result.is_error.unwrap_or(false);

                // Collect text content from ContentBlock items.
                let text = extract_text(&result.content);

                // Collect image attachments from ContentBlock items.
                let attachments = extract_image_attachments(&result.content);

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

/// Extract text content from rmcp `ContentBlock` items.
fn extract_text(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            ContentBlock::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract image attachments from rmcp `ContentBlock` items.
fn extract_image_attachments(content: &[ContentBlock]) -> Vec<assistant_core::Attachment> {
    let mut attachments = Vec::new();
    for item in content {
        if let ContentBlock::Image(img) = item
            && let Ok(bytes) =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &img.data)
        {
            attachments.push(assistant_core::Attachment::new(
                "image",
                img.mime_type.clone(),
                bytes,
            ));
        }
    }
    attachments
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
