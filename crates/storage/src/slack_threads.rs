//! Persistence for Slack threads where the bot has been @-mentioned.
//!
//! Rows in `slack_active_threads` survive service restarts so the bot keeps
//! responding in threads it was previously invited to.

use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashSet;

/// SQLite-backed store for active Slack thread keys.
pub struct SlackActiveThreadStore {
    pool: SqlitePool,
}

impl SlackActiveThreadStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Load all `(channel_id, thread_ts)` pairs from the database.
    pub async fn load_all(&self) -> Result<HashSet<(String, String)>> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT channel_id, thread_ts FROM slack_active_threads",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().collect())
    }

    /// Persist a `(channel_id, thread_ts)` pair. Idempotent (INSERT OR IGNORE).
    pub async fn insert(&self, channel_id: &str, thread_ts: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO slack_active_threads (channel_id, thread_ts) VALUES (?1, ?2)",
        )
        .bind(channel_id)
        .bind(thread_ts)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
