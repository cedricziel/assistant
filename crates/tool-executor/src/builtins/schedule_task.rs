//! Builtin handler for schedule-task tool — persists a cron-scheduled or one-shot prompt task.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use assistant_storage::StorageLayer;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
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
        "Schedule a prompt task. Supports recurring cron expressions and one-shot \
         execution at a specific datetime. Use `cron_expr` for recurring tasks, \
         `run_at` for one-shot tasks, or combine `cron_expr` with `once: true` \
         to run on the next cron tick only."
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
                    "description": "Cron expression (5-field standard or 7-field with seconds). Required unless `run_at` is provided."
                },
                "run_at": {
                    "type": "string",
                    "description": "ISO 8601 datetime for a one-shot task (e.g. '2026-03-01T09:00:00Z'). Mutually exclusive with `cron_expr`."
                },
                "prompt": {
                    "type": "string",
                    "description": "The prompt to run on each tick"
                },
                "once": {
                    "type": "boolean",
                    "description": "If true the task auto-disables after a single execution. Implied when `run_at` is used. Default: false."
                }
            },
            "required": ["name", "prompt"]
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

        let prompt = match params.get("prompt").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => {
                return Ok(ToolOutput::error("Missing required parameter 'prompt'"));
            }
        };

        let cron_expr = params.get("cron_expr").and_then(|v| v.as_str());
        let run_at = params.get("run_at").and_then(|v| v.as_str());
        let once_flag = params
            .get("once")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if cron_expr.is_some() && run_at.is_some() {
            return Ok(ToolOutput::error(
                "Provide either `cron_expr` or `run_at`, not both.",
            ));
        }

        if cron_expr.is_none() && run_at.is_none() {
            return Ok(ToolOutput::error(
                "Provide either `cron_expr` (recurring) or `run_at` (one-shot datetime).",
            ));
        }

        // -- One-shot via `run_at` ------------------------------------------------
        if let Some(raw) = run_at {
            let dt = match raw.parse::<DateTime<Utc>>() {
                Ok(dt) => dt,
                Err(e) => {
                    return Ok(ToolOutput::error(format!(
                        "Invalid ISO 8601 datetime '{}': {}",
                        raw, e
                    )));
                }
            };

            if dt <= Utc::now() {
                return Ok(ToolOutput::error(format!(
                    "run_at '{}' is in the past.",
                    raw
                )));
            }

            let id = self
                .storage
                .scheduled_task_store()
                .insert(&name, "", &prompt, true, Some(dt))
                .await?;

            return Ok(ToolOutput::success(format!(
                "One-shot task '{}' created (id: {}).\nRun at: {}\nPrompt: {}",
                name,
                id,
                dt.to_rfc3339(),
                prompt
            )));
        }

        // -- Cron-based (recurring or once) ---------------------------------------
        let cron_raw = cron_expr.unwrap();
        let (schedule, effective_expr) = match Schedule::from_str(cron_raw) {
            Ok(s) => (s, cron_raw.to_string()),
            Err(e) => {
                let extended = format!("0 {}", cron_raw);
                match Schedule::from_str(&extended) {
                    Ok(s) => (s, extended),
                    Err(_) => {
                        return Ok(ToolOutput::error(format!(
                            "Invalid cron expression '{}': {}",
                            cron_raw, e
                        )));
                    }
                }
            }
        };

        let next_run = schedule.upcoming(Utc).next();

        let id = self
            .storage
            .scheduled_task_store()
            .insert(&name, &effective_expr, &prompt, once_flag, next_run)
            .await?;

        let next_run_str = match next_run {
            Some(t) => t.to_rfc3339(),
            None => "never (cron expression has no future occurrences)".to_string(),
        };

        let mode = if once_flag { "once" } else { "recurring" };
        Ok(ToolOutput::success(format!(
            "Scheduled task '{}' created (id: {}, mode: {}).\nCron: {}\nNext run: {}\nPrompt: {}",
            name, id, mode, effective_expr, next_run_str, prompt
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Interface;
    use assistant_storage::StorageLayer;
    use uuid::Uuid;

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 1,
            interface: Interface::Cli,
            interactive: false,
        }
    }

    async fn handler() -> ScheduleTaskHandler {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        ScheduleTaskHandler::new(storage)
    }

    fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[tokio::test]
    async fn test_schedule_recurring_cron() {
        let h = handler().await;
        let p = params(&[
            ("name", serde_json::json!("daily")),
            ("cron_expr", serde_json::json!("0 9 * * *")),
            ("prompt", serde_json::json!("run report")),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(
            out.success,
            "recurring cron should succeed: {}",
            out.content
        );
        assert!(
            out.content.contains("recurring"),
            "mode should be recurring"
        );
    }

    #[tokio::test]
    async fn test_schedule_cron_once() {
        let h = handler().await;
        let p = params(&[
            ("name", serde_json::json!("one-timer")),
            ("cron_expr", serde_json::json!("0 9 * * *")),
            ("prompt", serde_json::json!("run once")),
            ("once", serde_json::json!(true)),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(out.success, "cron+once should succeed: {}", out.content);
        assert!(out.content.contains("once"), "mode should be once");
    }

    #[tokio::test]
    async fn test_schedule_run_at() {
        let h = handler().await;
        let future = (Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
        let p = params(&[
            ("name", serde_json::json!("reminder")),
            ("run_at", serde_json::json!(future)),
            ("prompt", serde_json::json!("remind me")),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(out.success, "run_at should succeed: {}", out.content);
        assert!(
            out.content.contains("One-shot"),
            "should indicate one-shot: {}",
            out.content
        );
    }

    #[tokio::test]
    async fn test_schedule_run_at_in_past_rejected() {
        let h = handler().await;
        let past = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let p = params(&[
            ("name", serde_json::json!("late")),
            ("run_at", serde_json::json!(past)),
            ("prompt", serde_json::json!("too late")),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(!out.success, "past run_at must be rejected");
        assert!(out.content.contains("past"));
    }

    #[tokio::test]
    async fn test_schedule_both_cron_and_run_at_rejected() {
        let h = handler().await;
        let future = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let p = params(&[
            ("name", serde_json::json!("both")),
            ("cron_expr", serde_json::json!("0 9 * * *")),
            ("run_at", serde_json::json!(future)),
            ("prompt", serde_json::json!("p")),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(!out.success, "both cron and run_at must be rejected");
    }

    #[tokio::test]
    async fn test_schedule_neither_cron_nor_run_at_rejected() {
        let h = handler().await;
        let p = params(&[
            ("name", serde_json::json!("neither")),
            ("prompt", serde_json::json!("p")),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(!out.success, "neither cron nor run_at must be rejected");
    }

    #[tokio::test]
    async fn test_schedule_invalid_cron_rejected() {
        let h = handler().await;
        let p = params(&[
            ("name", serde_json::json!("bad")),
            ("cron_expr", serde_json::json!("not a cron")),
            ("prompt", serde_json::json!("p")),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(!out.success, "invalid cron must be rejected");
        assert!(out.content.contains("Invalid cron"));
    }

    #[tokio::test]
    async fn test_schedule_invalid_run_at_rejected() {
        let h = handler().await;
        let p = params(&[
            ("name", serde_json::json!("bad-dt")),
            ("run_at", serde_json::json!("not-a-datetime")),
            ("prompt", serde_json::json!("p")),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(!out.success, "invalid datetime must be rejected");
        assert!(out.content.contains("Invalid ISO 8601"));
    }

    #[tokio::test]
    async fn test_schedule_missing_name_rejected() {
        let h = handler().await;
        let p = params(&[
            ("cron_expr", serde_json::json!("0 9 * * *")),
            ("prompt", serde_json::json!("p")),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("name"));
    }

    #[tokio::test]
    async fn test_schedule_missing_prompt_rejected() {
        let h = handler().await;
        let p = params(&[
            ("name", serde_json::json!("n")),
            ("cron_expr", serde_json::json!("0 9 * * *")),
        ]);

        let out = h.run(p, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("prompt"));
    }
}
