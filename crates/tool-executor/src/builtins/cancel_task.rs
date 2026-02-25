//! Builtin handler for cancel-task tool — disables or deletes a scheduled task.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use assistant_storage::StorageLayer;
use async_trait::async_trait;
use uuid::Uuid;

pub struct CancelTaskHandler {
    storage: Arc<StorageLayer>,
}

impl CancelTaskHandler {
    pub fn new(storage: Arc<StorageLayer>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl ToolHandler for CancelTaskHandler {
    fn name(&self) -> &str {
        "cancel-task"
    }

    fn description(&self) -> &str {
        "Cancel a scheduled task by ID or name. By default the task is disabled \
         (kept in history). Pass `delete: true` to remove it permanently."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "UUID of the task to cancel"
                },
                "name": {
                    "type": "string",
                    "description": "Name of the task to cancel (case-insensitive). Used when id is not provided."
                },
                "delete": {
                    "type": "boolean",
                    "description": "If true, permanently delete the task instead of disabling it. Default: false."
                }
            }
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
        let id_str = params.get("id").and_then(|v| v.as_str());
        let name = params.get("name").and_then(|v| v.as_str());
        let hard_delete = params
            .get("delete")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let task_store = self.storage.scheduled_task_store();

        // Resolve the task UUID from either `id` or `name`.
        let task_id = if let Some(raw) = id_str {
            match Uuid::parse_str(raw) {
                Ok(u) => u,
                Err(e) => {
                    return Ok(ToolOutput::error(format!("Invalid UUID '{}': {}", raw, e)));
                }
            }
        } else if let Some(n) = name {
            match task_store.find_by_name(n).await? {
                Some(t) => t.id,
                None => {
                    return Ok(ToolOutput::error(format!(
                        "No scheduled task found with name '{}'.",
                        n
                    )));
                }
            }
        } else {
            return Ok(ToolOutput::error(
                "Provide either `id` (UUID) or `name` to identify the task.",
            ));
        };

        if hard_delete {
            let deleted = task_store.delete(task_id).await?;
            if deleted {
                Ok(ToolOutput::success(format!(
                    "Task {} deleted permanently.",
                    task_id
                )))
            } else {
                Ok(ToolOutput::error(format!("Task {} not found.", task_id)))
            }
        } else {
            let disabled = task_store.disable(task_id).await?;
            if disabled {
                Ok(ToolOutput::success(format!(
                    "Task {} disabled. It will no longer run. Pass `delete: true` to remove it entirely.",
                    task_id
                )))
            } else {
                Ok(ToolOutput::error(format!(
                    "Task {} not found or already disabled.",
                    task_id
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Interface;
    use assistant_storage::StorageLayer;
    use chrono::Utc;

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 1,
            interface: Interface::Cli,
            interactive: false,
        }
    }

    async fn setup() -> (Arc<StorageLayer>, CancelTaskHandler) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let handler = CancelTaskHandler::new(storage.clone());
        (storage, handler)
    }

    fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    /// Insert a test task and return its UUID string.
    async fn seed_task(storage: &StorageLayer, name: &str) -> String {
        let next = Utc::now() + chrono::Duration::hours(1);
        let id = storage
            .scheduled_task_store()
            .insert(name, "0 * * * *", "prompt", false, Some(next))
            .await
            .unwrap();
        id.to_string()
    }

    #[tokio::test]
    async fn test_cancel_by_id_disables() {
        let (storage, h) = setup().await;
        let id = seed_task(&storage, "my-task").await;

        let out = h
            .run(params(&[("id", serde_json::json!(id))]), &ctx())
            .await
            .unwrap();
        assert!(out.success, "cancel by id should succeed: {}", out.content);
        assert!(out.content.contains("disabled"));

        let all = storage.scheduled_task_store().list_all().await.unwrap();
        assert!(!all[0].enabled, "task must be disabled in DB");
    }

    #[tokio::test]
    async fn test_cancel_by_name_disables() {
        let (storage, h) = setup().await;
        seed_task(&storage, "My-Task").await;

        let out = h
            .run(params(&[("name", serde_json::json!("my-task"))]), &ctx())
            .await
            .unwrap();
        assert!(
            out.success,
            "cancel by name should succeed: {}",
            out.content
        );
    }

    #[tokio::test]
    async fn test_cancel_by_id_with_delete() {
        let (storage, h) = setup().await;
        let id = seed_task(&storage, "doomed").await;

        let out = h
            .run(
                params(&[
                    ("id", serde_json::json!(id)),
                    ("delete", serde_json::json!(true)),
                ]),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(out.success, "hard delete should succeed: {}", out.content);
        assert!(out.content.contains("deleted permanently"));

        let all = storage.scheduled_task_store().list_all().await.unwrap();
        assert!(all.is_empty(), "task must be removed from DB");
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_name_errors() {
        let (_storage, h) = setup().await;
        let out = h
            .run(params(&[("name", serde_json::json!("nope"))]), &ctx())
            .await
            .unwrap();
        assert!(!out.success);
        assert!(out.content.contains("No scheduled task found"));
    }

    #[tokio::test]
    async fn test_cancel_invalid_uuid_errors() {
        let (_storage, h) = setup().await;
        let out = h
            .run(params(&[("id", serde_json::json!("not-a-uuid"))]), &ctx())
            .await
            .unwrap();
        assert!(!out.success);
        assert!(out.content.contains("Invalid UUID"));
    }

    #[tokio::test]
    async fn test_cancel_no_id_or_name_errors() {
        let (_storage, h) = setup().await;
        let out = h.run(params(&[]), &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("Provide either"));
    }

    #[tokio::test]
    async fn test_cancel_already_disabled_errors() {
        let (storage, h) = setup().await;
        let id = seed_task(&storage, "t").await;

        // First cancel succeeds
        h.run(params(&[("id", serde_json::json!(&id))]), &ctx())
            .await
            .unwrap();

        // Second cancel fails (already disabled)
        let out = h
            .run(params(&[("id", serde_json::json!(&id))]), &ctx())
            .await
            .unwrap();
        assert!(!out.success, "already-disabled cancel must error");
        assert!(out.content.contains("already disabled"));
    }
}
