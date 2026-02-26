//! Builtin handler for the `memory-append` tool.
//!
//! Appends a timestamped entry to a persistent memory file without requiring
//! a full read-edit-write cycle.  Particularly useful for daily notes.
//! Accepted targets: `soul`, `identity`, `user`, `memory`, `notes/YYYY-MM-DD`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use assistant_core::{
    base_dir, resolve_dir, resolve_path, AssistantConfig, ExecutionContext, ToolHandler, ToolOutput,
};

/// Canonicalize as much of `p` as exists, then re-append non-existent tail
/// components.  This allows security checks to work even when parent
/// directories have not been created yet (cold-start scenario).
fn canonicalize_prefix(p: &std::path::Path) -> PathBuf {
    let mut components = vec![];
    let mut cur = p.to_path_buf();
    loop {
        if let Ok(c) = cur.canonicalize() {
            let mut result = c;
            for comp in components.into_iter().rev() {
                result = result.join(comp);
            }
            return result;
        }
        match (
            cur.parent().map(|p| p.to_path_buf()),
            cur.file_name().map(|n| n.to_owned()),
        ) {
            (Some(parent), Some(name)) => {
                components.push(name);
                cur = parent;
            }
            _ => return p.to_path_buf(),
        }
    }
}

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
                // Enforce YYYY-MM-DD: positions 4 and 7 must be '-', rest digits.
                let valid_format = date.len() == 10
                    && date.chars().enumerate().all(|(i, c)| {
                        if i == 4 || i == 7 {
                            c == '-'
                        } else {
                            c.is_ascii_digit()
                        }
                    });
                if !valid_format {
                    return None;
                }
                let notes_dir = resolve_dir(&mem.notes_dir, &base, "memory");
                notes_dir.join(format!("{date}.md"))
            }
            _ => return None,
        };

        // Security: verify the resolved path stays within ~/.assistant/.
        let canonical_base = canonicalize_prefix(&base);
        let canonical_path = canonicalize_prefix(&path);
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
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return Ok(ToolOutput::error(format!(
                    "Failed to create directories for '{}': {}",
                    path.display(),
                    e
                )));
            }
        }

        let mut file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
        {
            Ok(f) => f,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to open '{}': {}",
                    path.display(),
                    e
                )))
            }
        };

        // Ensure entries start on their own line.
        let content = format!("\n{}", text);
        if let Err(e) = file.write_all(content.as_bytes()).await {
            return Ok(ToolOutput::error(format!(
                "Failed to write to '{}': {}",
                path.display(),
                e
            )));
        }
        if let Err(e) = file.flush().await {
            return Ok(ToolOutput::error(format!(
                "Failed to flush '{}': {}",
                path.display(),
                e
            )));
        }

        Ok(ToolOutput::success(format!(
            "Appended {} bytes to {}",
            content.len(),
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_prefix_returns_input_when_nothing_exists() {
        let p = PathBuf::from("/nonexistent/deeply/nested/path");
        let result = canonicalize_prefix(&p);
        // Should return the original path when nothing can be canonicalized.
        assert_eq!(result, p);
    }

    #[test]
    fn canonicalize_prefix_resolves_existing_ancestor() {
        // /tmp always exists on macOS/Linux.
        let p = PathBuf::from("/tmp/nonexistent_child/file.md");
        let result = canonicalize_prefix(&p);
        // The /tmp portion should be canonicalized, with the tail re-appended.
        let canonical_tmp = PathBuf::from("/tmp")
            .canonicalize()
            .expect("/tmp must exist");
        assert!(result.starts_with(&canonical_tmp));
        assert!(result.ends_with("nonexistent_child/file.md"));
    }

    #[test]
    fn date_validation_rejects_all_dashes() {
        // "----------" is 10 chars of dashes — must be rejected.
        let handler = MemoryAppendHandler::new(Arc::new(AssistantConfig::default()));
        let result = handler.resolve_target("notes/----------");
        assert!(result.is_none(), "all-dash string should be rejected");
    }

    #[test]
    fn date_validation_rejects_bad_structure() {
        let handler = MemoryAppendHandler::new(Arc::new(AssistantConfig::default()));
        // Digits in wrong positions.
        assert!(
            handler.resolve_target("notes/99-99-9999").is_none(),
            "99-99-9999 should be rejected"
        );
        // Too short.
        assert!(
            handler.resolve_target("notes/2026-1-1").is_none(),
            "short date should be rejected"
        );
    }

    #[test]
    fn date_validation_accepts_valid_date() {
        let handler = MemoryAppendHandler::new(Arc::new(AssistantConfig::default()));
        let result = handler.resolve_target("notes/2026-02-26");
        assert!(result.is_some(), "valid ISO date should be accepted");
    }

    #[test]
    fn unknown_target_returns_none() {
        let handler = MemoryAppendHandler::new(Arc::new(AssistantConfig::default()));
        assert!(handler.resolve_target("bogus").is_none());
        assert!(handler.resolve_target("").is_none());
        assert!(handler.resolve_target("notes/").is_none());
    }

    fn test_ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: uuid::Uuid::nil(),
            turn: 0,
            interface: assistant_core::Interface::Cli,
            interactive: false,
        }
    }

    #[tokio::test]
    async fn run_missing_params_returns_tool_error() {
        let handler = MemoryAppendHandler::new(Arc::new(AssistantConfig::default()));
        let ctx = test_ctx();

        // Missing both params.
        let out = handler.run(HashMap::new(), &ctx).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("target"));

        // Missing text.
        let mut params = HashMap::new();
        params.insert(
            "target".to_string(),
            serde_json::Value::String("soul".to_string()),
        );
        let out = handler.run(params, &ctx).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("text"));
    }

    #[tokio::test]
    async fn run_unknown_target_returns_tool_error() {
        let handler = MemoryAppendHandler::new(Arc::new(AssistantConfig::default()));
        let ctx = test_ctx();
        let mut params = HashMap::new();
        params.insert(
            "target".to_string(),
            serde_json::Value::String("bogus".to_string()),
        );
        params.insert(
            "text".to_string(),
            serde_json::Value::String("hello".to_string()),
        );
        let out = handler.run(params, &ctx).await.unwrap();
        assert!(!out.success, "unknown target should yield error ToolOutput");
        assert!(out.content.contains("Unknown target"));
    }
}
