//! Builtin handler for the `memory-append` tool.
//!
//! Appends a timestamped entry to a persistent memory file without requiring
//! a full read-edit-write cycle.  Particularly useful for daily notes.
//! Accepted targets: `soul`, `identity`, `user`, `memory`, `notes/YYYY-MM-DD`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    base_dir, resolve_dir, resolve_path, AssistantConfig, ExecutionContext, ToolHandler, ToolOutput,
};
use async_trait::async_trait;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub struct MemoryAppendHandler {
    config: Arc<AssistantConfig>,
}

impl MemoryAppendHandler {
    pub fn new(config: Arc<AssistantConfig>) -> Self {
        Self { config }
    }

    /// Map a target name to an absolute path within `~/.assistant/`.
    fn resolve_target(&self, target: &str) -> Option<PathBuf> {
        let mem = &self.config.memory;
        let base = base_dir();

        let path = match target {
            "soul" => resolve_path(&mem.soul_path, &base, "SOUL.md"),
            "identity" => resolve_path(&mem.identity_path, &base, "IDENTITY.md"),
            "user" => resolve_path(&mem.user_path, &base, "USER.md"),
            "memory" => resolve_path(&mem.memory_path, &base, "MEMORY.md"),
            notes if notes.starts_with("notes/") => {
                let date = &notes["notes/".len()..];
                if date.len() != 10 || !date.chars().all(|c| c.is_ascii_digit() || c == '-') {
                    return None;
                }
                let notes_dir = resolve_dir(&mem.notes_dir, &base, "memory");
                notes_dir.join(format!("{date}.md"))
            }
            _ => return None,
        };

        // Security: verify the resolved path stays within ~/.assistant/.
        let canonical_base = base.canonicalize().unwrap_or(base.clone());
        let canonical_path = if path.exists() {
            path.canonicalize().ok()?
        } else if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
            parent.canonicalize().ok()?.join(name)
        } else {
            return None;
        };
        if !canonical_path.starts_with(&canonical_base) {
            return None;
        }

        Some(path)
    }
}

#[async_trait]
impl ToolHandler for MemoryAppendHandler {
    fn name(&self) -> &str {
        "memory-append"
    }

    fn description(&self) -> &str {
        "Append text to a persistent memory file without a full read-write cycle. \
         Ideal for adding timestamped entries to daily notes. \
         A newline is prepended automatically so entries don't run together. \
         Accepted targets: soul, identity, user, memory, notes/YYYY-MM-DD."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Memory target: soul, identity, user, memory, or notes/YYYY-MM-DD"
                },
                "text": {
                    "type": "string",
                    "description": "Text to append to the memory file"
                }
            },
            "required": ["target", "text"]
        })
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let target = match params.get("target").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'target'")),
        };
        let text = match params.get("text").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'text'")),
        };

        let path = match self.resolve_target(&target) {
            Some(p) => p,
            None => {
                return Ok(ToolOutput::error(format!(
                "Unknown target '{target}'. Use: soul, identity, user, memory, or notes/YYYY-MM-DD"
            )))
            }
        };

        // Create parent directories if they don't exist yet (e.g. for a new notes/ day).
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        // Ensure entries start on their own line.
        let content = format!("\n{}", text);
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;

        Ok(ToolOutput::success(format!(
            "Appended {} bytes to {}",
            text.len(),
            path.display()
        )))
    }
}
