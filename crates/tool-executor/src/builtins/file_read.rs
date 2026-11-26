//! Builtin handler for file-read tool — reads a file inside the workspace roots.

use anyhow::Result;
use assistant_core::tool_render::{CodeFileData, ToolRenderData};
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::io::AsyncReadExt;
use tracing::debug;

use super::render_lang::language_for_path;

const DEFAULT_LIMIT: usize = 8_000;

/// Render budget for the `code_file` view (design D4): `content` has at most
/// this many bytes, independent of the LLM-facing `limit`.
const MAX_RENDER_CONTENT_BYTES: usize = 24 * 1024;

#[derive(Default)]
pub struct FileReadHandler {
    extra_roots: Vec<String>,
}

impl FileReadHandler {
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
impl ToolHandler for FileReadHandler {
    fn name(&self) -> &str {
        "file-read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file inside the workspace. Returns text content, optionally truncated to a character limit."
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

    fn output_schema(&self) -> Option<serde_json::Value> {
        ToolRenderData::schema("code_file")
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

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_LIMIT);

        let path = match super::confine::confine(ctx, &self.extra_roots, &path_str) {
            Ok(p) => p,
            Err(refusal) => return Ok(refusal),
        };
        debug!(path = %path.display(), "file-read access");

        // Read asynchronously, capping at limit bytes + 1 to detect truncation.
        let file = match tokio::fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to read '{}': {}",
                    path.display(),
                    e
                )));
            }
        };

        let mut buf = Vec::new();
        let mut file = file;
        if let Err(e) = (&mut file)
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
        let (shown, cut_by_limit) = if char_count > limit {
            // Find the byte offset corresponding to `limit` chars.
            let end = text
                .char_indices()
                .nth(limit)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
            (&text[..end], true)
        } else {
            (text, false)
        };
        let output = if cut_by_limit {
            format!(
                "File: {}\n\n{}\n\n[Content truncated at {} characters]",
                path.display(),
                shown,
                limit
            )
        } else {
            format!("File: {}\n\n{}", path.display(), shown)
        };

        let render_content = cut_to_bytes(shown, MAX_RENDER_CONTENT_BYTES);
        // The read pass keeps a prefix; count the rest so `total_chars` is the
        // whole file, trailing whitespace included.
        let unread_chars = count_remaining_chars(&mut file).await;
        let total_chars = content.chars().count() + unread_chars;
        let truncated = cut_by_limit || unread_chars > 0 || render_content.len() < shown.len();
        let render = ToolRenderData::CodeFile(CodeFileData {
            path: path.display().to_string(),
            language: language_for_path(&path).map(str::to_string),
            content: render_content.to_string(),
            truncated,
            total_chars: total_chars as u64,
        });
        Ok(ToolOutput::success(output).with_render(render))
    }
}

/// The longest prefix of `text` with at most `max_bytes` bytes, cut on a
/// char boundary.
fn cut_to_bytes(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Count the characters in the unread rest of `file` (UTF-8 lead bytes), as
/// the read pass only kept a prefix. A read error ends the count early.
async fn count_remaining_chars(file: &mut tokio::fs::File) -> usize {
    let mut chunk = vec![0u8; 64 * 1024];
    let mut count = 0;
    loop {
        match file.read(&mut chunk).await {
            Ok(0) | Err(_) => return count,
            Ok(n) => count += chunk[..n].iter().filter(|b| (**b & 0xC0) != 0x80).count(),
        }
    }
}
