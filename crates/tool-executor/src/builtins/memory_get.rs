//! Builtin handler for the `memory-get` tool.
//!
//! Reads one of the persistent memory files and returns its content.
//! Accepted targets: `soul`, `identity`, `user`, `memory`, `notes/YYYY-MM-DD`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    base_dir, resolve_dir, resolve_path, AssistantConfig, ExecutionContext, ToolHandler, ToolOutput,
};
use async_trait::async_trait;

pub struct MemoryGetHandler {
    config: Arc<AssistantConfig>,
}

impl MemoryGetHandler {
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
impl ToolHandler for MemoryGetHandler {
    fn name(&self) -> &str {
        "memory-get"
    }

    fn description(&self) -> &str {
        "Read the contents of a persistent memory file. Supported targets: soul, identity, user, memory, notes/YYYY-MM-DD."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Memory target: soul, identity, user, memory, or notes/YYYY-MM-DD"
                }
            },
            "required": ["target"]
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

        let path = match self.resolve_target(&target) {
            Some(p) => p,
            None => {
                return Ok(ToolOutput::error(format!(
                "Unknown target '{target}'. Use: soul, identity, user, memory, or notes/YYYY-MM-DD"
            )))
            }
        };

        match tokio::fs::read_to_string(&path).await {
            Ok(content) => Ok(ToolOutput::success(format!(
                "File: {}\n\n{}",
                path.display(),
                content.trim_end()
            ))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ToolOutput::error(format!(
                "Memory file not found: {} ({})",
                target,
                path.display()
            ))),
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to read '{}': {e}",
                path.display()
            ))),
        }
    }
}
