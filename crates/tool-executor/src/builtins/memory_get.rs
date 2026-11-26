//! Builtin handler for the `memory-get` tool.
//!
//! Reads one of the persistent memory files and returns its content.
//! Accepted targets: `soul`, `identity`, `user`, `tools`, `memory`,
//! `notes/YYYY-MM-DD`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    ToolHandler, ToolOutput, base_dir, resolve, types::agent::OrgConfig,
    types::conversation::ExecutionContext,
};
use async_trait::async_trait;

pub struct MemoryGetHandler {
    config: Arc<OrgConfig>,
}

impl MemoryGetHandler {
    pub fn new(config: Arc<OrgConfig>) -> Self {
        Self { config }
    }

    /// Map a target name to an absolute path within the default agent memory root.
    fn resolve_target(&self, target: &str) -> Option<PathBuf> {
        let mem = &self.config.memory;
        let base = base_dir();
        let root = base.as_deref();

        let path = match target {
            "soul" => resolve(&mem.soul_path, root, "SOUL.md")?,
            "identity" => resolve(&mem.identity_path, root, "IDENTITY.md")?,
            "user" => resolve(&mem.user_path, root, "USER.md")?,
            "tools" => resolve(&mem.tools_path, root, "TOOLS.md")?,
            "memory" => resolve(&mem.memory_path, root, "MEMORY.md")?,
            notes if notes.starts_with("notes/") => {
                let date = &notes["notes/".len()..];
                if date.len() != 10 || !date.chars().all(|c| c.is_ascii_digit() || c == '-') {
                    return None;
                }
                let notes_dir = resolve(&mem.notes_dir, root, "memory")?;
                notes_dir.join(format!("{date}.md"))
            }
            _ => return None,
        };

        // Security: verify the resolved path stays within the runtime memory root.
        if let Some(base) = &base {
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
        "Read the contents of a persistent memory file. Supported targets: soul, identity, user, tools, memory, notes/YYYY-MM-DD."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Memory target: soul, identity, user, tools, memory, or notes/YYYY-MM-DD"
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
                    "Unknown target '{target}'. Use: soul, identity, user, tools, memory, or notes/YYYY-MM-DD"
                )));
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
