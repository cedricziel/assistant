//! Query backend abstraction for trace and log data.
//!
//! The web-UI can read observability data from different storage backends
//! depending on the configured [`OtelExporter`]:
//!
//! - [`SqliteTraceBackend`] / [`SqliteLogBackend`] — default; reads from the
//!   `distributed_traces` and `logs` SQLite tables.
//! - [`IcebergTraceBackend`] / [`IcebergLogBackend`] — used when
//!   `exporter = "iceberg"`; scans Parquet files written by the Iceberg
//!   exporter directly from the warehouse directory.
//!
//! Handlers receive the active backend via [`AppState`] and call these trait
//! methods instead of constructing `TraceStore`/`LogStore` directly.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use assistant_storage::{LogStats, RecordedLog, RecordedSpan, TraceFilter, TraceSummary};

pub mod iceberg;
pub mod sqlite;

pub use iceberg::{IcebergLogBackend, IcebergTraceBackend};
pub use sqlite::{SqliteLogBackend, SqliteTraceBackend};

// -- TraceBackend -------------------------------------------------------------

/// Read-side interface for distributed trace data.
#[async_trait]
pub trait TraceBackend: Send + Sync {
    /// Return the `limit` most-recent traces, optionally filtered.
    ///
    /// `agent_id` is used by the SQLite backend for per-agent scoping and
    /// ignored by the Iceberg backend.
    async fn list_recent_traces(
        &self,
        limit: i64,
        filter: &TraceFilter,
        agent_id: &str,
    ) -> Result<Vec<TraceSummary>>;

    /// Return all spans belonging to a single trace.
    ///
    /// `agent_id` is used by the SQLite backend for per-agent scoping and
    /// ignored by the Iceberg backend.
    async fn get_trace(&self, trace_id: &str, agent_id: &str) -> Result<Vec<RecordedSpan>>;
}

// -- LogBackend ---------------------------------------------------------------

/// Read-side interface for log record data.
#[async_trait]
pub trait LogBackend: Send + Sync {
    /// Return the `limit` most-recent log records, optionally filtered.
    ///
    /// `agent_id` is used by the SQLite backend for per-agent scoping and
    /// ignored by the Iceberg backend.
    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<Vec<RecordedLog>>;

    /// Look up a single log record by ID.
    async fn get_log(&self, id: &str, agent_id: &str) -> Result<Option<RecordedLog>>;

    /// Aggregate counts by severity level.
    async fn log_stats(&self, agent_id: &str) -> Result<LogStats>;

    /// Return the distinct `target` values present in the log data.
    async fn list_targets(&self, agent_id: &str) -> Result<Vec<String>>;
}
