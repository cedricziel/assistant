//! Builtin handlers for memory-save and memory-update skills.
//!
//! `memory-save` appends a timestamped note to today's daily log file
//! (`~/.assistant/memory/YYYY-MM-DD.md`).
//!
//! `memory-update` updates one of the four persistent markdown identity files:
//! SOUL.md, IDENTITY.md, USER.md, or MEMORY.md.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    AssistantConfig, ExecutionContext, MemoryLoader, SkillDef, SkillHandler, SkillOutput,
};
use async_trait::async_trait;

// ---------------------------------------------------------------------------
// MemorySaveHandler
// ---------------------------------------------------------------------------

pub struct MemorySaveHandler {
    config: Arc<AssistantConfig>,
}

impl MemorySaveHandler {
    pub fn new(config: Arc<AssistantConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SkillHandler for MemorySaveHandler {
    fn skill_name(&self) -> &str {
        "memory-save"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let note = match params.get("note").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'note'")),
        };
        let category = params
            .get("category")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let loader = MemoryLoader::new(&self.config);
        match loader.append_daily_note(category.as_deref(), &note) {
            Ok(()) => {
                let path = loader.daily_notes_path();
                Ok(SkillOutput::success(format!(
                    "Note saved to {}",
                    path.display()
                )))
            }
            Err(e) => Ok(SkillOutput::error(format!("Failed to save note: {e}"))),
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryUpdateHandler
// ---------------------------------------------------------------------------

pub struct MemoryUpdateHandler {
    config: Arc<AssistantConfig>,
}

impl MemoryUpdateHandler {
    pub fn new(config: Arc<AssistantConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SkillHandler for MemoryUpdateHandler {
    fn skill_name(&self) -> &str {
        "memory-update"
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
        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'content'")),
        };
        let mode = params
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("append");

        let loader = MemoryLoader::new(&self.config);
        match loader.update_file(&target, &content, mode) {
            Ok(path) => Ok(SkillOutput::success(format!(
                "Updated {} ({})",
                target,
                path.display()
            ))),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to update {target}: {e}"
            ))),
        }
    }
}
