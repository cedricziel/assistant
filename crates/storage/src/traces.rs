//! Execution trace storage backed by the `execution_traces` SQLite table.

use anyhow::Result;
use assistant_core::ExecutionTrace;
use chrono::{DateTime, Utc};
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

/// SQLite-backed store for execution traces.
pub struct TraceStore {
    pool: SqlitePool,
}

impl TraceStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Persist an execution trace record.
    pub async fn insert(&self, trace: &ExecutionTrace) -> Result<()> {
        let id = trace.id.to_string();
        let conversation_id = trace.conversation_id.to_string();
        let action_params = serde_json::to_string(&trace.action_params)?;
        let created_at = trace.created_at;

        sqlx::query(
            "INSERT INTO execution_traces \
                (id, conversation_id, turn, action_skill, action_params, \
                 observation, error, duration_ms, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(id)
        .bind(conversation_id)
        .bind(trace.turn)
        .bind(&trace.action_skill)
        .bind(action_params)
        .bind(&trace.observation)
        .bind(&trace.error)
        .bind(trace.duration_ms)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Return the `limit` most-recent traces for the given skill name.
    pub async fn get_recent_for_skill(
        &self,
        skill_name: &str,
        limit: i64,
    ) -> Result<Vec<ExecutionTrace>> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, turn, action_skill, action_params, \
                    observation, error, duration_ms, created_at \
             FROM execution_traces \
             WHERE action_skill = ?1 \
             ORDER BY created_at DESC \
             LIMIT ?2",
        )
        .bind(skill_name)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let id_str: String = r.get("id");
                let conv_str: String = r.get("conversation_id");
                let id = Uuid::parse_str(&id_str)?;
                let conversation_id = Uuid::parse_str(&conv_str)?;
                let params_str: String = r.get("action_params");
                let action_params = serde_json::from_str(&params_str)?;
                let created_at: DateTime<Utc> = r.get("created_at");

                Ok(ExecutionTrace {
                    id,
                    conversation_id,
                    turn: r.get("turn"),
                    action_skill: r.get("action_skill"),
                    action_params,
                    observation: r.get("observation"),
                    error: r.get("error"),
                    duration_ms: r.get("duration_ms"),
                    created_at,
                })
            })
            .collect()
    }

    /// Return the `limit` most-recent traces across all skills.
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<ExecutionTrace>> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, turn, action_skill, action_params, \
                    observation, error, duration_ms, created_at \
             FROM execution_traces \
             ORDER BY created_at DESC \
             LIMIT ?1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let id_str: String = r.get("id");
                let conv_str: String = r.get("conversation_id");
                let id = Uuid::parse_str(&id_str)?;
                let conversation_id = Uuid::parse_str(&conv_str)?;
                let params_str: String = r.get("action_params");
                let action_params = serde_json::from_str(&params_str)?;
                let created_at: DateTime<Utc> = r.get("created_at");

                Ok(ExecutionTrace {
                    id,
                    conversation_id,
                    turn: r.get("turn"),
                    action_skill: r.get("action_skill"),
                    action_params,
                    observation: r.get("observation"),
                    error: r.get("error"),
                    duration_ms: r.get("duration_ms"),
                    created_at,
                })
            })
            .collect()
    }

    /// List distinct skill names that have recorded traces.
    pub async fn list_skills(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT action_skill \
             FROM execution_traces \
             ORDER BY action_skill",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|r| r.try_get::<String, _>("action_skill").ok())
            .collect())
    }

    /// Compute aggregate statistics over the most-recent `window` traces for a skill.
    pub async fn stats_for_skill(&self, skill_name: &str, window: i64) -> Result<TraceStats> {
        // Aggregate over the newest `window` rows for this skill.
        let agg_row = sqlx::query(
            "WITH recent AS ( \
                SELECT error, duration_ms \
                FROM execution_traces \
                WHERE action_skill = ?1 \
                ORDER BY created_at DESC \
                LIMIT ?2 \
            ) \
            SELECT \
                COUNT(*)                                           AS total, \
                SUM(CASE WHEN error IS NULL     THEN 1 ELSE 0 END) AS success_count, \
                SUM(CASE WHEN error IS NOT NULL THEN 1 ELSE 0 END) AS error_count, \
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
                SELECT error \
                FROM execution_traces \
                WHERE action_skill = ?1 \
                  AND error IS NOT NULL \
                ORDER BY created_at DESC \
                LIMIT ?2 \
            ) \
            SELECT error \
            FROM recent \
            GROUP BY error \
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

        let trace = ExecutionTrace::new(
            conv_id,
            1,
            "web-fetch",
            json!({"url": "https://example.com"}),
        )
        .with_success("200 OK", 120);

        store.insert(&trace).await.unwrap();

        let recent = store.get_recent_for_skill("web-fetch", 10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].action_skill, "web-fetch");
        assert_eq!(recent[0].duration_ms, 120);
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

        // 2 successes, 1 failure
        for i in 0..2_i64 {
            let t = ExecutionTrace::new(conv_id, i, "bash", json!({})).with_success("ok", 100);
            store.insert(&t).await.unwrap();
        }
        let failed =
            ExecutionTrace::new(conv_id, 3, "bash", json!({})).with_error("permission denied", 50);
        store.insert(&failed).await.unwrap();

        let stats = store.stats_for_skill("bash", 100).await.unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.error_count, 1);
        assert!(!stats.common_errors.is_empty());
    }
}
