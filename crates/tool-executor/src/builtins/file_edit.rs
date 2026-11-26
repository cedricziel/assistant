//! Builtin handler for file-edit tool — surgically replaces text in a file.

use std::collections::HashMap;
use std::fs;

use anyhow::Result;
use assistant_core::tool_render::{DiffOperation, ToolRenderData};
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use tracing::debug;

use super::render_diff::code_diff;

#[derive(Default)]
pub struct FileEditHandler {
    extra_roots: Vec<String>,
}

impl FileEditHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allow access to these directories in addition to the persona workspace.
    pub fn with_extra_roots(mut self, roots: Vec<String>) -> Self {
        self.extra_roots = roots;
        self
    }
}

#[async_trait]
impl ToolHandler for FileEditHandler {
    fn name(&self) -> &str {
        "file-edit"
    }

    fn description(&self) -> &str {
        "Surgically replace the first occurrence of old_string with new_string in a file. Returns an error if old_string is not found."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Absolute or ~-relative path to edit"},
                "old_string": {"type": "string", "description": "Exact text to find (returns error if not found)"},
                "new_string": {"type": "string", "description": "Replacement text"}
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn output_schema(&self) -> Option<serde_json::Value> {
        ToolRenderData::schema("code_diff")
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let path_str = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'path'")),
        };

        let old_string = match params.get("old_string").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'old_string'")),
        };

        let new_string = match params.get("new_string").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'new_string'")),
        };

        let path = match super::confine::confine(ctx, &self.extra_roots, &path_str) {
            Ok(p) => p,
            Err(refusal) => return Ok(refusal),
        };
        debug!(path = %path.display(), "file-edit access");

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to read '{}': {}",
                    path.display(),
                    e
                )));
            }
        };

        if !content.contains(old_string.as_str()) {
            return Ok(ToolOutput::error(format!(
                "'old_string' not found in '{}'. No changes made.",
                path.display()
            )));
        }

        let patched = content.replacen(old_string.as_str(), new_string.as_str(), 1);

        match fs::write(&path, patched.as_bytes()) {
            Ok(()) => Ok(ToolOutput::success(format!("Edited '{}'", path.display()))
                .with_render(code_diff(&path, DiffOperation::Edit, &content, &patched))),
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to write '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}
