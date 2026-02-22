//! Builtin handlers for `heartbeat-read` and `heartbeat-update` skills.
//!
//! `~/.assistant/HEARTBEAT.md` is a plain Markdown file that the model edits to
//! specify what the assistant should check or do every 30 minutes automatically.
//! The scheduler reads it on every heartbeat tick and runs its contents as a
//! ReAct prompt.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{AssistantConfig, ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use async_trait::async_trait;

fn heartbeat_path(config: &AssistantConfig) -> std::path::PathBuf {
    config
        .storage
        .db_path
        .as_ref()
        .and_then(|p| {
            std::path::Path::new(p)
                .parent()
                .map(|d| d.join("HEARTBEAT.md"))
        })
        .or_else(|| dirs::home_dir().map(|h| h.join(".assistant").join("HEARTBEAT.md")))
        .unwrap_or_else(|| std::path::PathBuf::from(".assistant/HEARTBEAT.md"))
}

// ── heartbeat-read ────────────────────────────────────────────────────────────

pub struct HeartbeatReadHandler {
    config: Arc<AssistantConfig>,
}

impl HeartbeatReadHandler {
    pub fn new(config: Arc<AssistantConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SkillHandler for HeartbeatReadHandler {
    fn skill_name(&self) -> &str {
        "heartbeat-read"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        _params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let path = heartbeat_path(&self.config);

        if !path.exists() {
            return Ok(SkillOutput::success(format!(
                "HEARTBEAT.md does not exist yet ({}). Use heartbeat-update to create it.",
                path.display()
            )));
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => Ok(SkillOutput::success(format!(
                "# HEARTBEAT.md ({})\n\n{}",
                path.display(),
                content
            ))),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to read HEARTBEAT.md: {e}"
            ))),
        }
    }
}

// ── heartbeat-update ──────────────────────────────────────────────────────────

pub struct HeartbeatUpdateHandler {
    config: Arc<AssistantConfig>,
}

impl HeartbeatUpdateHandler {
    pub fn new(config: Arc<AssistantConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SkillHandler for HeartbeatUpdateHandler {
    fn skill_name(&self) -> &str {
        "heartbeat-update"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'content'")),
        };

        let path = heartbeat_path(&self.config);

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Ok(SkillOutput::error(format!(
                    "Failed to create directory {}: {e}",
                    parent.display()
                )));
            }
        }

        match std::fs::write(&path, &content) {
            Ok(()) => Ok(SkillOutput::success(format!(
                "HEARTBEAT.md updated ({}).\nThe scheduler will run this prompt every 30 minutes.",
                path.display()
            ))),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to write HEARTBEAT.md: {e}"
            ))),
        }
    }
}
