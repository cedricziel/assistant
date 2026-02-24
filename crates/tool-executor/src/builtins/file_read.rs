//! Builtin handler for file-read tool — reads any file from disk.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use tokio::io::AsyncReadExt;
use tracing::debug;

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
                "limit": {"type": "integer", "minimum": 1, "description": "Max chars to return (default: 8000)"}
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
            .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_LIMIT);

        let path = expand_tilde(&path_str);
        debug!(path = %path.display(), "file-read access");

        // Read asynchronously, capping at limit bytes + 1 to detect truncation.
        let file = match tokio::fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to read '{}': {}",
                    path.display(),
                    e
                )))
            }
        };

        let mut buf = Vec::new();
        if let Err(e) = file
            .take((limit as u64).saturating_add(1024))
            .read_to_end(&mut buf)
            .await
        {
            return Ok(ToolOutput::error(format!(
                "Failed to read '{}': {}",
                path.display(),
                e
            )));
        }

        let content = String::from_utf8_lossy(&buf);
        let text = content.trim_end();

        // Count characters (not bytes) to honour the `limit` promise.
        let char_count = text.chars().count();
        let output = if char_count > limit {
            // Find the byte offset corresponding to `limit` chars.
            let end = text
                .char_indices()
                .nth(limit)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
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
}

pub(crate) fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}
