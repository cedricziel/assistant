//! Builtin handler for the `memory-patch` skill.
//!
//! Performs a surgical search-and-replace on one of the four persistent memory
//! files (SOUL.md, IDENTITY.md, USER.md, MEMORY.md).  Unlike `soul-update`,
//! which supports full-append or full-replace, this replaces only the *first*
//! occurrence of `search` with `replace`.  If the search string is not found,
//! an error is returned without modifying the file (no silent corruption).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{AssistantConfig, ExecutionContext, MemoryLoader, SkillDef, SkillHandler, SkillOutput};
use async_trait::async_trait;

pub struct MemoryPatchHandler {
    config: Arc<AssistantConfig>,
}

impl MemoryPatchHandler {
    pub fn new(config: Arc<AssistantConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SkillHandler for MemoryPatchHandler {
    fn skill_name(&self) -> &str {
        "memory-patch"
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
        let search = match params.get("search").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'search'")),
        };
        let replace = match params.get("replace").and_then(|v| v.as_str()) {
            Some(r) => r.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'replace'")),
        };

        let loader = MemoryLoader::new(&self.config);
        match loader.patch_file(&target, &search, &replace) {
            Ok(path) => Ok(SkillOutput::success(format!(
                "Patched {} ({})",
                target,
                path.display()
            ))),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to patch {target}: {e}"
            ))),
        }
    }
}
