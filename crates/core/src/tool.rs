//! The `ToolHandler` trait for primitive, self-describing tools.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::types::ExecutionContext;

/// Output returned by a [`ToolHandler`].
pub struct ToolOutput {
    /// The text content returned by the tool.
    pub content: String,
    /// Whether the tool completed successfully.
    pub success: bool,
    /// Optional structured data alongside the text content.
    pub data: Option<Value>,
}

impl ToolOutput {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            success: true,
            data: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: message.into(),
            success: false,
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
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
