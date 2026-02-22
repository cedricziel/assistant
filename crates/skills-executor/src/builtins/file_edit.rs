//! Builtin handler for file-edit skill — surgically replaces text in a file.

use std::collections::HashMap;
use std::fs;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use async_trait::async_trait;

use super::file_read::expand_tilde;

pub struct FileEditHandler;

impl FileEditHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileEditHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillHandler for FileEditHandler {
    fn skill_name(&self) -> &str {
        "file-edit"
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

        let old_string = match params.get("old_string").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return Ok(SkillOutput::error(
                    "Missing required parameter 'old_string'",
                ))
            }
        };

        let new_string = match params.get("new_string").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return Ok(SkillOutput::error(
                    "Missing required parameter 'new_string'",
                ))
            }
        };

        let path = expand_tilde(&path_str);

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(SkillOutput::error(format!(
                    "Failed to read '{}': {}",
                    path.display(),
                    e
                )))
            }
        };

        if !content.contains(old_string.as_str()) {
            return Ok(SkillOutput::error(format!(
                "'old_string' not found in '{}'. No changes made.",
                path.display()
            )));
        }

        let patched = content.replacen(old_string.as_str(), new_string.as_str(), 1);

        match fs::write(&path, patched.as_bytes()) {
            Ok(()) => Ok(SkillOutput::success(format!("Edited '{}'", path.display()))),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to write '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}
