//! Builtin handler for file-read skill — reads any file from disk.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use async_trait::async_trait;

const DEFAULT_LIMIT: usize = 8_000;

pub struct FileReadHandler;

impl FileReadHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileReadHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillHandler for FileReadHandler {
    fn skill_name(&self) -> &str {
        "file-read"
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

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_LIMIT);

        let path = expand_tilde(&path_str);

        match fs::read_to_string(&path) {
            Ok(content) => {
                let text = content.trim_end();
                let output = if text.len() > limit {
                    let mut end = limit;
                    while end > 0 && !text.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!(
                        "File: {}\n\n{}\n\n[Content truncated at {} characters]",
                        path.display(),
                        &text[..end],
                        limit
                    )
                } else {
                    format!("File: {}\n\n{}", path.display(), text)
                };
                Ok(SkillOutput::success(output))
            }
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to read '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}

pub(crate) fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}
