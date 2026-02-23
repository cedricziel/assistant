//! Builtin handler for the `memory-get` skill.
//!
//! Reads one of the persistent memory files and returns its content.
//! Accepted targets: `soul`, `identity`, `user`, `memory`, `notes/YYYY-MM-DD`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    base_dir, resolve_dir, resolve_path, AssistantConfig, ExecutionContext, SkillDef, SkillHandler,
    SkillOutput,
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
                // Reject anything that isn't a strict YYYY-MM-DD date (exactly
                // 10 chars: 4 digits, dash, 2 digits, dash, 2 digits) to prevent
                // path traversal and empty/overlong inputs.
                if date.len() != 10 || !date.chars().all(|c| c.is_ascii_digit() || c == '-') {
                    return None;
                }
                let notes_dir = resolve_dir(&mem.notes_dir, &base, "memory");
                notes_dir.join(format!("{date}.md"))
            }
            _ => return None,
        };

        // Security: verify the resolved path stays within ~/.assistant/.
        // For existing files, use the fully-resolved canonical path.
        // For not-yet-created files, canonicalize the parent directory and
        // join the filename — this avoids falling back to a raw starts_with
        // check that can be bypassed by unresolved `..` segments.
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
impl SkillHandler for MemoryGetHandler {
    fn skill_name(&self) -> &str {
        "memory-get"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let target = match params.get("target").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'target'")),
        };

        let path = match self.resolve_target(&target) {
            Some(p) => p,
            None => {
                return Ok(SkillOutput::error(format!(
                "Unknown target '{target}'. Use: soul, identity, user, memory, or notes/YYYY-MM-DD"
            )))
            }
        };

        match tokio::fs::read_to_string(&path).await {
            Ok(content) => Ok(SkillOutput::success(format!(
                "File: {}\n\n{}",
                path.display(),
                content.trim_end()
            ))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SkillOutput::error(format!(
                "Memory file not found: {} ({})",
                target,
                path.display()
            ))),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to read '{}': {e}",
                path.display()
            ))),
        }
    }
}
