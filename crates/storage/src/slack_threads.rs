//! Persistence for Slack threads where the bot has been @-mentioned.
//!
//! Rows in `slack_active_threads` survive service restarts so the bot keeps
//! responding in threads it was previously invited to.  `last_seen_at` is
//! refreshed on every access so stale threads can be pruned.

use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;

/// SQLite-backed store for active Slack thread keys.
pub struct SlackActiveThreadStore {
    pool: SqlitePool,
}

impl SlackActiveThreadStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Return `true` if `(channel_id, thread_ts)` exists in the database.
    pub async fn contains(&self, channel_id: &str, thread_ts: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM slack_active_threads \
             WHERE channel_id = ?1 AND thread_ts = ?2",
        )
        .bind(channel_id)
        .bind(thread_ts)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    /// Insert or update a `(channel_id, thread_ts)` pair, refreshing `last_seen_at`.
    pub async fn upsert(&self, channel_id: &str, thread_ts: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO slack_active_threads (channel_id, thread_ts, last_seen_at) \
             VALUES (?1, ?2, ?3) \
             ON CONFLICT (channel_id, thread_ts) DO UPDATE SET last_seen_at = excluded.last_seen_at",
        )
        .bind(channel_id)
        .bind(thread_ts)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete threads that have not been seen for more than `older_than_days` days.
    /// Returns the number of rows deleted.
    pub async fn prune(&self, older_than_days: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(older_than_days);
        let result = sqlx::query("DELETE FROM slack_active_threads WHERE last_seen_at < ?1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
