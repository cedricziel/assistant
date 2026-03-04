//! OpenTelemetry log exporter that persists log records into the `logs` SQLite table.

use chrono::{DateTime, Utc};
use opentelemetry::logs::{AnyValue, Severity};
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::logs::{LogBatch, LogExporter, SdkLogRecord};
use opentelemetry_sdk::Resource;
use serde_json::{Map, Number};
use sqlx::{SqliteConnection, SqlitePool};
use tokio::runtime::Handle;
use uuid::Uuid;

/// OpenTelemetry log exporter that persists log records into the `logs`
/// SQLite table.
///
/// The OTel SDK's `BatchLogProcessor` runs its export loop on a plain OS
/// thread (not a Tokio task), so `sqlx` operations would panic with
/// *"this functionality requires a Tokio context"*.  We capture the Tokio
/// [`Handle`] at construction time and use [`Handle::block_on`] when no
/// Tokio context is available (batch processor thread), falling back to
/// direct async execution when one already exists (e.g. unit tests).
#[derive(Clone, Debug)]
pub struct SqliteLogExporter {
    pool: SqlitePool,
    rt_handle: Handle,
}

impl SqliteLogExporter {
    /// Create a new exporter.
    ///
    /// # Panics
    /// Panics if called outside a Tokio runtime (the handle is captured
    /// via [`Handle::current()`]).
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            rt_handle: Handle::current(),
        }
    }

    /// Inner export logic, always called with a Tokio context available.
    async fn export_inner(&self, batch: LogBatch<'_>) -> OTelSdkResult {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| OTelSdkError::InternalFailure(format!("SQLite begin failed: {e}")))?;

        for (record, _scope) in batch.iter() {
            if let Err(err) = Self::persist_log(&mut tx, record).await {
                return Err(OTelSdkError::InternalFailure(format!(
                    "SQLite log export failed: {err}"
                )));
            }
        }

        tx.commit()
            .await
            .map_err(|e| OTelSdkError::InternalFailure(format!("SQLite commit failed: {e}")))?;

        Ok(())
    }

    async fn persist_log(
        conn: &mut SqliteConnection,
        record: &SdkLogRecord,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4().to_string();

        let timestamp: DateTime<Utc> = record.timestamp().map(Into::into).unwrap_or_else(Utc::now);

        let observed_timestamp: Option<DateTime<Utc>> = record.observed_timestamp().map(Into::into);

        let severity_number = record.severity_number().map(severity_to_i32);

        let severity_text = record.severity_text().map(|s| s.to_string());

        let body = record.body().map(any_value_to_string);

        let (trace_id, span_id) = match record.trace_context() {
            Some(ctx) => (
                Some(ctx.trace_id.to_string()),
                Some(ctx.span_id.to_string()),
            ),
            None => (None, None),
        };

        let target = record.target().map(|s| s.to_string());

        let attrs = attributes_to_json(record);
        let attrs_serialized = attrs.to_string();

        sqlx::query(
            "INSERT INTO logs \
                (id, timestamp, observed_timestamp, severity_number, severity_text, \
                 body, trace_id, span_id, target, attributes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(&id)
        .bind(timestamp)
        .bind(observed_timestamp)
        .bind(severity_number)
        .bind(&severity_text)
        .bind(&body)
        .bind(&trace_id)
        .bind(&span_id)
        .bind(&target)
        .bind(&attrs_serialized)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}

impl LogExporter for SqliteLogExporter {
    async fn export(&self, batch: LogBatch<'_>) -> OTelSdkResult {
        // The BatchLogProcessor calls us from a plain OS thread with no
        // Tokio runtime.  When a Tokio context already exists (tests), we
        // await directly; otherwise we block on the captured Handle.
        if Handle::try_current().is_ok() {
            self.export_inner(batch).await
        } else {
            self.rt_handle.block_on(self.export_inner(batch))
        }
    }

    fn set_resource(&mut self, _resource: &Resource) {
        // Resource metadata is not persisted to the logs table.
    }
}

// -- Helpers --

fn severity_to_i32(severity: Severity) -> i32 {
    match severity {
        Severity::Trace => 1,
        Severity::Trace2 => 2,
        Severity::Trace3 => 3,
        Severity::Trace4 => 4,
        Severity::Debug => 5,
        Severity::Debug2 => 6,
        Severity::Debug3 => 7,
        Severity::Debug4 => 8,
        Severity::Info => 9,
        Severity::Info2 => 10,
        Severity::Info3 => 11,
        Severity::Info4 => 12,
        Severity::Warn => 13,
        Severity::Warn2 => 14,
        Severity::Warn3 => 15,
        Severity::Warn4 => 16,
        Severity::Error => 17,
        Severity::Error2 => 18,
        Severity::Error3 => 19,
        Severity::Error4 => 20,
        Severity::Fatal => 21,
        Severity::Fatal2 => 22,
        Severity::Fatal3 => 23,
        Severity::Fatal4 => 24,
    }
}

fn any_value_to_string(value: &AnyValue) -> String {
    match value {
        AnyValue::Int(v) => v.to_string(),
        AnyValue::Double(v) => v.to_string(),
        AnyValue::String(v) => v.to_string(),
        AnyValue::Boolean(v) => v.to_string(),
        AnyValue::Bytes(v) => format!("{:?}", v),
        AnyValue::ListAny(v) => format!("{:?}", v),
        AnyValue::Map(v) => format!("{:?}", v),
        _ => format!("{value:?}"),
    }
}

fn attributes_to_json(record: &SdkLogRecord) -> serde_json::Value {
    let mut map = Map::new();
    for (key, value) in record.attributes_iter() {
        map.insert(key.as_str().to_string(), any_value_to_json(value));
    }
    serde_json::Value::Object(map)
}

fn any_value_to_json(value: &AnyValue) -> serde_json::Value {
    match value {
        AnyValue::Int(v) => serde_json::Value::Number(Number::from(*v)),
        AnyValue::Double(v) => Number::from_f64(*v)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        AnyValue::String(v) => serde_json::Value::String(v.to_string()),
        AnyValue::Boolean(v) => serde_json::Value::Bool(*v),
        AnyValue::Bytes(v) => serde_json::Value::String(format!("{:?}", v)),
        AnyValue::ListAny(v) => serde_json::Value::Array(v.iter().map(any_value_to_json).collect()),
        AnyValue::Map(v) => {
            let mut map = Map::new();
            for (k, val) in v.iter() {
                map.insert(k.as_str().to_string(), any_value_to_json(val));
            }
            serde_json::Value::Object(map)
        }
        _ => serde_json::Value::String(format!("{value:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::logs::{
        AnyValue, LogRecord as _, Logger as _, LoggerProvider as _, Severity,
    };
    use opentelemetry::trace::{SpanId, TraceFlags, TraceId};
    use opentelemetry::InstrumentationScope;
    use opentelemetry_sdk::logs::{SdkLogRecord, SdkLoggerProvider};

    /// Create a logger from a no-op provider so we can call
    /// `create_log_record()` (which is `pub(crate)` on `SdkLogRecord`).
    fn new_record() -> SdkLogRecord {
        let provider = SdkLoggerProvider::builder().build();
        let logger = provider.logger("test");
        logger.create_log_record()
    }

    fn make_log(
        severity: Severity,
        body: &str,
        target: &str,
    ) -> (SdkLogRecord, InstrumentationScope) {
        let mut record = new_record();
        record.set_severity_number(severity);
        record.set_severity_text(severity_text_for(severity));
        record.set_body(AnyValue::String(body.to_string().into()));
        record.set_timestamp(std::time::SystemTime::now());
        record.set_target(target.to_string());
        (record, InstrumentationScope::default())
    }

    fn make_log_with_trace(
        severity: Severity,
        body: &str,
        trace_id_hex: &str,
        span_id_hex: &str,
    ) -> (SdkLogRecord, InstrumentationScope) {
        let mut record = new_record();
        record.set_severity_number(severity);
        record.set_severity_text("INFO");
        record.set_body(AnyValue::String(body.to_string().into()));
        record.set_timestamp(std::time::SystemTime::now());
        // Set trace context via the LogRecord trait method.
        let trace_id = TraceId::from_hex(trace_id_hex).unwrap();
        let span_id = SpanId::from_hex(span_id_hex).unwrap();
        record.set_trace_context(trace_id, span_id, Some(TraceFlags::SAMPLED));
        (record, InstrumentationScope::default())
    }

    fn severity_text_for(s: Severity) -> &'static str {
        match s {
            Severity::Debug | Severity::Debug2 | Severity::Debug3 | Severity::Debug4 => "DEBUG",
            Severity::Info | Severity::Info2 | Severity::Info3 | Severity::Info4 => "INFO",
            Severity::Warn | Severity::Warn2 | Severity::Warn3 | Severity::Warn4 => "WARN",
            Severity::Error | Severity::Error2 | Severity::Error3 | Severity::Error4 => "ERROR",
            _ => "TRACE",
        }
    }

    async fn row_count(pool: &SqlitePool, table: &str) -> i64 {
        let q = format!("SELECT COUNT(*) AS cnt FROM {table}");
        sqlx::query_scalar::<_, i64>(&q)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// Convert owned items into the reference-tuple slice that `LogBatch::new` expects.
    fn as_batch_refs(
        items: &[(SdkLogRecord, InstrumentationScope)],
    ) -> Vec<(&SdkLogRecord, &InstrumentationScope)> {
        items.iter().map(|(r, s)| (r, s)).collect()
    }

    #[tokio::test]
    async fn test_export_persists_exact_count() {
        let pool = crate::test_utils::test_pool().await;
        let exporter = SqliteLogExporter::new(pool.clone());

        let items: Vec<(SdkLogRecord, InstrumentationScope)> = vec![
            make_log(Severity::Info, "msg-1", "app"),
            make_log(Severity::Warn, "msg-2", "app"),
            make_log(Severity::Error, "msg-3", "app"),
        ];
        let refs = as_batch_refs(&items);
        let batch = LogBatch::new(&refs);

        exporter.export(batch).await.unwrap();

        let count = row_count(&pool, "logs").await;
        assert_eq!(count, 3, "exporter must produce exactly one row per record");
    }

    #[tokio::test]
    async fn test_export_preserves_severity_and_body() {
        let pool = crate::test_utils::test_pool().await;
        let exporter = SqliteLogExporter::new(pool.clone());

        let items: Vec<(SdkLogRecord, InstrumentationScope)> = vec![make_log(
            Severity::Warn,
            "disk almost full",
            "infra::monitor",
        )];
        let refs = as_batch_refs(&items);
        let batch = LogBatch::new(&refs);
        exporter.export(batch).await.unwrap();

        let row =
            sqlx::query_as::<_, (Option<i32>, Option<String>, Option<String>, Option<String>)>(
                "SELECT severity_number, severity_text, body, target FROM logs LIMIT 1",
            )
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row.0, Some(13), "Severity::Warn maps to 13");
        assert_eq!(row.1.as_deref(), Some("WARN"));
        assert_eq!(row.2.as_deref(), Some("disk almost full"));
        assert_eq!(row.3.as_deref(), Some("infra::monitor"));
    }

    #[tokio::test]
    async fn test_export_preserves_trace_context() {
        let pool = crate::test_utils::test_pool().await;
        let exporter = SqliteLogExporter::new(pool.clone());

        let items: Vec<(SdkLogRecord, InstrumentationScope)> = vec![make_log_with_trace(
            Severity::Info,
            "correlated",
            "0102030405060708090a0b0c0d0e0f10",
            "f1e2d3c4b5a69788",
        )];
        let refs = as_batch_refs(&items);
        let batch = LogBatch::new(&refs);
        exporter.export(batch).await.unwrap();

        let row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
            "SELECT trace_id, span_id FROM logs LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0.as_deref(), Some("0102030405060708090a0b0c0d0e0f10"));
        assert_eq!(row.1.as_deref(), Some("f1e2d3c4b5a69788"));
    }

    #[tokio::test]
    async fn test_export_without_trace_context_stores_null() {
        let pool = crate::test_utils::test_pool().await;
        let exporter = SqliteLogExporter::new(pool.clone());

        let items: Vec<(SdkLogRecord, InstrumentationScope)> =
            vec![make_log(Severity::Debug, "no span", "test")];
        let refs = as_batch_refs(&items);
        let batch = LogBatch::new(&refs);
        exporter.export(batch).await.unwrap();

        let row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
            "SELECT trace_id, span_id FROM logs LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(row.0.is_none(), "trace_id should be NULL without context");
        assert!(row.1.is_none(), "span_id should be NULL without context");
    }
}
