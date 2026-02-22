//! Builtin handler for file-write skill — writes content to any file on disk.

use std::collections::HashMap;
use std::fs;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use async_trait::async_trait;

use super::file_read::expand_tilde;

pub struct FileWriteHandler;

impl FileWriteHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileWriteHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillHandler for FileWriteHandler {
    fn skill_name(&self) -> &str {
        "file-write"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let path_str = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'path'")),
        };

        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'content'")),
        };

        let path = expand_tilde(&path_str);

        // Create parent directories if needed.
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Ok(SkillOutput::error(format!(
                    "Failed to create directories for '{}': {}",
                    path.display(),
                    e
                )));
            }
        }

        match fs::write(&path, content.as_bytes()) {
            Ok(()) => Ok(SkillOutput::success(format!(
                "Written {} bytes to '{}'",
                content.len(),
                path.display()
            ))),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to write '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}
