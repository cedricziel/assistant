//! Scheduled task persistence (cron-style recurring prompts).

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// A single scheduled task row.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub name: String,
    pub cron_expr: String,
    pub prompt: String,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// SQLite-backed store for scheduled tasks.
pub struct ScheduledTaskStore {
    pool: SqlitePool,
}

impl ScheduledTaskStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new scheduled task.
    pub async fn insert(
        &self,
        name: &str,
        cron_expr: &str,
        prompt: &str,
        next_run: Option<DateTime<Utc>>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO scheduled_tasks \
                (id, name, cron_expr, prompt, enabled, next_run, created_at) \
             VALUES (?1, ?2, ?3, ?4, TRUE, ?5, ?6)",
        )
        .bind(&id_str)
        .bind(name)
        .bind(cron_expr)
        .bind(prompt)
        .bind(next_run)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// List all enabled tasks whose next_run is at or before `now`.
    pub async fn due_tasks(&self, now: DateTime<Utc>) -> Result<Vec<ScheduledTask>> {
        let rows = sqlx::query(
            "SELECT id, name, cron_expr, prompt, enabled, last_run, next_run, created_at \
             FROM scheduled_tasks \
             WHERE enabled = TRUE AND next_run <= ?1 \
             ORDER BY next_run ASC",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(parse_row).collect()
    }

    /// List all tasks.
    pub async fn list_all(&self) -> Result<Vec<ScheduledTask>> {
        let rows = sqlx::query(
            "SELECT id, name, cron_expr, prompt, enabled, last_run, next_run, created_at \
             FROM scheduled_tasks ORDER BY created_at ASC",
        )
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
        sqlx::query("UPDATE scheduled_tasks SET last_run = ?1, next_run = ?2 WHERE id = ?3")
            .bind(last_run)
            .bind(next_run)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn parse_row(r: sqlx::sqlite::SqliteRow) -> Result<ScheduledTask> {
    let raw_id: String = r.get("id");
    Ok(ScheduledTask {
        id: Uuid::parse_str(&raw_id)?,
        name: r.get("name"),
        cron_expr: r.get("cron_expr"),
        prompt: r.get("prompt"),
        enabled: r.get("enabled"),
        last_run: r.get("last_run"),
        next_run: r.get("next_run"),
        created_at: r.get("created_at"),
    })
}
