//! Scheduled task persistence (cron-style recurring prompts and one-shot tasks).

use std::sync::Arc;

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// A single scheduled task row.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub agent_id: String,
    pub name: String,
    pub cron_expr: String,
    pub prompt: String,
    pub enabled: bool,
    pub once: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// SQLite-backed store for scheduled tasks.
pub struct ScheduledTaskStore {
    pool: SqlitePool,
    agent_id: String,
    /// Clock for row timestamps. Default `Arc::new(SystemClock)`.
    clock: Arc<dyn Clock>,
}

impl ScheduledTaskStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            agent_id: "default".to_string(),
            clock: Arc::new(SystemClock),
        }
    }

    pub fn for_agent(pool: SqlitePool, agent_id: impl Into<String>) -> Self {
        Self {
            pool,
            agent_id: agent_id.into(),
            clock: Arc::new(SystemClock),
        }
    }

    /// Inject a [`Clock`] implementation.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Insert a new scheduled task.
    pub async fn insert(
        &self,
        name: &str,
        cron_expr: &str,
        prompt: &str,
        once: bool,
        next_run: Option<DateTime<Utc>>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let now = self.clock.now();

        sqlx::query(
            "INSERT INTO scheduled_tasks \
                (id, agent_id, name, cron_expr, prompt, enabled, once, next_run, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, TRUE, ?6, ?7, ?8)",
        )
        .bind(&id_str)
        .bind(&self.agent_id)
        .bind(name)
        .bind(cron_expr)
        .bind(prompt)
        .bind(once)
        .bind(next_run)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// List all enabled tasks whose next_run is at or before `now`.
    pub async fn due_tasks(&self, now: DateTime<Utc>) -> Result<Vec<ScheduledTask>> {
        let rows = sqlx::query(
            "SELECT id, agent_id, name, cron_expr, prompt, enabled, once, last_run, next_run, created_at \
             FROM scheduled_tasks \
             WHERE agent_id = ?1 AND enabled = TRUE AND next_run <= ?2 \
             ORDER BY next_run ASC",
        )
        .bind(&self.agent_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(parse_row).collect()
    }

    /// List all tasks.
    pub async fn list_all(&self) -> Result<Vec<ScheduledTask>> {
        let rows = sqlx::query(
            "SELECT id, agent_id, name, cron_expr, prompt, enabled, once, last_run, next_run, created_at \
             FROM scheduled_tasks WHERE agent_id = ?1 ORDER BY created_at ASC",
        )
        .bind(&self.agent_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(parse_row).collect()
    }

    /// Update the last_run and next_run timestamps after execution.
    pub async fn record_run(
        &self,
        id: Uuid,
        last_run: DateTime<Utc>,
        next_run: Option<DateTime<Utc>>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE scheduled_tasks
             SET last_run = ?1, next_run = ?2
             WHERE id = ?3 AND agent_id = ?4",
        )
        .bind(last_run)
        .bind(next_run)
        .bind(id.to_string())
        .bind(&self.agent_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Disable a task (set `enabled = FALSE`).
    pub async fn disable(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE scheduled_tasks
             SET enabled = FALSE
             WHERE id = ?1 AND agent_id = ?2 AND enabled = TRUE",
        )
        .bind(id.to_string())
        .bind(&self.agent_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete a task permanently.
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM scheduled_tasks WHERE id = ?1 AND agent_id = ?2")
            .bind(id.to_string())
            .bind(&self.agent_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Find a task by name (case-insensitive).
    pub async fn find_by_name(&self, name: &str) -> Result<Option<ScheduledTask>> {
        let row = sqlx::query(
            "SELECT id, agent_id, name, cron_expr, prompt, enabled, once, last_run, next_run, created_at \
             FROM scheduled_tasks WHERE agent_id = ?1 AND LOWER(name) = LOWER(?2) LIMIT 1",
        )
        .bind(&self.agent_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        row.map(parse_row).transpose()
    }
}

fn parse_row(r: sqlx::sqlite::SqliteRow) -> Result<ScheduledTask> {
    let raw_id: String = r.get("id");
    Ok(ScheduledTask {
        id: Uuid::parse_str(&raw_id)?,
        agent_id: r.get("agent_id"),
        name: r.get("name"),
        cron_expr: r.get("cron_expr"),
        prompt: r.get("prompt"),
        enabled: r.get("enabled"),
        once: r.get("once"),
        last_run: r.get("last_run"),
        next_run: r.get("next_run"),
        created_at: r.get("created_at"),
    })
}

// -- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;
    use chrono::Duration;

    async fn store() -> (StorageLayer, ScheduledTaskStore) {
        let s = StorageLayer::new_in_memory().await.unwrap();
        let ts = s.scheduled_task_store();
        (s, ts)
    }

    #[tokio::test]
    async fn test_insert_and_list_all() {
        let (_s, ts) = store().await;
        let next = Utc::now() + Duration::hours(1);

        let id = ts
            .insert(
                "daily-report",
                "0 0 9 * * *",
                "run report",
                false,
                Some(next),
            )
            .await
            .unwrap();

        let all = ts.list_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, id);
        assert_eq!(all[0].name, "daily-report");
        assert_eq!(all[0].cron_expr, "0 0 9 * * *");
        assert_eq!(all[0].prompt, "run report");
        assert!(all[0].enabled);
        assert!(!all[0].once);
        assert!(all[0].last_run.is_none());
    }

    #[tokio::test]
    async fn test_insert_once_flag() {
        let (_s, ts) = store().await;
        let next = Utc::now() + Duration::hours(1);

        ts.insert("one-shot", "", "do thing", true, Some(next))
            .await
            .unwrap();

        let all = ts.list_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].once, "once flag must be persisted");
    }

    #[tokio::test]
    async fn test_due_tasks_returns_only_past_enabled() {
        let (_s, ts) = store().await;
        let past = Utc::now() - Duration::hours(1);
        let future = Utc::now() + Duration::hours(1);

        ts.insert("past-task", "0 * * * *", "p1", false, Some(past))
            .await
            .unwrap();
        ts.insert("future-task", "0 * * * *", "p2", false, Some(future))
            .await
            .unwrap();

        let due = ts.due_tasks(Utc::now()).await.unwrap();
        assert_eq!(due.len(), 1, "only the past task should be due");
        assert_eq!(due[0].name, "past-task");
    }

    #[tokio::test]
    async fn test_due_tasks_excludes_disabled() {
        let (_s, ts) = store().await;
        let past = Utc::now() - Duration::hours(1);

        let id = ts
            .insert("disabled-task", "0 * * * *", "p", false, Some(past))
            .await
            .unwrap();
        ts.disable(id).await.unwrap();

        let due = ts.due_tasks(Utc::now()).await.unwrap();
        assert!(due.is_empty(), "disabled tasks must not appear as due");
    }

    #[tokio::test]
    async fn test_record_run_updates_timestamps() {
        let (_s, ts) = store().await;
        let past = Utc::now() - Duration::hours(1);
        let new_next = Utc::now() + Duration::hours(1);

        let id = ts
            .insert("t", "0 * * * *", "p", false, Some(past))
            .await
            .unwrap();

        let now = Utc::now();
        ts.record_run(id, now, Some(new_next)).await.unwrap();

        let all = ts.list_all().await.unwrap();
        assert!(all[0].last_run.is_some(), "last_run must be set");
        assert_eq!(
            all[0].next_run.unwrap().timestamp(),
            new_next.timestamp(),
            "next_run must be updated"
        );
    }

    #[tokio::test]
    async fn test_disable_returns_true_then_false() {
        let (_s, ts) = store().await;
        let next = Utc::now() + Duration::hours(1);

        let id = ts
            .insert("t", "0 * * * *", "p", false, Some(next))
            .await
            .unwrap();

        assert!(
            ts.disable(id).await.unwrap(),
            "first disable should return true"
        );
        assert!(
            !ts.disable(id).await.unwrap(),
            "second disable should return false (already disabled)"
        );

        let all = ts.list_all().await.unwrap();
        assert!(!all[0].enabled);
    }

    #[tokio::test]
    async fn test_delete_removes_row() {
        let (_s, ts) = store().await;
        let next = Utc::now() + Duration::hours(1);

        let id = ts
            .insert("t", "0 * * * *", "p", false, Some(next))
            .await
            .unwrap();

        assert!(ts.delete(id).await.unwrap(), "delete should return true");
        assert!(
            !ts.delete(id).await.unwrap(),
            "second delete should return false (row gone)"
        );
        assert!(ts.list_all().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_find_by_name_case_insensitive() {
        let (_s, ts) = store().await;
        let next = Utc::now() + Duration::hours(1);

        ts.insert("My-Task", "0 * * * *", "p", false, Some(next))
            .await
            .unwrap();

        let found = ts.find_by_name("my-task").await.unwrap();
        assert!(
            found.is_some(),
            "case-insensitive lookup must find the task"
        );
        assert_eq!(found.unwrap().name, "My-Task");
    }

    #[tokio::test]
    async fn test_find_by_name_returns_none_when_missing() {
        let (_s, ts) = store().await;
        let found = ts.find_by_name("nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_all_includes_disabled() {
        let (_s, ts) = store().await;
        let next = Utc::now() + Duration::hours(1);

        let id = ts
            .insert("t", "0 * * * *", "p", false, Some(next))
            .await
            .unwrap();
        ts.disable(id).await.unwrap();

        let all = ts.list_all().await.unwrap();
        assert_eq!(all.len(), 1, "list_all must include disabled tasks");
        assert!(!all[0].enabled);
    }
}
