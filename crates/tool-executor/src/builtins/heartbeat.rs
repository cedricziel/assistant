//! Builtin handlers for `heartbeat-read` and `heartbeat-update` tools.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{AssistantConfig, ExecutionContext, ToolHandler, ToolOutput};
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
impl ToolHandler for HeartbeatReadHandler {
    fn name(&self) -> &str {
        "heartbeat-read"
    }

    fn description(&self) -> &str {
        "Read the current HEARTBEAT.md file which specifies what the assistant checks every 30 minutes."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn run(
        &self,
        _params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let path = heartbeat_path(&self.config);

        if !path.exists() {
            return Ok(ToolOutput::success(format!(
                "HEARTBEAT.md does not exist yet ({}). Use heartbeat-update to create it.",
                path.display()
            )));
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => Ok(ToolOutput::success(format!(
                "# HEARTBEAT.md ({})\n\n{}",
                path.display(),
                content
            ))),
            Err(e) => Ok(ToolOutput::error(format!(
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
impl ToolHandler for HeartbeatUpdateHandler {
    fn name(&self) -> &str {
        "heartbeat-update"
    }

    fn description(&self) -> &str {
        "Update or create the HEARTBEAT.md file. The scheduler runs its contents as a prompt every 30 minutes."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "New content for HEARTBEAT.md (Markdown)"
                }
            },
            "required": ["content"]
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'content'")),
        };

        let path = heartbeat_path(&self.config);

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Ok(ToolOutput::error(format!(
                    "Failed to create directory {}: {e}",
                    parent.display()
                )));
            }
        }

        match std::fs::write(&path, &content) {
            Ok(()) => Ok(ToolOutput::success(format!(
                "HEARTBEAT.md updated ({}).\nThe scheduler will run this prompt every 30 minutes.",
                path.display()
            ))),
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to write HEARTBEAT.md: {e}"
            ))),
        }
    }
}
