//! Builtin handler for file-glob tool — finds files matching a glob pattern.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use assistant_core::permission::{WorkspaceRoots, glob_literal_dir, resolve_real_path};
use assistant_core::{
    ToolHandler, ToolOutput, expand_tilde, types::conversation::ExecutionContext,
};
use async_trait::async_trait;

const DEFAULT_LIMIT: usize = 200;

#[derive(Default)]
pub struct FileGlobHandler {
    extra_roots: Vec<String>,
}

impl FileGlobHandler {
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
impl ToolHandler for FileGlobHandler {
    fn name(&self) -> &str {
        "file-glob"
    }

    fn description(&self) -> &str {
        "Find files and directories inside the workspace matching a glob pattern. Returns a newline-separated list of matching paths."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Glob pattern, e.g. **/*.rs or ~/notes/*.md"},
                "limit": {"type": "integer", "minimum": 1, "description": "Max results (default: 200)"}
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
        ctx: &ExecutionContext,
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

        // The literal prefix (before any glob metacharacter) must lie inside the
        // workspace roots; matches are re-checked below.
        let roots = super::confine::roots(ctx, &self.extra_roots);
        let prefix = glob_literal_dir(&pattern_raw);
        if let Err(outside) = roots.confine(&prefix) {
            return Ok(ToolOutput::error(format!(
                "{outside}; file tools are confined to the workspace roots"
            )));
        }
        let pattern = expand_tilde(&pattern_raw).to_string_lossy().into_owned();

        let entries = match glob::glob(&pattern) {
            Ok(paths) => paths,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Invalid glob pattern '{}': {}",
                    pattern, e
                )));
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
                Ok(path) => {
                    if is_inside(&roots, &path) {
                        results.push(path.display().to_string());
                    }
                }
                Err(e) if is_inside(&roots, e.path()) => {
                    results.push(format!("[error reading entry: {}]", e))
                }
                Err(_) => results.push("[error reading entry outside the workspace]".to_string()),
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

fn is_inside(roots: &WorkspaceRoots, path: &Path) -> bool {
    resolve_real_path(&path.to_string_lossy(), None)
        .map(|real| roots.contains(&real))
        .unwrap_or(false)
}
