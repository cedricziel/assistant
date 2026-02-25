//! Builtin handler for list-tasks tool — lists all scheduled tasks.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use assistant_storage::StorageLayer;
use async_trait::async_trait;

pub struct ListTasksHandler {
    storage: Arc<StorageLayer>,
}

impl ListTasksHandler {
    pub fn new(storage: Arc<StorageLayer>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl ToolHandler for ListTasksHandler {
    fn name(&self) -> &str {
        "list-tasks"
    }

    fn description(&self) -> &str {
        "List all scheduled tasks with their status, cron expression, next run time, and prompt."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "enabled_only": {
                    "type": "boolean",
                    "description": "If true, only show enabled (active) tasks. Default: false (show all)."
                }
            }
        })
    }

    fn is_mutating(&self) -> bool {
        false
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let enabled_only = params
            .get("enabled_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let task_store = self.storage.scheduled_task_store();
        let tasks = task_store.list_all().await?;

        let tasks: Vec<_> = if enabled_only {
            tasks.into_iter().filter(|t| t.enabled).collect()
        } else {
            tasks
        };

        if tasks.is_empty() {
            return Ok(ToolOutput::success("No scheduled tasks found.".to_string()));
        }

        let mut lines = Vec::with_capacity(tasks.len() + 1);
        lines.push(format!("Found {} scheduled task(s):\n", tasks.len()));

        for t in &tasks {
            let status = if t.enabled { "enabled" } else { "disabled" };
            let mode = if t.once { "once" } else { "recurring" };
            let schedule = if t.cron_expr.is_empty() {
                "one-shot (run_at)".to_string()
            } else {
                format!("cron: {}", t.cron_expr)
            };

            let next = t
                .next_run
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "none".to_string());

            let last = t
                .last_run
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "never".to_string());

            lines.push(format!(
                "- **{}** (id: {})\n  Status: {} | Mode: {} | {}\n  Next run: {} | Last run: {}\n  Prompt: {}",
                t.name, t.id, status, mode, schedule, next, last, t.prompt
            ));
        }

        Ok(ToolOutput::success(lines.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Interface;
    use assistant_storage::StorageLayer;
    use chrono::Utc;
    use uuid::Uuid;

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 1,
            interface: Interface::Cli,
            interactive: false,
        }
    }

    async fn setup() -> (Arc<StorageLayer>, ListTasksHandler) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let handler = ListTasksHandler::new(storage.clone());
        (storage, handler)
    }

    fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[tokio::test]
    async fn test_list_empty() {
        let (_storage, h) = setup().await;
        let out = h.run(params(&[]), &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("No scheduled tasks found"));
    }

    #[tokio::test]
    async fn test_list_shows_all_tasks() {
        let (storage, h) = setup().await;
        let next = Utc::now() + chrono::Duration::hours(1);
        let ts = storage.scheduled_task_store();

        ts.insert("task-a", "0 * * * *", "prompt-a", false, Some(next))
            .await
            .unwrap();
        ts.insert("task-b", "", "prompt-b", true, Some(next))
            .await
            .unwrap();

        let out = h.run(params(&[]), &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("task-a"), "must list task-a");
        assert!(out.content.contains("task-b"), "must list task-b");
        assert!(out.content.contains("recurring"), "task-a is recurring");
        assert!(out.content.contains("once"), "task-b is once");
        assert!(
            out.content.contains("one-shot (run_at)"),
            "task-b has no cron, should show one-shot"
        );
    }

    #[tokio::test]
    async fn test_list_enabled_only_filter() {
        let (storage, h) = setup().await;
        let next = Utc::now() + chrono::Duration::hours(1);
        let ts = storage.scheduled_task_store();

        ts.insert("active", "0 * * * *", "p", false, Some(next))
            .await
            .unwrap();
        let disabled_id = ts
            .insert("inactive", "0 * * * *", "p", false, Some(next))
            .await
            .unwrap();
        ts.disable(disabled_id).await.unwrap();

        let out = h
            .run(params(&[("enabled_only", serde_json::json!(true))]), &ctx())
            .await
            .unwrap();
        assert!(out.success);
        assert!(out.content.contains("active"), "enabled task must appear");
        assert!(
            !out.content.contains("inactive"),
            "disabled task must be filtered out"
        );
    }

    #[tokio::test]
    async fn test_list_includes_disabled_by_default() {
        let (storage, h) = setup().await;
        let next = Utc::now() + chrono::Duration::hours(1);
        let ts = storage.scheduled_task_store();

        let id = ts
            .insert("gone", "0 * * * *", "p", false, Some(next))
            .await
            .unwrap();
        ts.disable(id).await.unwrap();

        let out = h.run(params(&[]), &ctx()).await.unwrap();
        assert!(out.success);
        assert!(
            out.content.contains("gone"),
            "disabled task must appear by default"
        );
        assert!(
            out.content.contains("disabled"),
            "status should say disabled"
        );
    }
}
