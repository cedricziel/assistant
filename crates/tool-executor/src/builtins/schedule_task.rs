//! Builtin handler for schedule-task tool — persists a cron-scheduled prompt task.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use assistant_storage::StorageLayer;
use async_trait::async_trait;
use chrono::Utc;
use cron::Schedule;

pub struct ScheduleTaskHandler {
    storage: Arc<StorageLayer>,
}

impl ScheduleTaskHandler {
    pub fn new(storage: Arc<StorageLayer>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl ToolHandler for ScheduleTaskHandler {
    fn name(&self) -> &str {
        "schedule-task"
    }

    fn description(&self) -> &str {
        "Schedule a recurring prompt task using a cron expression. The assistant will run the prompt automatically on each tick."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Human-readable name for the task"
                },
                "cron_expr": {
                    "type": "string",
                    "description": "Cron expression (5-field standard or 7-field with seconds)"
                },
                "prompt": {
                    "type": "string",
                    "description": "The prompt to run on each tick"
                }
            },
            "required": ["name", "cron_expr", "prompt"]
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
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => {
                return Ok(ToolOutput::error("Missing required parameter 'name'"));
            }
        };

        let cron_expr = match params.get("cron_expr").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => {
                return Ok(ToolOutput::error("Missing required parameter 'cron_expr'"));
            }
        };

        let prompt = match params.get("prompt").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => {
                return Ok(ToolOutput::error("Missing required parameter 'prompt'"));
            }
        };

        // Validate and compute next run time from the cron expression.
        let schedule = match Schedule::from_str(&cron_expr) {
            Ok(s) => s,
            Err(e) => {
                // Try prefixing with "0 " (seconds=0) to handle standard 5-field cron
                let extended = format!("0 {}", cron_expr);
                match Schedule::from_str(&extended) {
                    Ok(s) => s,
                    Err(_) => {
                        return Ok(ToolOutput::error(format!(
                            "Invalid cron expression '{}': {}",
                            cron_expr, e
                        )));
                    }
                }
            }
        };

        let next_run = schedule.upcoming(Utc).next();

        let id = self
            .storage
            .scheduled_task_store()
            .insert(&name, &cron_expr, &prompt, next_run)
            .await?;

        let next_run_str = match next_run {
            Some(t) => t.to_rfc3339(),
            None => "never (cron expression has no future occurrences)".to_string(),
        };

        Ok(ToolOutput::success(format!(
            "Scheduled task '{}' created (id: {}).\nCron expression: {}\nNext run: {}\nPrompt: {}",
            name, id, cron_expr, next_run_str, prompt
        )))
    }
}
