//! Persistence for Slack threads where the bot has been @-mentioned.
//!
//! Rows in `slack_active_threads` survive service restarts so the bot keeps
//! responding in threads it was previously invited to.  `last_seen_at` is
//! refreshed on every access so stale threads can be pruned.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

/// Trait-based interface for active Slack thread persistence.
#[async_trait]
pub trait SlackActiveThreadStore: Send + Sync {
    /// Return `true` if `(channel_id, thread_ts)` exists.
    async fn contains(&self, channel_id: &str, thread_ts: &str) -> Result<bool>;

    /// Insert or refresh `(channel_id, thread_ts)`.
    async fn upsert(&self, channel_id: &str, thread_ts: &str) -> Result<()>;

    /// Delete rows whose `last_seen_at` is older than `older_than_days`.
    /// Returns the number of pruned rows.
    async fn prune(&self, older_than_days: i64) -> Result<u64>;
}

/// SQLite-backed store for active Slack thread keys.
pub struct SqliteSlackActiveThreadStore {
    pool: SqlitePool,
    /// Clock for row timestamps. Default `Arc::new(SystemClock)`.
    clock: Arc<dyn Clock>,
}

impl SqliteSlackActiveThreadStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            clock: Arc::new(SystemClock),
        }
    }

    /// Inject a [`Clock`] implementation.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }
}

#[async_trait]
impl SlackActiveThreadStore for SqliteSlackActiveThreadStore {
    /// Return `true` if `(channel_id, thread_ts)` exists in the database.
    async fn contains(&self, channel_id: &str, thread_ts: &str) -> Result<bool> {
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
    async fn upsert(&self, channel_id: &str, thread_ts: &str) -> Result<()> {
        let now = self.clock.now();
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
    async fn prune(&self, older_than_days: i64) -> Result<u64> {
        let cutoff = self.clock.now() - chrono::Duration::days(older_than_days);
        let result = sqlx::query("DELETE FROM slack_active_threads WHERE last_seen_at < ?1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

type ThreadKey = (String, String);
type ThreadMap = HashMap<ThreadKey, DateTime<Utc>>;

/// In-memory [`SlackActiveThreadStore`]. HashMap-backed.
pub struct InMemorySlackActiveThreadStore {
    state: Arc<Mutex<ThreadMap>>,
    clock: Arc<dyn Clock>,
}

impl InMemorySlackActiveThreadStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ThreadMap> {
        match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }
}

impl Default for InMemorySlackActiveThreadStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SlackActiveThreadStore for InMemorySlackActiveThreadStore {
    async fn contains(&self, channel_id: &str, thread_ts: &str) -> Result<bool> {
        let state = self.lock();
        Ok(state.contains_key(&(channel_id.to_string(), thread_ts.to_string())))
    }

    async fn upsert(&self, channel_id: &str, thread_ts: &str) -> Result<()> {
        let now = self.clock.now();
        let mut state = self.lock();
        state.insert((channel_id.to_string(), thread_ts.to_string()), now);
        Ok(())
    }

    async fn prune(&self, older_than_days: i64) -> Result<u64> {
        let cutoff = self.clock.now() - chrono::Duration::days(older_than_days);
        let mut state = self.lock();
        let before = state.len();
        state.retain(|_, last_seen| *last_seen >= cutoff);
        Ok((before - state.len()) as u64)
    }
}
