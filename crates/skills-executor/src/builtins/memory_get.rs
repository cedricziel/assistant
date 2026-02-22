//! Builtin handler for the `memory-get` skill.
//!
//! Reads one of the persistent memory files and returns its content.
//! Accepted targets: `soul`, `identity`, `user`, `memory`, `notes/YYYY-MM-DD`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{AssistantConfig, ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use async_trait::async_trait;

use crate::builtins::file_read::expand_tilde;

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
                let notes_dir = resolve_dir(&mem.notes_dir, &base, "memory");
                notes_dir.join(format!("{date}.md"))
            }
            _ => return None,
        };

        // Security: verify the resolved path stays within ~/.assistant/.
        let canonical_base = base.canonicalize().unwrap_or(base.clone());
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !canonical_path.starts_with(&canonical_base) && !path.starts_with(&base) {
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

        match fs::read_to_string(&path) {
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

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".assistant"))
        .unwrap_or_else(|| PathBuf::from(".assistant"))
}

fn resolve_path(opt: &Option<String>, base: &std::path::Path, filename: &str) -> PathBuf {
    match opt {
        Some(p) => expand_tilde(p),
        None => base.join(filename),
    }
}

fn resolve_dir(opt: &Option<String>, base: &std::path::Path, dirname: &str) -> PathBuf {
    match opt {
        Some(p) => expand_tilde(p),
        None => base.join(dirname),
    }
}
