//! Builtin handler for file-read tool — reads any file from disk.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;

const DEFAULT_LIMIT: usize = 8_000;

pub struct FileReadHandler;

impl FileReadHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileReadHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for FileReadHandler {
    fn name(&self) -> &str {
        "file-read"
    }

    fn description(&self) -> &str {
        "Read the contents of any file from disk. Returns text content, optionally truncated to a character limit."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Absolute or ~-relative path to the file"},
                "limit": {"type": "number", "description": "Max chars to return (default: 8000)"}
            },
            "required": ["path"]
        })
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

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_LIMIT);

        let path = expand_tilde(&path_str);

        match fs::read_to_string(&path) {
            Ok(content) => {
                let text = content.trim_end();
                let output = if text.len() > limit {
                    let mut end = limit;
                    while end > 0 && !text.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!(
                        "File: {}\n\n{}\n\n[Content truncated at {} characters]",
                        path.display(),
                        &text[..end],
                        limit
                    )
                } else {
                    format!("File: {}\n\n{}", path.display(), text)
                };
                Ok(ToolOutput::success(output))
            }
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to read '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}

pub(crate) fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}
