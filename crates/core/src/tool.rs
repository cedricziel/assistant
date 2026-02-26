//! The `ToolHandler` trait for primitive, self-describing tools.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::types::ExecutionContext;

// ── Attachment ────────────────────────────────────────────────────────────────

/// A file attachment produced by a tool.
///
/// Attachments carry binary data (images, documents, archives, etc.) alongside
/// the text content in [`ToolOutput`].  They flow through the orchestrator and
/// are delivered to the user via the active interface (saved to disk in the CLI,
/// uploaded in Slack/Mattermost, etc.).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Attachment {
    /// Suggested filename (e.g. `"chart.png"`, `"report.csv"`).
    pub filename: String,
    /// MIME type (e.g. `"image/png"`, `"application/pdf"`).
    pub mime_type: String,
    /// Raw file content.
    pub data: Vec<u8>,
}

impl Attachment {
    /// Create a new attachment.
    pub fn new(filename: impl Into<String>, mime_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            filename: filename.into(),
            mime_type: mime_type.into(),
            data,
        }
    }

    /// Whether this attachment is an image (based on MIME type).
    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }
}

// ── ToolOutput ────────────────────────────────────────────────────────────────

/// Output returned by a [`ToolHandler`].
pub struct ToolOutput {
    /// The text content returned by the tool.
    pub content: String,
    /// Whether the tool completed successfully.
    pub success: bool,
    /// Optional structured data alongside the text content.
    pub data: Option<Value>,
    /// File attachments produced by the tool (images, documents, etc.).
    ///
    /// These are collected by the orchestrator and forwarded to the active
    /// interface for delivery to the user.
    pub attachments: Vec<Attachment>,
}

impl ToolOutput {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            success: true,
            data: None,
            attachments: Vec::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: message.into(),
            success: false,
            data: None,
            attachments: Vec::new(),
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Attach a single file to this output.
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Attach multiple files to this output.
    pub fn with_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments.extend(attachments);
        self
    }
}

/// A primitive, self-describing tool handler.
///
/// Every method except `run` has a required return value (no `Option`),
/// because tools are *always* self-describing.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// The tool name (kebab-case, e.g. "file-read").
    fn name(&self) -> &str;

    /// Short description of what this tool does (1-2 sentences).
    fn description(&self) -> &str;

    /// Full JSON Schema object for the tool's parameters.
    ///
    /// Must return a proper JSON Schema with `type: "object"`, `properties`,
    /// and `required` (listing mandatory parameters). Example:
    /// ```json
    /// { "type": "object",
    ///   "properties": { "path": {"type":"string","description":"..."} },
    ///   "required": ["path"] }
    /// ```
    fn params_schema(&self) -> Value;

    /// Whether this tool mutates state (used for SafetyGate).
    fn is_mutating(&self) -> bool {
        false
    }

    /// Whether the user must confirm before this tool runs.
    fn requires_confirmation(&self) -> bool {
        false
    }

    /// Optional JSON Schema describing the structure of `ToolOutput.data`.
    ///
    /// Return `Some(schema)` if this tool populates `ToolOutput.data` with
    /// structured JSON. The schema is stored in `SkillDef` metadata and
    /// included in tool observations so the model knows what to expect.
    fn output_schema(&self) -> Option<Value> {
        None
    }

    /// Execute the tool with the given parameters.
    async fn run(
        &self,
        params: HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Attachment ────────────────────────────────────────────────────────────

    #[test]
    fn attachment_new_sets_fields() {
        let data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes
        let a = Attachment::new("chart.png", "image/png", data.clone());
        assert_eq!(a.filename, "chart.png");
        assert_eq!(a.mime_type, "image/png");
        assert_eq!(a.data, data);
    }

    #[test]
    fn attachment_is_image_true_for_image_types() {
        let cases = [
            "image/png",
            "image/jpeg",
            "image/gif",
            "image/webp",
            "image/svg+xml",
        ];
        for mime in cases {
            let a = Attachment::new("f", mime, vec![]);
            assert!(a.is_image(), "{mime} should be recognised as image");
        }
    }

    #[test]
    fn attachment_is_image_false_for_non_image_types() {
        let cases = [
            "application/pdf",
            "text/plain",
            "application/octet-stream",
            "video/mp4",
        ];
        for mime in cases {
            let a = Attachment::new("f", mime, vec![]);
            assert!(!a.is_image(), "{mime} should NOT be recognised as image");
        }
    }

    #[test]
    fn attachment_clone() {
        let a = Attachment::new("doc.pdf", "application/pdf", vec![1, 2, 3]);
        let b = a.clone();
        assert_eq!(a.filename, b.filename);
        assert_eq!(a.mime_type, b.mime_type);
        assert_eq!(a.data, b.data);
    }

    // ── ToolOutput ───────────────────────────────────────────────────────────

    #[test]
    fn tool_output_success_has_empty_attachments() {
        let out = ToolOutput::success("ok");
        assert!(out.success);
        assert!(out.attachments.is_empty());
    }

    #[test]
    fn tool_output_error_has_empty_attachments() {
        let out = ToolOutput::error("fail");
        assert!(!out.success);
        assert!(out.attachments.is_empty());
    }

    #[test]
    fn tool_output_with_attachment_adds_one() {
        let a = Attachment::new("file.txt", "text/plain", b"hello".to_vec());
        let out = ToolOutput::success("ok").with_attachment(a);
        assert_eq!(out.attachments.len(), 1);
        assert_eq!(out.attachments[0].filename, "file.txt");
        assert_eq!(out.attachments[0].data, b"hello");
    }

    #[test]
    fn tool_output_with_attachments_adds_many() {
        let a1 = Attachment::new("a.png", "image/png", vec![1]);
        let a2 = Attachment::new("b.pdf", "application/pdf", vec![2]);
        let out = ToolOutput::success("ok").with_attachments(vec![a1, a2]);
        assert_eq!(out.attachments.len(), 2);
        assert_eq!(out.attachments[0].filename, "a.png");
        assert_eq!(out.attachments[1].filename, "b.pdf");
    }

    #[test]
    fn tool_output_chained_builders() {
        let a1 = Attachment::new("a.txt", "text/plain", vec![1]);
        let a2 = Attachment::new("b.txt", "text/plain", vec![2]);
        let a3 = Attachment::new("c.txt", "text/plain", vec![3]);
        let out = ToolOutput::success("ok")
            .with_data(serde_json::json!({"key": "value"}))
            .with_attachment(a1)
            .with_attachments(vec![a2, a3]);
        assert_eq!(out.attachments.len(), 3);
        assert!(out.data.is_some());
        assert!(out.success);
    }

    #[test]
    fn tool_output_with_attachment_preserves_content() {
        let a = Attachment::new("f", "text/plain", vec![]);
        let out = ToolOutput::success("my content").with_attachment(a);
        assert_eq!(out.content, "my content");
        assert!(out.success);
    }
}
