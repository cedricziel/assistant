//! Builtin handler for file-write tool — writes content to any file on disk.

use std::collections::HashMap;
use std::fs;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;

use super::file_read::expand_tilde;

pub struct FileWriteHandler;

impl FileWriteHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileWriteHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for FileWriteHandler {
    fn name(&self) -> &str {
        "file-write"
    }

    fn description(&self) -> &str {
        "Write content to a file, creating it and parent directories if needed. Completely replaces existing content."
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
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let path_str = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'path'")),
        };

        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'content'")),
        };

        let path = expand_tilde(&path_str);

        // Create parent directories if needed.
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Ok(ToolOutput::error(format!(
                    "Failed to create directories for '{}': {}",
                    path.display(),
                    e
                )));
            }
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
