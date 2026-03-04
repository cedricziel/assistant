use chrono::{DateTime, Utc};
use opentelemetry::trace::SpanId;
use opentelemetry::{KeyValue, Value};
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::trace::{SpanData, SpanExporter};
use serde_json::{Map, Number};
use sqlx::{SqliteConnection, SqlitePool};
use tokio::runtime::Handle;
use uuid::Uuid;

/// OpenTelemetry span exporter that persists spans into the `distributed_traces`
/// SQLite table.
///
/// See [`SqliteLogExporter`](crate::SqliteLogExporter) for details on why
/// we capture a Tokio [`Handle`].
#[derive(Clone, Debug)]
pub struct SqliteSpanExporter {
    pool: SqlitePool,
    rt_handle: Handle,
}

impl SqliteSpanExporter {
    /// Create a new exporter.
    ///
    /// # Panics
    /// Panics if called outside a Tokio runtime.
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            rt_handle: Handle::current(),
        }
    }

    /// Inner export logic, always called with a Tokio context available.
    async fn export_inner(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| OTelSdkError::InternalFailure(format!("SQLite begin failed: {e}")))?;

        for span in batch {
            if let Err(err) = SqliteSpanExporter::persist_span(&mut tx, span).await {
                return Err(OTelSdkError::InternalFailure(format!(
                    "SQLite span export failed: {err}"
                )));
            }
        }

        tx.commit()
            .await
            .map_err(|e| OTelSdkError::InternalFailure(format!("SQLite commit failed: {e}")))?;

        Ok(())
    }

    async fn persist_span(conn: &mut SqliteConnection, span: SpanData) -> Result<(), sqlx::Error> {
        let mut attrs = attributes_to_map(&span.attributes);
        let (status_code, status_message) = status_fields(&span.status);
        attrs.insert(
            "otel.status_code".to_string(),
            serde_json::Value::String(status_code.to_string()),
        );
        if let Some(msg) = status_message {
            attrs.insert(
                "otel.status_message".to_string(),
                serde_json::Value::String(msg),
            );
        }

        let start_time: DateTime<Utc> = span.start_time.into();
        let end_time: DateTime<Utc> = span.end_time.into();
        let duration_ms = span
            .end_time
            .duration_since(span.start_time)
            .map(|dur| dur.as_millis() as i64)
            .unwrap_or(0);
        attrs.insert(
            "duration_ms".to_string(),
            serde_json::Value::Number(Number::from(duration_ms)),
        );

        let attributes_json = serde_json::Value::Object(attrs.clone());
        let attrs_serialized = attributes_json.to_string();

        let conversation_id = attrs
            .get("conversation_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .map(|uuid| uuid.to_string());
        let turn = attrs.get("turn").and_then(|v| v.as_i64());
        let tool_name = attrs
            .get("tool_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let tool_status = attrs
            .get("tool_status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let observation = attrs
            .get("tool_observation")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let error = attrs
            .get("tool_error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let input_tokens = attrs
            .get("gen_ai.usage.input_tokens")
            .and_then(|v| v.as_i64());
        let output_tokens = attrs
            .get("gen_ai.usage.output_tokens")
            .and_then(|v| v.as_i64());

        let parent_span_id = if span.parent_span_id == SpanId::INVALID {
            None
        } else {
            Some(span.parent_span_id.to_string())
        };

        sqlx::query(
            "INSERT INTO distributed_traces \
                (span_id, trace_id, parent_span_id, name, conversation_id, turn, tool_name, \
                 tool_status, tool_observation, tool_error, duration_ms, start_time, end_time, \
                 attributes, input_tokens, output_tokens) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16) \
             ON CONFLICT(span_id) DO NOTHING",
        )
        .bind(span.span_context.span_id().to_string())
        .bind(span.span_context.trace_id().to_string())
        .bind(parent_span_id)
        .bind(span.name.to_string())
        .bind(conversation_id)
        .bind(turn)
        .bind(tool_name)
        .bind(tool_status)
        .bind(observation)
        .bind(error)
        .bind(duration_ms)
        .bind(start_time)
        .bind(end_time)
        .bind(attrs_serialized)
        .bind(input_tokens)
        .bind(output_tokens)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}

impl SpanExporter for SqliteSpanExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        if Handle::try_current().is_ok() {
            self.export_inner(batch).await
        } else {
            self.rt_handle.block_on(self.export_inner(batch))
        }
    }
}

fn attributes_to_map(attrs: &[KeyValue]) -> Map<String, serde_json::Value> {
    let mut map = Map::new();
    for kv in attrs {
        map.insert(kv.key.as_str().to_string(), otel_value_to_json(&kv.value));
    }
    map
}

fn otel_value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Bool(v) => serde_json::Value::Bool(*v),
        Value::I64(v) => serde_json::Value::Number(Number::from(*v)),
        Value::F64(v) => Number::from_f64(*v)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(v) => serde_json::Value::String(v.as_str().to_string()),
        Value::Array(array) => match array {
            opentelemetry::Array::Bool(values) => serde_json::Value::Array(
                values.iter().map(|b| serde_json::Value::Bool(*b)).collect(),
            ),
            opentelemetry::Array::I64(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(|i| serde_json::Value::Number(Number::from(*i)))
                    .collect(),
            ),
            opentelemetry::Array::F64(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(|f| {
                        Number::from_f64(*f)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null)
                    })
                    .collect(),
            ),
            opentelemetry::Array::String(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(|s| serde_json::Value::String(s.as_str().to_string()))
                    .collect(),
            ),
            _ => serde_json::Value::String(format!("{array:?}")),
        },
        _ => serde_json::Value::String(format!("{value:?}")),
    }
}

