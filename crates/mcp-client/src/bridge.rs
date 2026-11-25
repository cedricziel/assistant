//! Tool bridge — wraps a remote MCP tool as a local `ToolHandler`.
//!
//! Each remote tool is exposed under the namespaced name
//! `mcp__{server}__{tool}` to avoid collisions with builtin tools.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{CallToolResult, ContentBlock, ResourceContents, Tool};
use tracing::{debug, warn};

use assistant_core::types::conversation::ExecutionContext;
use assistant_core::{Attachment, ToolHandler, ToolOutput};

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
            Ok(result) => Ok(output_from_result(result)),
            Err(e) => Ok(ToolOutput::error(format!(
                "MCP tool call to '{}' failed: {e:#}",
                self.remote_name
            ))),
        }
    }
}

/// Convert a tool result into a [`ToolOutput`] without dropping content.
///
/// Text joins the output text; images, audio and embedded binary resources
/// become attachments; resource links and embedded text resources are
/// rendered into the text with their URI. `structuredContent` becomes the
/// output text when no text block describes it, and always rides along as
/// `data`.
fn output_from_result(result: CallToolResult) -> ToolOutput {
    let mut converted = Converted::default();
    for block in &result.content {
        converted.push(block);
    }
    let has_text = result
        .content
        .iter()
        .any(|block| matches!(block, ContentBlock::Text(_)));
    if !has_text && let Some(structured) = &result.structured_content {
        converted.parts.push(structured.to_string());
    }

    let text = converted.parts.join("\n");
    let mut output = if result.is_error.unwrap_or(false) {
        if text.is_empty() {
            ToolOutput::error("MCP tool returned an error with no message")
        } else {
            ToolOutput::error(text)
        }
    } else if text.is_empty() {
        ToolOutput::success("(no text output)")
    } else {
        ToolOutput::success(text)
    };
    output = output.with_attachments(converted.attachments);
    match result.structured_content {
        Some(structured) => output.with_data(structured),
        None => output,
    }
}

/// The text parts and attachments of a result's content blocks.
#[derive(Default)]
struct Converted {
    parts: Vec<String>,
    attachments: Vec<Attachment>,
}

impl Converted {
    fn push(&mut self, block: &ContentBlock) {
        match block {
            ContentBlock::Text(text) => self.parts.push(text.text.clone()),
            ContentBlock::Image(image) => {
                self.push_encoded("image", &image.mime_type, &image.data);
            }
            ContentBlock::Audio(audio) => {
                self.push_encoded("audio", &audio.mime_type, &audio.data);
            }
            ContentBlock::ResourceLink(link) => {
                self.parts
                    .push(format!("[resource: {}] {}", link.name, link.uri));
            }
            ContentBlock::Resource(embedded) => match &embedded.resource {
                ResourceContents::TextResourceContents { uri, text, .. } => {
                    self.parts.push(format!("[resource: {uri}]\n{text}"));
                }
                ResourceContents::BlobResourceContents {
                    uri,
                    mime_type,
                    blob,
                    ..
                } => {
                    let name = uri
                        .rsplit('/')
                        .find(|s| !s.is_empty())
                        .unwrap_or("resource");
                    let mime = mime_type.as_deref().unwrap_or("application/octet-stream");
                    if self.push_encoded(name, mime, blob) {
                        self.parts.push(format!("[attachment: {uri}]"));
                    }
                }
                _ => self.note_unsupported("embedded resource"),
            },
            _ => self.note_unsupported("content block"),
        }
    }

    /// Attach base64 `data`; returns whether it decoded. A payload that does
    /// not decode is reported in the text instead of vanishing.
    fn push_encoded(&mut self, filename: &str, mime_type: &str, data: &str) -> bool {
        match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data) {
            Ok(bytes) => {
                self.attachments
                    .push(Attachment::new(filename, mime_type, bytes));
                true
            }
            Err(error) => {
                warn!(%error, mime_type, "MCP content is not valid base64");
                self.parts
                    .push(format!("[{mime_type} content could not be decoded]"));
                false
            }
        }
    }

    fn note_unsupported(&mut self, what: &str) {
        warn!(
            what,
            "MCP result contains content this client does not understand"
        );
        self.parts.push(format!("[unsupported {what} omitted]"));
    }
}

