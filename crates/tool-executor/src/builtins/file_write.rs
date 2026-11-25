//! Builtin handler for file-write tool — writes content to a file inside the workspace roots.

use std::collections::HashMap;
use std::fs;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use tracing::debug;

#[derive(Default)]
pub struct FileWriteHandler {
    extra_roots: Vec<String>,
}

impl FileWriteHandler {
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
impl ToolHandler for FileWriteHandler {
    fn name(&self) -> &str {
        "file-write"
    }

    fn description(&self) -> &str {
        "Write content to a file inside the workspace, creating it and parent directories if needed. Completely replaces existing content."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Absolute or ~-relative path to write"},
                "content": {"type": "string", "description": "Content to write"}
            },
            "required": ["path", "content"]
        })
    }

    fn is_mutating(&self) -> bool {
        true
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

        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'content'")),
        };

        let path = match super::confine::confine(ctx, &self.extra_roots, &path_str) {
            Ok(p) => p,
            Err(refusal) => return Ok(refusal),
        };
        debug!(path = %path.display(), "file-write access");

        // Create parent directories if needed.
        if let Some(parent) = path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            return Ok(ToolOutput::error(format!(
                "Failed to create directories for '{}': {}",
                path.display(),
                e
            )));
        }

        match fs::write(&path, content.as_bytes()) {
            Ok(()) => Ok(ToolOutput::success(format!(
                "Written {} bytes to '{}'",
                content.len(),
                path.display()
            ))),
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to write '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}
