//! OpenTelemetry log exporter that persists log records into the `logs` SQLite table.

use chrono::{DateTime, Utc};
use opentelemetry::logs::{AnyValue, Severity};
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::logs::{LogBatch, LogExporter, SdkLogRecord};
use opentelemetry_sdk::Resource;
use serde_json::{Map, Number};
use sqlx::SqlitePool;
use uuid::Uuid;

/// OpenTelemetry log exporter that persists log records into the `logs`
/// SQLite table.
#[derive(Clone, Debug)]
pub struct SqliteLogExporter {
    pool: SqlitePool,
}

impl SqliteLogExporter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn persist_log(pool: SqlitePool, record: &SdkLogRecord) -> Result<(), sqlx::Error> {
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
        .execute(&pool)
        .await?;

        Ok(())
    }
}

impl LogExporter for SqliteLogExporter {
    async fn export(&self, batch: LogBatch<'_>) -> OTelSdkResult {
        for (record, _scope) in batch.iter() {
            if let Err(err) = Self::persist_log(self.pool.clone(), record).await {
                return Err(opentelemetry_sdk::error::OTelSdkError::InternalFailure(
                    format!("SQLite log export failed: {err}"),
                ));
            }
        }
        Ok(())
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
    use crate::StorageLayer;
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
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let exporter = SqliteLogExporter::new(storage.pool.clone());

        let items: Vec<(SdkLogRecord, InstrumentationScope)> = vec![
            make_log(Severity::Info, "msg-1", "app"),
            make_log(Severity::Warn, "msg-2", "app"),
            make_log(Severity::Error, "msg-3", "app"),
        ];
        let refs = as_batch_refs(&items);
        let batch = LogBatch::new(&refs);

        exporter.export(batch).await.unwrap();

        let count = row_count(&storage.pool, "logs").await;
        assert_eq!(count, 3, "exporter must produce exactly one row per record");
    }

    #[tokio::test]
    async fn test_export_preserves_severity_and_body() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let exporter = SqliteLogExporter::new(storage.pool.clone());

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
            .fetch_one(&storage.pool)
            .await
            .unwrap();

        assert_eq!(row.0, Some(13), "Severity::Warn maps to 13");
        assert_eq!(row.1.as_deref(), Some("WARN"));
        assert_eq!(row.2.as_deref(), Some("disk almost full"));
        assert_eq!(row.3.as_deref(), Some("infra::monitor"));
    }

    #[tokio::test]
    async fn test_export_preserves_trace_context() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let exporter = SqliteLogExporter::new(storage.pool.clone());

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
        .fetch_one(&storage.pool)
        .await
        .unwrap();

        assert_eq!(row.0.as_deref(), Some("0102030405060708090a0b0c0d0e0f10"));
        assert_eq!(row.1.as_deref(), Some("f1e2d3c4b5a69788"));
    }

    #[tokio::test]
    async fn test_export_without_trace_context_stores_null() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let exporter = SqliteLogExporter::new(storage.pool.clone());

        let items: Vec<(SdkLogRecord, InstrumentationScope)> =
            vec![make_log(Severity::Debug, "no span", "test")];
        let refs = as_batch_refs(&items);
        let batch = LogBatch::new(&refs);
        exporter.export(batch).await.unwrap();

        let row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
            "SELECT trace_id, span_id FROM logs LIMIT 1",
        )
        .fetch_one(&storage.pool)
        .await
        .unwrap();

        assert!(row.0.is_none(), "trace_id should be NULL without context");
        assert!(row.1.is_none(), "span_id should be NULL without context");
    }
}
