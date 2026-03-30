//! SQLite-backed query backends — thin wrappers around `TraceStore`/`LogStore`.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use assistant_storage::{
    LogStats, LogStore, RecordedLog, RecordedSpan, TraceFilter, TraceStore, TraceSummary,
};

use super::{LogBackend, TraceBackend};

// -- SqliteTraceBackend -------------------------------------------------------

pub struct SqliteTraceBackend {
    pool: SqlitePool,
}

impl SqliteTraceBackend {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TraceBackend for SqliteTraceBackend {
    async fn list_recent_traces(
        &self,
        limit: i64,
        filter: &TraceFilter,
        agent_id: &str,
    ) -> Result<Vec<TraceSummary>> {
        TraceStore::new(self.pool.clone())
            .list_recent_traces_for_agent(limit, filter, agent_id)
            .await
    }

    async fn get_trace(&self, trace_id: &str, agent_id: &str) -> Result<Vec<RecordedSpan>> {
        TraceStore::new(self.pool.clone())
            .get_trace_for_agent(trace_id, agent_id)
            .await
    }
}

// -- SqliteLogBackend ---------------------------------------------------------

pub struct SqliteLogBackend {
    pool: SqlitePool,
}

impl SqliteLogBackend {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LogBackend for SqliteLogBackend {
    async fn list_recent_logs(
        &self,
        limit: i64,
        min_severity: Option<i32>,
        target_filter: Option<&str>,
        search: Option<&str>,
        trace_id: Option<&str>,
        conversation_id: Option<&str>,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
        agent_id: &str,
    ) -> Result<Vec<RecordedLog>> {
        LogStore::new(self.pool.clone())
            .list_recent_for_agent(
                limit,
                min_severity,
                target_filter,
                search,
                trace_id,
                conversation_id,
                since,
                until,
                agent_id,
            )
            .await
    }

    async fn get_log(&self, id: &str, agent_id: &str) -> Result<Option<RecordedLog>> {
        LogStore::new(self.pool.clone())
            .get_log_for_agent(id, agent_id)
            .await
    }

    async fn log_stats(&self, agent_id: &str) -> Result<LogStats> {
        LogStore::new(self.pool.clone())
            .stats_for_agent(agent_id)
            .await
    }

    async fn list_targets(&self, agent_id: &str) -> Result<Vec<String>> {
        LogStore::new(self.pool.clone())
            .list_targets_for_agent(agent_id)
            .await
    }
}
