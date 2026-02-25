use futures::future::BoxFuture;

use chrono::{DateTime, Utc};
use opentelemetry::trace::{SpanId, Status, TraceError};
use opentelemetry::{KeyValue, Value};
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};
use serde_json::{Map, Number};
use sqlx::SqlitePool;
use uuid::Uuid;

/// OpenTelemetry span exporter that persists spans into the `distributed_traces`
/// SQLite table.
#[derive(Clone, Debug)]
pub struct SqliteSpanExporter {
    pool: SqlitePool,
}

impl SqliteSpanExporter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn persist_span(pool: SqlitePool, span: SpanData) -> Result<(), sqlx::Error> {
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
        .execute(&pool)
        .await?;

        Ok(())
    }
}

impl SpanExporter for SqliteSpanExporter {
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        let pool = self.pool.clone();
        Box::pin(async move {
            for span in batch {
                if let Err(err) = SqliteSpanExporter::persist_span(pool.clone(), span).await {
                    return Err(TraceError::Other(Box::new(err)));
                }
            }
            Ok(())
        })
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
        },
    }
}

fn status_fields(status: &Status) -> (&'static str, Option<String>) {
    match status {
        Status::Ok => ("ok", None),
        Status::Unset => ("unset", None),
        Status::Error { description } => ("error", Some(description.to_string())),
    }
}
