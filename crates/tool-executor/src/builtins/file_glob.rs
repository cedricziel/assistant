//! Builtin handler for file-glob tool — finds files matching a glob pattern.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;

const DEFAULT_LIMIT: usize = 200;

pub struct FileGlobHandler;

impl FileGlobHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileGlobHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for FileGlobHandler {
    fn name(&self) -> &str {
        "file-glob"
    }

    fn description(&self) -> &str {
        "Find files and directories matching a glob pattern. Returns a newline-separated list of matching paths."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Glob pattern, e.g. **/*.rs or ~/notes/*.md"},
                "limit": {"type": "number", "description": "Max results (default: 200)"}
            },
            "required": ["pattern"]
        })
    }

    fn output_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Matched file/directory paths"
                },
                "truncated": {
                    "type": "boolean",
                    "description": "True if results were cut off at the limit"
                }
            },
            "required": ["paths", "truncated"]
        }))
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let pattern_raw = match params.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'pattern'")),
        };

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_LIMIT);

        // Expand ~ in the pattern prefix (before any glob metacharacter).
        let pattern = if let Some(rest) = pattern_raw.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                format!("{}/{}", home.display(), rest)
            } else {
                pattern_raw
            }
        } else {
            pattern_raw
        };

        let entries = match glob::glob(&pattern) {
            Ok(paths) => paths,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Invalid glob pattern '{}': {}",
                    pattern, e
                )))
            }
        };

        let mut results: Vec<String> = Vec::new();
        let mut truncated = false;

        for entry in entries {
            if results.len() >= limit {
                truncated = true;
                break;
            }
            match entry {
                Ok(path) => results.push(path.display().to_string()),
                Err(e) => results.push(format!("[error reading entry: {}]", e)),
            }
        }

        if results.is_empty() {
            let data = serde_json::json!({"paths": [], "truncated": false});
            return Ok(
                ToolOutput::success(format!("No files matched pattern '{}'", pattern))
                    .with_data(data),
            );
        }

        let mut output = results.join("\n");
        if truncated {
            output.push_str(&format!("\n\n[Results truncated at {} entries]", limit));
        }

        let data = serde_json::json!({"paths": results, "truncated": truncated});
        Ok(ToolOutput::success(output).with_data(data))
    }
}
