//! Builtin handler for file-glob skill — finds files matching a glob pattern.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use async_trait::async_trait;

const DEFAULT_LIMIT: usize = 200;

pub struct FileGlobHandler;

impl FileGlobHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileGlobHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillHandler for FileGlobHandler {
    fn skill_name(&self) -> &str {
        "file-glob"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let pattern_raw = match params.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'pattern'")),
        };

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_LIMIT);

        // Expand ~ in the pattern prefix (before any glob metacharacter).
        let pattern = if let Some(rest) = pattern_raw.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                format!("{}/{}", home.display(), rest)
            } else {
                pattern_raw
            }
        } else {
            pattern_raw
        };

        let entries = match glob::glob(&pattern) {
            Ok(paths) => paths,
            Err(e) => {
                return Ok(SkillOutput::error(format!(
                    "Invalid glob pattern '{}': {}",
                    pattern, e
                )))
            }
        };

        let mut results: Vec<String> = Vec::new();
        let mut truncated = false;

        for entry in entries {
            if results.len() >= limit {
                truncated = true;
                break;
            }
            match entry {
                Ok(path) => results.push(path.display().to_string()),
                Err(e) => results.push(format!("[error reading entry: {}]", e)),
            }
        }

        if results.is_empty() {
            return Ok(SkillOutput::success(format!(
                "No files matched pattern '{}'",
                pattern
            )));
        }

        let mut output = results.join("\n");
        if truncated {
            output.push_str(&format!("\n\n[Results truncated at {} entries]", limit));
        }

        Ok(SkillOutput::success(output))
    }
}
