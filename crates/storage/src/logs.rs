//! Log record storage backed by OpenTelemetry log records persisted in SQLite.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

/// A persisted OpenTelemetry log record row.
#[derive(Debug, Clone)]
pub struct RecordedLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub observed_timestamp: Option<DateTime<Utc>>,
    pub severity_number: Option<i32>,
    pub severity_text: Option<String>,
    pub body: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub span_name: Option<String>,
    pub service_name: Option<String>,
    pub target: Option<String>,
    pub attributes: Value,
}

/// Aggregate counts by severity level.
#[derive(Debug, Clone, Default)]
pub struct LogStats {
    pub total: i64,
    pub trace_count: i64,
    pub debug_count: i64,
    pub info_count: i64,
    pub warn_count: i64,
    pub error_count: i64,
    pub fatal_count: i64,
}

/// SQLite-backed store for log records.
pub struct LogStore {
    pool: SqlitePool,
}

impl LogStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Return the `limit` most-recent log records, optionally filtered.
    pub async fn list_recent(
        &self,
        limit: i64,
        min_severity: Option<i32>,
        target_filter: Option<&str>,
        search: Option<&str>,
        trace_id: Option<&str>,
    ) -> Result<Vec<RecordedLog>> {
        let rows = sqlx::query(
            "SELECT l.id, l.timestamp, l.observed_timestamp, l.severity_number, l.severity_text, \
                    l.body, l.trace_id, l.span_id, dt.name AS span_name, l.service_name, l.target, l.attributes \
             FROM logs l \
             LEFT JOIN distributed_traces dt ON dt.span_id = l.span_id AND dt.trace_id = l.trace_id \
             WHERE (?1 IS NULL OR l.severity_number >= ?1) \
               AND (?2 IS NULL OR l.target = ?2) \
               AND (?3 IS NULL OR l.body LIKE '%' || ?3 || '%') \
               AND (?4 IS NULL OR l.trace_id = ?4) \
             ORDER BY l.timestamp DESC \
             LIMIT ?5",
        )
        .bind(min_severity)
        .bind(target_filter)
        .bind(search)
        .bind(trace_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::row_to_log).collect()
    }

    /// Return recent logs scoped to a specific assistant agent.
    pub async fn list_recent_for_agent(
        &self,
        limit: i64,
        min_severity: Option<i32>,
        target_filter: Option<&str>,
        search: Option<&str>,
        trace_id: Option<&str>,
        agent_id: &str,
    ) -> Result<Vec<RecordedLog>> {
        let rows = sqlx::query(
            "SELECT l.id, l.timestamp, l.observed_timestamp, l.severity_number, l.severity_text, \
                    l.body, l.trace_id, l.span_id, dt.name AS span_name, l.service_name, l.target, l.attributes \
             FROM logs l \
             LEFT JOIN distributed_traces dt ON dt.span_id = l.span_id AND dt.trace_id = l.trace_id \
             WHERE (?1 IS NULL OR l.severity_number >= ?1) \
               AND (?2 IS NULL OR l.target = ?2) \
               AND (?3 IS NULL OR l.body LIKE '%' || ?3 || '%') \
               AND (?4 IS NULL OR l.trace_id = ?4) \
               AND EXISTS ( \
                   SELECT 1 \
                   FROM distributed_traces dt \
                   INNER JOIN conversations c ON c.id = dt.conversation_id \
                   WHERE dt.trace_id = l.trace_id AND c.agent_id = ?5 \
               ) \
             ORDER BY l.timestamp DESC \
             LIMIT ?6",
        )
        .bind(min_severity)
        .bind(target_filter)
        .bind(search)
        .bind(trace_id)
        .bind(agent_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::row_to_log).collect()
    }

    /// Return all log records for a specific trace, ordered by time.
    pub async fn list_by_trace(&self, trace_id: &str) -> Result<Vec<RecordedLog>> {
        let rows = sqlx::query(
            "SELECT l.id, l.timestamp, l.observed_timestamp, l.severity_number, l.severity_text, \
                    l.body, l.trace_id, l.span_id, dt.name AS span_name, l.service_name, l.target, l.attributes \
             FROM logs l \
             LEFT JOIN distributed_traces dt ON dt.span_id = l.span_id AND dt.trace_id = l.trace_id \
             WHERE l.trace_id = ?1 \
             ORDER BY l.timestamp ASC",
        )
        .bind(trace_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::row_to_log).collect()
    }

    /// Fetch a single log record by ID.
    pub async fn get_log(&self, id: &str) -> Result<Option<RecordedLog>> {
        let row = sqlx::query(
            "SELECT l.id, l.timestamp, l.observed_timestamp, l.severity_number, l.severity_text, \
                    l.body, l.trace_id, l.span_id, dt.name AS span_name, l.service_name, l.target, l.attributes \
             FROM logs l \
             LEFT JOIN distributed_traces dt ON dt.span_id = l.span_id AND dt.trace_id = l.trace_id \
             WHERE l.id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Self::row_to_log).transpose()
    }

    /// Fetch one log record constrained to a specific assistant agent.
    pub async fn get_log_for_agent(&self, id: &str, agent_id: &str) -> Result<Option<RecordedLog>> {
        let row = sqlx::query(
            "SELECT l.id, l.timestamp, l.observed_timestamp, l.severity_number, l.severity_text, \
                    l.body, l.trace_id, l.span_id, dt.name AS span_name, l.service_name, l.target, l.attributes \
             FROM logs l \
             LEFT JOIN distributed_traces dt ON dt.span_id = l.span_id AND dt.trace_id = l.trace_id \
             WHERE l.id = ?1 \
               AND EXISTS ( \
                   SELECT 1 \
                   FROM distributed_traces dt \
                   INNER JOIN conversations c ON c.id = dt.conversation_id \
                   WHERE dt.trace_id = l.trace_id AND c.agent_id = ?2 \
               )",
        )
        .bind(id)
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Self::row_to_log).transpose()
    }

    /// List distinct targets that have recorded logs.
    pub async fn list_targets(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT target \
             FROM logs \
             WHERE target IS NOT NULL \
             ORDER BY target",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|r| r.try_get::<Option<String>, _>("target").ok().flatten())
            .collect())
    }

    /// List distinct targets with logs belonging to one assistant agent.
    pub async fn list_targets_for_agent(&self, agent_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT l.target \
             FROM logs l \
             WHERE l.target IS NOT NULL \
               AND EXISTS ( \
                   SELECT 1 \
                   FROM distributed_traces dt \
                   INNER JOIN conversations c ON c.id = dt.conversation_id \
                   WHERE dt.trace_id = l.trace_id AND c.agent_id = ?1 \
               ) \
             ORDER BY l.target",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|r| r.try_get::<Option<String>, _>("target").ok().flatten())
            .collect())
    }

    /// Compute aggregate log statistics.
    pub async fn stats(&self) -> Result<LogStats> {
        let row = sqlx::query(
            "SELECT \
                COUNT(*) AS total, \
                SUM(CASE WHEN severity_number BETWEEN 1 AND 4 THEN 1 ELSE 0 END) AS trace_count, \
                SUM(CASE WHEN severity_number BETWEEN 5 AND 8 THEN 1 ELSE 0 END) AS debug_count, \
                SUM(CASE WHEN severity_number BETWEEN 9 AND 12 THEN 1 ELSE 0 END) AS info_count, \
                SUM(CASE WHEN severity_number BETWEEN 13 AND 16 THEN 1 ELSE 0 END) AS warn_count, \
                SUM(CASE WHEN severity_number BETWEEN 17 AND 20 THEN 1 ELSE 0 END) AS error_count, \
                SUM(CASE WHEN severity_number >= 21 THEN 1 ELSE 0 END) AS fatal_count \
             FROM logs",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(LogStats {
            total: row.try_get("total").unwrap_or(0),
            trace_count: row.try_get("trace_count").unwrap_or(0),
            debug_count: row.try_get("debug_count").unwrap_or(0),
            info_count: row.try_get("info_count").unwrap_or(0),
            warn_count: row.try_get("warn_count").unwrap_or(0),
            error_count: row.try_get("error_count").unwrap_or(0),
            fatal_count: row.try_get("fatal_count").unwrap_or(0),
        })
    }

    /// Compute aggregate log statistics for one assistant agent.
    pub async fn stats_for_agent(&self, agent_id: &str) -> Result<LogStats> {
        let row = sqlx::query(
            "SELECT \
                COUNT(*) AS total, \
                SUM(CASE WHEN l.severity_number BETWEEN 1 AND 4 THEN 1 ELSE 0 END) AS trace_count, \
                SUM(CASE WHEN l.severity_number BETWEEN 5 AND 8 THEN 1 ELSE 0 END) AS debug_count, \
                SUM(CASE WHEN l.severity_number BETWEEN 9 AND 12 THEN 1 ELSE 0 END) AS info_count, \
                SUM(CASE WHEN l.severity_number BETWEEN 13 AND 16 THEN 1 ELSE 0 END) AS warn_count, \
                SUM(CASE WHEN l.severity_number BETWEEN 17 AND 20 THEN 1 ELSE 0 END) AS error_count, \
                SUM(CASE WHEN l.severity_number >= 21 THEN 1 ELSE 0 END) AS fatal_count \
             FROM logs l \
             WHERE EXISTS ( \
                 SELECT 1 \
                 FROM distributed_traces dt \
                 INNER JOIN conversations c ON c.id = dt.conversation_id \
                 WHERE dt.trace_id = l.trace_id AND c.agent_id = ?1 \
             )",
        )
        .bind(agent_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(LogStats {
            total: row.try_get("total").unwrap_or(0),
            trace_count: row.try_get("trace_count").unwrap_or(0),
            debug_count: row.try_get("debug_count").unwrap_or(0),
            info_count: row.try_get("info_count").unwrap_or(0),
            warn_count: row.try_get("warn_count").unwrap_or(0),
            error_count: row.try_get("error_count").unwrap_or(0),
            fatal_count: row.try_get("fatal_count").unwrap_or(0),
        })
    }

    fn row_to_log(row: SqliteRow) -> Result<RecordedLog> {
        let attrs_str: String = row.get("attributes");
        let attributes: Value = serde_json::from_str(&attrs_str)?;

        Ok(RecordedLog {
            id: row.get("id"),
            timestamp: row.get("timestamp"),
            observed_timestamp: row
                .try_get::<Option<DateTime<Utc>>, _>("observed_timestamp")
                .ok()
                .flatten(),
            severity_number: row
                .try_get::<Option<i32>, _>("severity_number")
                .ok()
                .flatten(),
            severity_text: row
                .try_get::<Option<String>, _>("severity_text")
                .ok()
                .flatten(),
            body: row.try_get::<Option<String>, _>("body").ok().flatten(),
            trace_id: row.try_get::<Option<String>, _>("trace_id").ok().flatten(),
            span_id: row.try_get::<Option<String>, _>("span_id").ok().flatten(),
            span_name: row.try_get::<Option<String>, _>("span_name").ok().flatten(),
            service_name: row
                .try_get::<Option<String>, _>("service_name")
                .ok()
                .flatten(),
            target: row.try_get::<Option<String>, _>("target").ok().flatten(),
            attributes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    #[tokio::test]
    async fn test_insert_and_query_logs() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.log_store();

        insert_log(
            &storage.pool,
            "test-1",
            9,
            "INFO",
            "Hello world",
            None,
            Some("assistant_runtime"),
        )
        .await;

        let recent = store.list_recent(10, None, None, None, None).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].body.as_deref(), Some("Hello world"));
        assert_eq!(recent[0].severity_number, Some(9));
    }

    #[tokio::test]
    async fn test_severity_filter() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.log_store();

        insert_log(&storage.pool, "l1", 5, "DEBUG", "debug msg", None, None).await;
        insert_log(&storage.pool, "l2", 9, "INFO", "info msg", None, None).await;
        insert_log(&storage.pool, "l3", 17, "ERROR", "error msg", None, None).await;

        // Only WARN+ (13+)
        let logs = store
            .list_recent(10, Some(13), None, None, None)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].severity_text.as_deref(), Some("ERROR"));
    }

    #[tokio::test]
    async fn test_trace_correlation() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.log_store();

        let tid = "abc123trace";
        insert_log(&storage.pool, "l1", 9, "INFO", "msg1", Some(tid), None).await;
        insert_log(&storage.pool, "l2", 9, "INFO", "msg2", Some(tid), None).await;
        insert_log(&storage.pool, "l3", 9, "INFO", "other", None, None).await;

        let trace_logs = store.list_by_trace(tid).await.unwrap();
        assert_eq!(trace_logs.len(), 2);
    }

    #[tokio::test]
    async fn test_log_stats() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.log_store();

        insert_log(&storage.pool, "l1", 5, "DEBUG", "a", None, None).await;
        insert_log(&storage.pool, "l2", 9, "INFO", "b", None, None).await;
        insert_log(&storage.pool, "l3", 9, "INFO", "c", None, None).await;
        insert_log(&storage.pool, "l4", 13, "WARN", "d", None, None).await;
        insert_log(&storage.pool, "l5", 17, "ERROR", "e", None, None).await;

        let stats = store.stats().await.unwrap();
        assert_eq!(stats.total, 5);
        assert_eq!(stats.debug_count, 1);
        assert_eq!(stats.info_count, 2);
        assert_eq!(stats.warn_count, 1);
        assert_eq!(stats.error_count, 1);
    }

    async fn insert_log(
        pool: &SqlitePool,
        id: &str,
        severity_number: i32,
        severity_text: &str,
        body: &str,
        trace_id: Option<&str>,
        target: Option<&str>,
    ) {
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO logs \
                (id, timestamp, severity_number, severity_text, body, trace_id, target, attributes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '{}')",
        )
        .bind(id)
        .bind(now)
        .bind(severity_number)
        .bind(severity_text)
        .bind(body)
        .bind(trace_id)
        .bind(target)
        .execute(pool)
        .await
        .unwrap();
    }
}
