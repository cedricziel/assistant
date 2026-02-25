//! Distributed trace storage backed by OpenTelemetry spans persisted in SQLite.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Summary statistics about a skill's past executions.
#[derive(Debug, Clone)]
pub struct TraceStats {
    pub skill_name: String,
    pub total: i64,
    pub success_count: i64,
    pub error_count: i64,
    pub avg_duration_ms: f64,
    /// Up to 5 most-frequent error messages observed in the window.
    pub common_errors: Vec<String>,
}

/// A persisted OpenTelemetry span row enriched with tool metadata.
#[derive(Debug, Clone)]
pub struct RecordedSpan {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub conversation_id: Option<Uuid>,
    pub turn: Option<i64>,
    pub tool_name: Option<String>,
    pub tool_status: Option<String>,
    pub observation: Option<String>,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub attributes: Value,
}

/// SQLite-backed store for execution traces.
pub struct TraceStore {
    pool: SqlitePool,
}

impl TraceStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Return the `limit` most-recent traces for the given skill name.
    pub async fn get_recent_for_skill(
        &self,
        skill_name: &str,
        limit: i64,
    ) -> Result<Vec<RecordedSpan>> {
        let rows = sqlx::query(
            "SELECT span_id, trace_id, parent_span_id, name, conversation_id, turn, \
                    tool_name, tool_status, tool_observation, tool_error, duration_ms, \
                    start_time, end_time, attributes \
             FROM distributed_traces \
             WHERE tool_name = ?1 \
             ORDER BY start_time DESC \
             LIMIT ?2",
        )
        .bind(skill_name)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::row_to_span).collect()
    }

    /// Return the `limit` most-recent traces across all skills.
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<RecordedSpan>> {
        let rows = sqlx::query(
            "SELECT span_id, trace_id, parent_span_id, name, conversation_id, turn, \
                    tool_name, tool_status, tool_observation, tool_error, duration_ms, \
                    start_time, end_time, attributes \
             FROM distributed_traces \
             WHERE tool_name IS NOT NULL \
             ORDER BY start_time DESC \
             LIMIT ?1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::row_to_span).collect()
    }

    /// List distinct skill names that have recorded traces.
    pub async fn list_skills(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT tool_name \
             FROM distributed_traces \
             WHERE tool_name IS NOT NULL \
             ORDER BY tool_name",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|r| r.try_get::<Option<String>, _>("tool_name").ok().flatten())
            .collect())
    }

    /// Compute aggregate statistics over the most-recent `window` traces for a skill.
    pub async fn stats_for_skill(&self, skill_name: &str, window: i64) -> Result<TraceStats> {
        // Aggregate over the newest `window` rows for this skill.
        let agg_row = sqlx::query(
            "WITH recent AS ( \
                SELECT tool_status, duration_ms \
                FROM distributed_traces \
                WHERE tool_name = ?1 \
                ORDER BY start_time DESC \
                LIMIT ?2 \
            ) \
            SELECT \
                COUNT(*)                                           AS total, \
                SUM(CASE WHEN tool_status = 'error' THEN 0 ELSE 1 END) AS success_count, \
                SUM(CASE WHEN tool_status = 'error' THEN 1 ELSE 0 END) AS error_count, \
                COALESCE(AVG(CAST(duration_ms AS REAL)), 0.0)      AS avg_duration_ms \
            FROM recent",
        )
        .bind(skill_name)
        .bind(window)
        .fetch_one(&self.pool)
        .await?;

        let total: i64 = agg_row.try_get("total").unwrap_or(0);
        let success_count: i64 = agg_row.try_get("success_count").unwrap_or(0);
        let error_count: i64 = agg_row.try_get("error_count").unwrap_or(0);
        let avg_duration_ms: f64 = agg_row.try_get("avg_duration_ms").unwrap_or(0.0);

        // Collect the most common error strings (up to 5).
        let err_rows = sqlx::query(
            "WITH recent AS ( \
                SELECT tool_error \
                FROM distributed_traces \
                WHERE tool_name = ?1 \
                  AND tool_error IS NOT NULL \
                ORDER BY start_time DESC \
                LIMIT ?2 \
            ) \
            SELECT tool_error AS error \
            FROM recent \
            GROUP BY tool_error \
            ORDER BY COUNT(*) DESC \
            LIMIT 5",
        )
        .bind(skill_name)
        .bind(window)
        .fetch_all(&self.pool)
        .await?;

        let common_errors: Vec<String> = err_rows
            .into_iter()
            .filter_map(|r| r.try_get::<Option<String>, _>("error").ok().flatten())
            .collect();

        Ok(TraceStats {
            skill_name: skill_name.to_string(),
            total,
            success_count,
            error_count,
            avg_duration_ms,
            common_errors,
        })
    }

    fn row_to_span(row: SqliteRow) -> Result<RecordedSpan> {
        let conv_raw: Option<String> = row.try_get("conversation_id").ok().flatten();
        let conversation_id = match conv_raw {
            Some(ref raw) if !raw.is_empty() => Some(Uuid::parse_str(raw)?),
            _ => None,
        };

        let attrs_str: String = row.get("attributes");
        let attributes: Value = serde_json::from_str(&attrs_str)?;

        Ok(RecordedSpan {
            span_id: row.get("span_id"),
            trace_id: row.get("trace_id"),
            parent_span_id: row
                .try_get::<Option<String>, _>("parent_span_id")
                .ok()
                .flatten(),
            name: row.get("name"),
            conversation_id,
            turn: row.try_get::<Option<i64>, _>("turn").ok().flatten(),
            tool_name: row.try_get::<Option<String>, _>("tool_name").ok().flatten(),
            tool_status: row
                .try_get::<Option<String>, _>("tool_status")
                .ok()
                .flatten(),
            observation: row
                .try_get::<Option<String>, _>("tool_observation")
                .ok()
                .flatten(),
            error: row
                .try_get::<Option<String>, _>("tool_error")
                .ok()
                .flatten(),
            duration_ms: row.get("duration_ms"),
            start_time: row.get("start_time"),
            end_time: row.get("end_time"),
            attributes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_insert_and_query() {
        let storage = StorageLayer::new_in_memory().await.unwrap();

        // Insert a conversation row first to satisfy FK
        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("test")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        insert_span(
            &storage.pool,
            conv_id,
            "web-fetch",
            "ok",
            Some("200 OK"),
            None,
            120,
        )
        .await;

        let recent = store.get_recent_for_skill("web-fetch", 10).await.unwrap();
        assert_eq!(recent.len(), 1);
        let span = &recent[0];
        assert_eq!(span.tool_name.as_deref(), Some("web-fetch"));
        assert_eq!(span.observation.as_deref(), Some("200 OK"));
        assert_eq!(span.duration_ms, 120);
    }

    #[tokio::test]
    async fn test_stats() {
        let storage = StorageLayer::new_in_memory().await.unwrap();

        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("test")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        insert_span(&storage.pool, conv_id, "bash", "ok", Some("ok"), None, 100).await;
        insert_span(&storage.pool, conv_id, "bash", "ok", Some("ok"), None, 100).await;
        insert_span(
            &storage.pool,
            conv_id,
            "bash",
            "error",
            None,
            Some("permission denied"),
            50,
        )
        .await;

        let stats = store.stats_for_skill("bash", 100).await.unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.error_count, 1);
        assert!(!stats.common_errors.is_empty());
    }

    async fn insert_span(
        pool: &SqlitePool,
        conversation_id: Uuid,
        tool_name: &str,
        status: &str,
        observation: Option<&str>,
        error: Option<&str>,
        duration_ms: i64,
    ) {
        let span_id = Uuid::new_v4().to_string();
        let trace_id = Uuid::new_v4().to_string();
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(duration_ms.max(0));
        let attrs = json!({
            "conversation_id": conversation_id.to_string(),
            "tool_name": tool_name,
            "tool_status": status,
        });

        sqlx::query(
            "INSERT INTO distributed_traces \
                (span_id, trace_id, parent_span_id, name, conversation_id, turn, tool_name, \
                 tool_status, tool_observation, tool_error, duration_ms, start_time, end_time, attributes) \
             VALUES (?1, ?2, NULL, 'tool_execution', ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
        )
        .bind(span_id)
        .bind(trace_id)
        .bind(conversation_id.to_string())
        .bind(tool_name)
        .bind(status)
        .bind(observation)
        .bind(error)
        .bind(duration_ms)
        .bind(start)
        .bind(end)
        .bind(attrs.to_string())
        .execute(pool)
        .await
        .unwrap();
    }
}
