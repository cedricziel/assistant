//! SQLite-backed OpenTelemetry exporters for spans, logs, and metrics.
//!
//! Each exporter persists telemetry data into a local SQLite database using
//! the schema managed by `assistant-storage`.

pub mod log;
pub mod metric;
pub mod span;

pub use log::SqliteLogExporter;
pub use metric::SqliteMetricExporter;
pub use span::SqliteSpanExporter;

/// Test-only helpers — provides an in-memory SQLite pool with the tables
/// the exporters need, sourced from the shared `migrations/` directory via
/// `include_str!`.
#[cfg(test)]
pub(crate) mod test_utils {
    use sqlx::SqlitePool;

    /// Create an in-memory SQLite pool with only the tables the exporters
    /// require.  Migration SQL is pulled from the workspace `migrations/`
    /// directory at compile time so there is a single source of truth.
    pub async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query("PRAGMA journal_mode=WAL;")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("PRAGMA busy_timeout = 5000;")
            .execute(&pool)
            .await
            .unwrap();

        // Stub tables referenced by migration 009 (legacy data migration).
        // The SELECT from execution_traces is a no-op on an empty table, and
        // the FK on conversations is satisfied by the stub.
        sqlx::query("CREATE TABLE IF NOT EXISTS conversations (id TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS execution_traces (
                id TEXT PRIMARY KEY, conversation_id TEXT, turn INTEGER,
                action_skill TEXT, error TEXT, observation TEXT,
                duration_ms INTEGER DEFAULT 0, created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                action_params TEXT DEFAULT '{}'
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // 009 — distributed_traces (spans)
        sqlx::raw_sql(include_str!(
            "../../../migrations/009_distributed_traces.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        // 010 — add token-usage columns to distributed_traces
        sqlx::raw_sql(include_str!(
            "../../../migrations/010_trace_token_usage.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        // 011 — logs
        sqlx::raw_sql(include_str!("../../../migrations/011_logs.sql"))
            .execute(&pool)
            .await
            .unwrap();
        // 015 — metrics (resources, metric_scopes, metric_points)
        sqlx::raw_sql(include_str!("../../../migrations/015_metrics.sql"))
            .execute(&pool)
            .await
            .unwrap();

        pool
    }
}
