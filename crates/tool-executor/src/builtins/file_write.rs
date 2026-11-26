//! Builtin handler for file-write tool — writes content to a file inside the workspace roots.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use assistant_core::tool_render::{DiffOperation, ToolRenderData};
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use tracing::debug;

use super::render_diff::code_diff;

/// Prior content larger than this is not diffed (design D6).
const MAX_PRIOR_BYTES: u64 = 256 * 1024;

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

        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'content'")),
        };

        let path = match super::confine::confine(ctx, &self.extra_roots, &path_str) {
            Ok(p) => p,
            Err(refusal) => return Ok(refusal),
        };
        debug!(path = %path.display(), "file-write access");

        let prior = read_prior(&path);

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
            Ok(()) => {
                let output = ToolOutput::success(format!(
                    "Written {} bytes to '{}'",
                    content.len(),
                    path.display()
                ));
                Ok(match prior {
                    Prior::Missing => {
                        output.with_render(code_diff(&path, DiffOperation::Create, "", &content))
                    }
                    Prior::Text(old) => output.with_render(code_diff(
                        &path,
                        DiffOperation::Overwrite,
                        &old,
                        &content,
                    )),
                    Prior::Unreadable => output,
                })
            }
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to write '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}

/// What `file-write` found at the path before writing.
enum Prior {
    /// No file there: the write creates it.
    Missing,
    /// UTF-8 content of at most [`MAX_PRIOR_BYTES`].
    Text(String),
    /// Too large, not UTF-8 or not readable: no diff is published.
    Unreadable,
}

fn read_prior(path: &Path) -> Prior {
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Prior::Missing,
        Err(_) => return Prior::Unreadable,
    };
    if !meta.is_file() || meta.len() > MAX_PRIOR_BYTES {
        return Prior::Unreadable;
    }
    match fs::read_to_string(path) {
        Ok(text) => Prior::Text(text),
        Err(_) => Prior::Unreadable,
    }
}
