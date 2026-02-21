//! Builtin handler for skill-file-read — reads auxiliary files from a skill directory.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use assistant_storage::SkillRegistry;
use async_trait::async_trait;

pub struct SkillFileReadHandler {
    registry: Arc<SkillRegistry>,
}

impl SkillFileReadHandler {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl SkillHandler for SkillFileReadHandler {
    fn skill_name(&self) -> &str {
        "skill-file-read"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let skill_name = match params.get("skill").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return Ok(SkillOutput::error("Missing required parameter: skill")),
        };

        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return Ok(SkillOutput::error("Missing required parameter: path")),
        };

        // Reject paths with ".." components to prevent directory traversal
        if Path::new(path)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Ok(SkillOutput::error(
                "Invalid path: directory traversal (..) is not allowed",
            ));
        }

        // Look up the skill
        let skill = match self.registry.get(skill_name).await {
            Some(s) => s,
            None => {
                return Ok(SkillOutput::error(format!("Skill not found: {skill_name}")));
            }
        };

        let file_path = skill.dir.join(path);

        // Verify the resolved path is still within the skill directory
        let canonical_dir = match skill.dir.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                return Ok(SkillOutput::error(format!(
                    "Skill directory not accessible: {}",
                    skill.dir.display()
                )));
            }
        };
        let canonical_file = match file_path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                return Ok(SkillOutput::error(format!(
                    "File not found: {path} in skill {skill_name}"
                )));
            }
        };
        if !canonical_file.starts_with(&canonical_dir) {
            return Ok(SkillOutput::error(
                "Invalid path: resolved path is outside the skill directory",
            ));
        }

        // Read the file
        match tokio::fs::read_to_string(&canonical_file).await {
            Ok(contents) => Ok(SkillOutput::success(contents)),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to read {path} in skill {skill_name}: {e}"
            ))),
        }
    }
}