fn status_fields(status: &opentelemetry::trace::Status) -> (&'static str, Option<String>) {
    match status {
        opentelemetry::trace::Status::Ok => ("ok", None),
        opentelemetry::trace::Status::Unset => ("unset", None),
        opentelemetry::trace::Status::Error { description } => {
            ("error", Some(description.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::trace::{SpanContext, SpanKind, Status, TraceFlags, TraceId, TraceState};
    use opentelemetry::InstrumentationScope;
    use opentelemetry_sdk::trace::{SpanData, SpanEvents, SpanLinks};
    use std::time::{Duration, SystemTime};

    fn make_span(name: &str, tool_name: Option<&str>, status: Status) -> SpanData {
        let now = SystemTime::now();
        let trace_id = TraceId::from(uuid::Uuid::new_v4().as_u128());
        let span_id = SpanId::from(rand_u64());

        let mut attributes = vec![];
        if let Some(tn) = tool_name {
            attributes.push(KeyValue::new("tool_name", tn.to_string()));
            attributes.push(KeyValue::new("tool_status", "ok"));
        }

        SpanData {
            span_context: SpanContext::new(
                trace_id,
                span_id,
                TraceFlags::SAMPLED,
                false,
                TraceState::default(),
            ),
            parent_span_id: SpanId::INVALID,
            parent_span_is_remote: false,
            span_kind: SpanKind::Internal,
            name: name.to_string().into(),
            start_time: now,
            end_time: now + Duration::from_millis(42),
            attributes,
            dropped_attributes_count: 0,
            events: SpanEvents::default(),
            links: SpanLinks::default(),
            status,
            instrumentation_scope: InstrumentationScope::default(),
        }
    }

    fn rand_u64() -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        SystemTime::now().hash(&mut h);
        std::thread::current().id().hash(&mut h);
        h.finish()
    }

    async fn row_count(pool: &SqlitePool, table: &str) -> i64 {
        let q = format!("SELECT COUNT(*) AS cnt FROM {table}");
        sqlx::query_scalar::<_, i64>(&q)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_export_persists_exact_count() {
        let pool = crate::test_utils::test_pool().await;
        let exporter = SqliteSpanExporter::new(pool.clone());

        let batch = vec![
            make_span("span-1", Some("bash"), Status::Ok),
            make_span("span-2", Some("file-read"), Status::Ok),
            make_span("span-3", None, Status::Unset),
        ];

        exporter.export(batch).await.unwrap();

        let count = row_count(&pool, "distributed_traces").await;
        assert_eq!(count, 3, "exporter must produce exactly one row per span");
    }

    #[tokio::test]
    async fn test_export_preserves_tool_metadata() {
        let pool = crate::test_utils::test_pool().await;
        let exporter = SqliteSpanExporter::new(pool.clone());

        let batch = vec![make_span("execute_tool bash", Some("bash"), Status::Ok)];
        exporter.export(batch).await.unwrap();

        let row = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
            "SELECT name, tool_name, tool_status FROM distributed_traces LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "execute_tool bash");
        assert_eq!(row.1.as_deref(), Some("bash"));
        assert_eq!(row.2.as_deref(), Some("ok"));
    }

    #[tokio::test]
    async fn test_export_records_duration() {
        let pool = crate::test_utils::test_pool().await;
        let exporter = SqliteSpanExporter::new(pool.clone());

        let now = SystemTime::now();
        let trace_id = TraceId::from(uuid::Uuid::new_v4().as_u128());
        let span_id = SpanId::from(rand_u64());

        let span = SpanData {
            span_context: SpanContext::new(
                trace_id,
                span_id,
                TraceFlags::SAMPLED,
                false,
                TraceState::default(),
            ),
            parent_span_id: SpanId::INVALID,
            parent_span_is_remote: false,
            span_kind: SpanKind::Internal,
            name: "timed".into(),
            start_time: now,
            end_time: now + Duration::from_millis(150),
            attributes: vec![],
            dropped_attributes_count: 0,
            events: SpanEvents::default(),
            links: SpanLinks::default(),
            status: Status::Ok,
            instrumentation_scope: InstrumentationScope::default(),
        };

        exporter.export(vec![span]).await.unwrap();

        let dur: i64 = sqlx::query_scalar("SELECT duration_ms FROM distributed_traces LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(dur, 150);
    }

    #[tokio::test]
    async fn test_export_captures_error_status() {
        let pool = crate::test_utils::test_pool().await;
        let exporter = SqliteSpanExporter::new(pool.clone());

        let batch = vec![make_span(
            "failing",
            Some("web-fetch"),
            Status::Error {
                description: "timeout".into(),
            },
        )];
        exporter.export(batch).await.unwrap();

        let attrs_str: String =
            sqlx::query_scalar("SELECT attributes FROM distributed_traces LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        let attrs: serde_json::Value = serde_json::from_str(&attrs_str).unwrap();

        assert_eq!(attrs["otel.status_code"], "error");
        assert_eq!(attrs["otel.status_message"], "timeout");
    }
}
