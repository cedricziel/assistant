//! SQLite-backed query backends — thin wrappers around storage stores.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use assistant_storage::{
    LogStats, LogStore, MetricsStore, MetricsSummary, ModelTokenUsage, RecordedLog, RecordedSpan,
    SqliteTraceStore, TimeSeriesPoint, ToolUsageStats, TraceFilter, TraceStore, TraceSummary,
};

use super::{LogBackend, MetricsBackend, TraceBackend};

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
        let store = SqliteTraceStore::new(self.pool.clone());
        if agent_id.is_empty() {
            // Unscoped: the base query only accepts a skill filter, so we
            // fetch a larger batch and apply the remaining filters in memory.
            let fetch_limit = if filter.has_post_filters() {
                limit * 4
            } else {
                limit
            };
            let mut traces = store
                .list_recent_traces(fetch_limit, filter.skill.as_deref())
                .await?;
            if filter.has_post_filters() {
                traces.retain(|t| {
                    if let Some(ref status) = filter.status {
                        let t_status = if t.error_count > 0 { "error" } else { "ok" };
                        if t_status != status.as_str() {
                            return false;
                        }
                    }
                    if let Some(conv) = filter.conversation
                        && t.conversation_id != Some(conv)
                    {
                        return false;
                    }
                    if let Some(since) = filter.since
                        && t.start_time < since
                    {
                        return false;
                    }
                    if let Some(until) = filter.until
                        && t.start_time > until
                    {
                        return false;
                    }
                    true
                });
                traces.truncate(limit as usize);
            }
            Ok(traces)
        } else {
            store
                .list_recent_traces_for_agent(limit, filter, agent_id)
                .await
        }
    }

    async fn get_trace(&self, trace_id: &str, agent_id: &str) -> Result<Vec<RecordedSpan>> {
        let store = SqliteTraceStore::new(self.pool.clone());
        if agent_id.is_empty() {
            store.get_trace(trace_id).await
        } else {
            store.get_trace_for_agent(trace_id, agent_id).await
        }
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
        let store = LogStore::new(self.pool.clone());
        if agent_id.is_empty() {
            // Unscoped: return all logs (top-level API view).
            store
                .list_recent_filtered(
                    limit,
                    min_severity,
                    target_filter,
                    search,
                    trace_id,
                    conversation_id,
                    since,
                    until,
                )
                .await
        } else {
            store
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
    }

    async fn get_log(&self, id: &str, agent_id: &str) -> Result<Option<RecordedLog>> {
        let store = LogStore::new(self.pool.clone());
        if agent_id.is_empty() {
            store.get_log(id).await
        } else {
            store.get_log_for_agent(id, agent_id).await
        }
    }

    async fn log_stats(&self, agent_id: &str) -> Result<LogStats> {
        let store = LogStore::new(self.pool.clone());
        if agent_id.is_empty() {
            store.stats().await
        } else {
            store.stats_for_agent(agent_id).await
        }
    }

    async fn list_targets(&self, agent_id: &str) -> Result<Vec<String>> {
        let store = LogStore::new(self.pool.clone());
        if agent_id.is_empty() {
            store.list_targets().await
        } else {
            store.list_targets_for_agent(agent_id).await
        }
    }
}

// -- SqliteMetricsBackend -----------------------------------------------------

pub struct SqliteMetricsBackend {
    pool: SqlitePool,
}

impl SqliteMetricsBackend {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MetricsBackend for SqliteMetricsBackend {
    async fn summary(&self, window_hours: i64, agent_id: &str) -> Result<MetricsSummary> {
        MetricsStore::for_agent(self.pool.clone(), agent_id)
            .summary(window_hours)
            .await
    }

    async fn token_usage_over_time(
        &self,
        window_hours: i64,
        bucket_minutes: i64,
        agent_id: &str,
    ) -> Result<Vec<TimeSeriesPoint>> {
        MetricsStore::for_agent(self.pool.clone(), agent_id)
            .token_usage_over_time(window_hours, bucket_minutes)
            .await
    }

    async fn model_comparison(
        &self,
        window_hours: i64,
        agent_id: &str,
    ) -> Result<Vec<ModelTokenUsage>> {
        MetricsStore::for_agent(self.pool.clone(), agent_id)
            .model_comparison(window_hours)
            .await
    }

    async fn tool_usage(&self, window_hours: i64, agent_id: &str) -> Result<Vec<ToolUsageStats>> {
        MetricsStore::for_agent(self.pool.clone(), agent_id)
            .tool_usage(window_hours)
            .await
    }

    async fn request_rate(
        &self,
        window_hours: i64,
        bucket_minutes: i64,
        agent_id: &str,
    ) -> Result<Vec<TimeSeriesPoint>> {
        MetricsStore::for_agent(self.pool.clone(), agent_id)
            .request_rate(window_hours, bucket_minutes)
            .await
    }
}