/// Whether the Persona may be offered `tool`.
///
/// MCP Apps visibility lives in `_meta.ui.visibility` (the deprecated flat
/// `_meta["ui/visibility"]` is accepted too) and defaults to
/// `["model", "app"]`; a tool without `"model"` there is for apps only.
pub fn is_model_visible(tool: &Tool) -> bool {
    let Some(meta) = &tool.meta else {
        return true;
    };
    let visibility = meta
        .0
        .get("ui")
        .and_then(|ui| ui.get("visibility"))
        .or_else(|| meta.0.get("ui/visibility"));
    match visibility.and_then(serde_json::Value::as_array) {
        Some(audiences) => audiences.iter().any(|a| a.as_str() == Some("model")),
        None => true,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use rmcp::model::{MetaObject, Resource};
    use serde_json::json;

    fn tool_with_meta(meta: serde_json::Value) -> Tool {
        let serde_json::Value::Object(meta) = meta else {
            return Tool::new("t", "d", Arc::new(serde_json::Map::new()));
        };
        Tool::new("t", "d", Arc::new(serde_json::Map::new())).with_meta(MetaObject(meta))
    }

    fn result_with_structured(
        content: Vec<ContentBlock>,
        structured: serde_json::Value,
    ) -> CallToolResult {
        let mut result = CallToolResult::success(content);
        result.structured_content = Some(structured);
        result
    }

    #[test]
    fn audio_becomes_an_attachment_with_its_mime_type() {
        let output = output_from_result(CallToolResult::success(vec![ContentBlock::audio(
            "AAE=",
            "audio/wav",
        )]));
        assert_eq!(
            output.attachments,
            vec![assistant_core::Attachment::new(
                "audio",
                "audio/wav",
                vec![0, 1]
            )],
            "audio block should carry its mime type and decoded bytes"
        );
    }

    #[test]
    fn resource_link_is_rendered_into_the_text_with_its_uri() {
        let link = Resource::new("file:///notes.md", "notes").with_mime_type("text/markdown");
        let output = output_from_result(CallToolResult::success(vec![
            ContentBlock::text("see below"),
            ContentBlock::ResourceLink(link),
        ]));
        assert!(
            output.content.contains("see below") && output.content.contains("file:///notes.md"),
            "text and link uri should both be present, got: {}",
            output.content
        );
    }

    #[test]
    fn embedded_text_resource_is_rendered_with_its_uri_and_text() {
        let output =
            output_from_result(CallToolResult::success(vec![ContentBlock::embedded_text(
                "file:///a.txt",
                "alpha body",
            )]));
        assert!(
            output.content.contains("file:///a.txt") && output.content.contains("alpha body"),
            "uri and text should be rendered, got: {}",
            output.content
        );
    }

    #[test]
    fn embedded_blob_resource_becomes_an_attachment() {
        let blob =
            ResourceContents::blob("AAE=", "file:///a.bin").with_mime_type("application/pdf");
        let output =
            output_from_result(CallToolResult::success(vec![ContentBlock::resource(blob)]));
        assert_eq!(
            output.attachments,
            vec![assistant_core::Attachment::new(
                "a.bin",
                "application/pdf",
                vec![0, 1]
            )],
            "blob should be an attachment named after the uri's last segment"
        );
        assert!(
            output.content.contains("file:///a.bin"),
            "the output text should still mention the blob's uri, got: {}",
            output.content
        );
    }

    #[test]
    fn structured_content_without_text_becomes_the_output_text() {
        let output = output_from_result(result_with_structured(vec![], json!({"temp": 21})));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&output.content).ok(),
            Some(json!({"temp": 21})),
            "output text should be the JSON of the structured content"
        );
        assert_eq!(
            output.data,
            Some(json!({"temp": 21})),
            "structured content should also ride along as data"
        );
    }

    #[test]
    fn structured_content_does_not_replace_existing_text() {
        let output = output_from_result(result_with_structured(
            vec![ContentBlock::text("21 degrees")],
            json!({"temp": 21}),
        ));
        assert_eq!(
            output.content, "21 degrees",
            "text that already describes the result wins"
        );
    }

    #[test]
    fn undecodable_image_is_reported_not_dropped() {
        let output = output_from_result(CallToolResult::success(vec![ContentBlock::image(
            "not base64!!",
            "image/png",
        )]));
        assert!(
            output.attachments.is_empty() && output.content.contains("image/png"),
            "a broken image should leave a note in the text, got: {}",
            output.content
        );
    }

    #[test]
    fn tools_without_ui_meta_are_visible_to_the_model() {
        assert!(is_model_visible(&Tool::new(
            "t",
            "d",
            Arc::new(serde_json::Map::new())
        )));
    }

    #[test]
    fn app_only_tool_is_hidden_from_the_model() {
        assert!(!is_model_visible(&tool_with_meta(
            json!({"ui": {"visibility": ["app"]}})
        )));
    }

    #[test]
    fn tool_visible_to_model_and_app_is_offered() {
        assert!(is_model_visible(&tool_with_meta(
            json!({"ui": {"visibility": ["model", "app"]}})
        )));
        assert!(is_model_visible(&tool_with_meta(
            json!({"ui": {"visibility": ["model"]}})
        )));
    }

    #[test]
    fn ui_meta_without_visibility_defaults_to_visible() {
        assert!(is_model_visible(&tool_with_meta(
            json!({"ui": {"resourceUri": "ui://x"}})
        )));
    }

    #[test]
    fn deprecated_flat_visibility_key_is_honoured() {
        assert!(!is_model_visible(&tool_with_meta(
            json!({"ui/visibility": ["app"]})
        )));
    }

    #[test]
    fn empty_visibility_list_hides_the_tool() {
        assert!(!is_model_visible(&tool_with_meta(
            json!({"ui": {"visibility": []}})
        )));
    }

    #[test]
    fn namespaced_name_formats_correctly() {
        assert_eq!(
            namespaced_name("github", "create-issue"),
            "mcp__github__create-issue"
        );
        assert_eq!(namespaced_name("fs", "read"), "mcp__fs__read");
    }
}
