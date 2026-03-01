//! Query API for persisted metrics, powering the analytics dashboard.

use anyhow::Result;
use sqlx::SqlitePool;

/// Query API for the `metric_points` table and its join tables.
pub struct MetricsStore {
    pool: SqlitePool,
}

/// High-level summary for the analytics overview.
pub struct MetricsSummary {
    pub total_tokens_in: i64,
    pub total_tokens_out: i64,
    pub total_requests: i64,
    pub total_tool_invocations: i64,
    pub avg_duration_s: f64,
    pub error_count: i64,
    pub unique_models: Vec<String>,
}

/// A single point in a time series.
pub struct TimeSeriesPoint {
    pub bucket: String,
    pub value: f64,
}

/// Token usage breakdown per model.
pub struct ModelTokenUsage {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: i64,
    pub avg_duration_s: f64,
}

/// Tool invocation statistics.
pub struct ToolUsageStats {
    pub tool_name: String,
    pub invocations: i64,
    pub avg_duration_s: f64,
}

/// Resource identity stored in the `resources` table.
pub struct ResourceRecord {
    pub id: i64,
    pub fingerprint: String,
    pub attributes: String,
}

impl MetricsStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Overall summary for the last `window_hours` hours.
    pub async fn summary(&self, window_hours: i64) -> Result<MetricsSummary> {
        let window = format!("-{window_hours} hours");

        // Token counts from gen_ai.client.token.usage histogram.
        // For histograms the `sum` column holds the total value of all
        // observations in the collection interval.
        let token_in: f64 = sqlx::query_scalar(
            "SELECT CAST(COALESCE(SUM(sum), 0) AS REAL) FROM metric_points \
             WHERE metric_name = 'gen_ai.client.token.usage' \
               AND json_extract(attributes, '$.\"gen_ai.token.type\"') = 'input' \
               AND recorded_at >= datetime('now', ?1)",
        )
        .bind(&window)
        .fetch_one(&self.pool)
        .await?;

        let token_out: f64 = sqlx::query_scalar(
            "SELECT CAST(COALESCE(SUM(sum), 0) AS REAL) FROM metric_points \
             WHERE metric_name = 'gen_ai.client.token.usage' \
               AND json_extract(attributes, '$.\"gen_ai.token.type\"') = 'output' \
               AND recorded_at >= datetime('now', ?1)",
        )
        .bind(&window)
        .fetch_one(&self.pool)
        .await?;

        // Request count (counter).
        let requests: f64 = sqlx::query_scalar(
            "SELECT CAST(COALESCE(SUM(value), 0) AS REAL) FROM metric_points \
             WHERE metric_name = 'assistant.turn.count' \
               AND recorded_at >= datetime('now', ?1)",
        )
        .bind(&window)
        .fetch_one(&self.pool)
        .await?;

        // Tool invocations (counter).
        let tools: f64 = sqlx::query_scalar(
            "SELECT CAST(COALESCE(SUM(value), 0) AS REAL) FROM metric_points \
             WHERE metric_name = 'assistant.tool.invocations' \
               AND recorded_at >= datetime('now', ?1)",
        )
        .bind(&window)
        .fetch_one(&self.pool)
        .await?;

        // Weighted-average operation duration (histogram sum / count).
        let avg_dur: f64 = sqlx::query_scalar(
            "SELECT CAST(COALESCE( \
                 CASE WHEN SUM(count) > 0 THEN SUM(sum) / SUM(count) ELSE 0.0 END, 0) AS REAL) \
             FROM metric_points \
             WHERE metric_name = 'gen_ai.client.operation.duration' \
               AND recorded_at >= datetime('now', ?1)",
        )
        .bind(&window)
        .fetch_one(&self.pool)
        .await?;

        // Error count (counter).
        let errors: f64 = sqlx::query_scalar(
            "SELECT CAST(COALESCE(SUM(value), 0) AS REAL) FROM metric_points \
             WHERE metric_name = 'assistant.error.count' \
               AND recorded_at >= datetime('now', ?1)",
        )
        .bind(&window)
        .fetch_one(&self.pool)
        .await?;

        // Unique models seen.
        let models: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT model FROM metric_points \
             WHERE model IS NOT NULL \
               AND recorded_at >= datetime('now', ?1)",
        )
        .bind(&window)
        .fetch_all(&self.pool)
        .await?;

        Ok(MetricsSummary {
            total_tokens_in: token_in as i64,
            total_tokens_out: token_out as i64,
            total_requests: requests as i64,
            total_tool_invocations: tools as i64,
            avg_duration_s: avg_dur,
            error_count: errors as i64,
            unique_models: models.into_iter().map(|r| r.0).collect(),
        })
    }

    /// Token-usage time series grouped into fixed-width time buckets.
    pub async fn token_usage_over_time(
        &self,
        window_hours: i64,
        bucket_minutes: i64,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT \
                 strftime('%Y-%m-%dT%H:', recorded_at) || \
                 printf('%02d', (CAST(strftime('%M', recorded_at) AS INTEGER) / ?1) * ?1) \
                 || ':00Z' AS bucket, \
                 CAST(COALESCE(SUM(sum), 0) AS REAL) AS total \
             FROM metric_points \
             WHERE metric_name = 'gen_ai.client.token.usage' \
               AND recorded_at >= datetime('now', ?2) \
             GROUP BY bucket \
             ORDER BY bucket",
        )
        .bind(bucket_minutes)
        .bind(format!("-{window_hours} hours"))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(bucket, value)| TimeSeriesPoint { bucket, value })
            .collect())
    }

    /// Per-model token-usage breakdown.
    pub async fn model_comparison(&self, window_hours: i64) -> Result<Vec<ModelTokenUsage>> {
        let rows: Vec<(String, f64, f64, f64, f64)> = sqlx::query_as(
            "SELECT \
                 COALESCE(model, 'unknown') AS m, \
                 CAST(COALESCE(SUM(CASE WHEN json_extract(attributes, '$.\"gen_ai.token.type\"') = 'input' \
                     THEN sum ELSE 0 END), 0) AS REAL) AS input_tok, \
                 CAST(COALESCE(SUM(CASE WHEN json_extract(attributes, '$.\"gen_ai.token.type\"') = 'output' \
                     THEN sum ELSE 0 END), 0) AS REAL) AS output_tok, \
                 CAST(COALESCE(SUM(count), 0) AS REAL) AS req_count, \
                 0.0 AS avg_dur \
             FROM metric_points \
             WHERE metric_name = 'gen_ai.client.token.usage' \
               AND recorded_at >= datetime('now', ?1) \
             GROUP BY m \
             ORDER BY input_tok + output_tok DESC",
        )
        .bind(format!("-{window_hours} hours"))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(model, input, output, count, dur)| ModelTokenUsage {
                model,
                input_tokens: input as i64,
                output_tokens: output as i64,
                request_count: count as i64,
                avg_duration_s: dur,
            })
            .collect())
    }

    /// Tool-usage stats for the operational dashboard.
    pub async fn tool_usage(&self, window_hours: i64) -> Result<Vec<ToolUsageStats>> {
        let rows: Vec<(String, f64, f64)> = sqlx::query_as(
            "SELECT \
                 COALESCE(json_extract(attributes, '$.\"tool.name\"'), 'unknown') AS tn, \
                 CAST(COALESCE(SUM(value), 0) AS REAL) AS invocations, \
                 0.0 AS avg_dur \
             FROM metric_points \
             WHERE metric_name = 'assistant.tool.invocations' \
               AND recorded_at >= datetime('now', ?1) \
             GROUP BY tn \
             ORDER BY invocations DESC",
        )
        .bind(format!("-{window_hours} hours"))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(tool_name, invocations, avg_dur)| ToolUsageStats {
                tool_name,
                invocations: invocations as i64,
                avg_duration_s: avg_dur,
            })
            .collect())
    }

    /// Request-rate time series (turns per bucket).
    pub async fn request_rate(
        &self,
        window_hours: i64,
        bucket_minutes: i64,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT \
                 strftime('%Y-%m-%dT%H:', recorded_at) || \
                 printf('%02d', (CAST(strftime('%M', recorded_at) AS INTEGER) / ?1) * ?1) \
                 || ':00Z' AS bucket, \
                 CAST(COALESCE(SUM(value), 0) AS REAL) AS total \
             FROM metric_points \
             WHERE metric_name = 'assistant.turn.count' \
               AND recorded_at >= datetime('now', ?2) \
             GROUP BY bucket \
             ORDER BY bucket",
        )
        .bind(bucket_minutes)
        .bind(format!("-{window_hours} hours"))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(bucket, value)| TimeSeriesPoint { bucket, value })
            .collect())
    }

    /// Error-count time series.
    pub async fn error_rate(
        &self,
        window_hours: i64,
        bucket_minutes: i64,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT \
                 strftime('%Y-%m-%dT%H:', recorded_at) || \
                 printf('%02d', (CAST(strftime('%M', recorded_at) AS INTEGER) / ?1) * ?1) \
                 || ':00Z' AS bucket, \
                 CAST(COALESCE(SUM(value), 0) AS REAL) AS total \
             FROM metric_points \
             WHERE metric_name = 'assistant.error.count' \
               AND recorded_at >= datetime('now', ?2) \
             GROUP BY bucket \
             ORDER BY bucket",
        )
        .bind(bucket_minutes)
        .bind(format!("-{window_hours} hours"))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(bucket, value)| TimeSeriesPoint { bucket, value })
            .collect())
    }

    /// List all known resources.
    pub async fn list_resources(&self) -> Result<Vec<ResourceRecord>> {
        let rows: Vec<(i64, String, String)> = sqlx::query_as(
            "SELECT id, fingerprint, attributes FROM resources ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, fingerprint, attributes)| ResourceRecord {
                id,
                fingerprint,
                attributes,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    #[tokio::test]
    async fn summary_returns_zeros_on_empty_db() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = MetricsStore::new(storage.pool.clone());

        let summary = store.summary(24).await.unwrap();
        assert_eq!(summary.total_tokens_in, 0);
        assert_eq!(summary.total_tokens_out, 0);
        assert_eq!(summary.total_requests, 0);
        assert_eq!(summary.total_tool_invocations, 0);
        assert_eq!(summary.error_count, 0);
        assert!(summary.unique_models.is_empty());
    }

    #[tokio::test]
    async fn token_usage_over_time_empty() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = MetricsStore::new(storage.pool.clone());

        let series = store.token_usage_over_time(24, 5).await.unwrap();
        assert!(series.is_empty());
    }

    #[tokio::test]
    async fn list_resources_empty() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = MetricsStore::new(storage.pool.clone());

        let resources = store.list_resources().await.unwrap();
        assert!(resources.is_empty());
    }

    /// Insert FK parent rows and return (resource_id, scope_id).
    async fn seed_parents(pool: &SqlitePool) -> (i64, i64) {
        let rid: (i64,) = sqlx::query_as(
            "INSERT INTO resources (fingerprint, attributes) VALUES ('test-fp', '{}') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        let sid: (i64,) = sqlx::query_as(
            "INSERT INTO metric_scopes (name, version) VALUES ('test', '0.1') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        (rid.0, sid.0)
    }

    /// Regression test: queries must not fail when rows exist and SQLite
    /// returns INTEGER for aggregated columns (SUM of INTEGER column).
    /// See: <https://github.com/cedricziel/assistant/issues/XXX>
    #[tokio::test]
    async fn queries_handle_integer_column_types() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let pool = storage.pool.clone();
        let (rid, sid) = seed_parents(&pool).await;

        // Insert token usage (histogram: sum + count are the key fields).
        sqlx::query(
            "INSERT INTO metric_points \
             (resource_id, scope_id, metric_name, metric_kind, \
              sum, count, attributes, model, recorded_at) \
             VALUES (?1, ?2, 'gen_ai.client.token.usage', 'histogram', \
                     150, 3, '{\"gen_ai.token.type\": \"input\"}', 'test-model', \
                     strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        )
        .bind(rid)
        .bind(sid)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO metric_points \
             (resource_id, scope_id, metric_name, metric_kind, \
              sum, count, attributes, model, recorded_at) \
             VALUES (?1, ?2, 'gen_ai.client.token.usage', 'histogram', \
                     80, 2, '{\"gen_ai.token.type\": \"output\"}', 'test-model', \
                     strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        )
        .bind(rid)
        .bind(sid)
        .execute(&pool)
        .await
        .unwrap();

        // Insert turn count (counter: value column).
        sqlx::query(
            "INSERT INTO metric_points \
             (resource_id, scope_id, metric_name, metric_kind, \
              value, attributes, recorded_at) \
             VALUES (?1, ?2, 'assistant.turn.count', 'counter', \
                     5, '{}', strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        )
        .bind(rid)
        .bind(sid)
        .execute(&pool)
        .await
        .unwrap();

        // Insert tool invocation.
        sqlx::query(
            "INSERT INTO metric_points \
             (resource_id, scope_id, metric_name, metric_kind, \
              value, attributes, recorded_at) \
             VALUES (?1, ?2, 'assistant.tool.invocations', 'counter', \
                     7, '{\"tool.name\": \"file-read\"}', strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        )
        .bind(rid)
        .bind(sid)
        .execute(&pool)
        .await
        .unwrap();

        // Insert operation duration (histogram).
        sqlx::query(
            "INSERT INTO metric_points \
             (resource_id, scope_id, metric_name, metric_kind, \
              sum, count, attributes, recorded_at) \
             VALUES (?1, ?2, 'gen_ai.client.operation.duration', 'histogram', \
                     12, 4, '{}', strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        )
        .bind(rid)
        .bind(sid)
        .execute(&pool)
        .await
        .unwrap();

        let store = MetricsStore::new(pool);

        // All of these previously failed with "mismatched types; Rust type
        // `f64` (as SQL type `REAL`) is not compatible with SQL type `INTEGER`"
        let summary = store.summary(24).await.expect("summary must not fail");
        assert_eq!(summary.total_tokens_in, 150);
        assert_eq!(summary.total_tokens_out, 80);
        assert_eq!(summary.total_requests, 5);
        assert_eq!(summary.total_tool_invocations, 7);
        assert!(summary.avg_duration_s > 0.0, "avg duration should be > 0");

        let models = store
            .model_comparison(24)
            .await
            .expect("model_comparison must not fail");
        assert_eq!(models.len(), 1, "should have one model");
        assert_eq!(models[0].model, "test-model");

        let tools = store
            .tool_usage(24)
            .await
            .expect("tool_usage must not fail");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].tool_name, "file-read");

        let tokens = store
            .token_usage_over_time(24, 15)
            .await
            .expect("token_usage_over_time must not fail");
        assert!(!tokens.is_empty(), "should have at least one bucket");

        let requests = store
            .request_rate(24, 15)
            .await
            .expect("request_rate must not fail");
        assert!(!requests.is_empty(), "should have at least one bucket");
    }
}
