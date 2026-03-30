//! Iceberg table schema definitions for spans, logs, and metric points.

use anyhow::Result;
use iceberg::spec::{NestedField, PrimitiveType, Schema, Type};
use std::sync::Arc;

// -- Field IDs for assistant_spans ---------------------------------------

/// Field ID for `start_time` in `assistant_spans` — used as the partition source.
pub const SPANS_START_TIME_FIELD_ID: i32 = 5;

/// Build the Iceberg schema for the `assistant_spans` table.
pub fn spans_schema() -> Result<Schema> {
    Schema::builder()
        .with_schema_id(1)
        .with_fields(vec![
            NestedField::required(1, "trace_id", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::required(2, "span_id", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(3, "parent_span_id", Type::Primitive(PrimitiveType::String))
                .into(),
            NestedField::required(4, "name", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::required(
                SPANS_START_TIME_FIELD_ID,
                "start_time",
                Type::Primitive(PrimitiveType::Timestamptz),
            )
            .into(),
            NestedField::optional(6, "end_time", Type::Primitive(PrimitiveType::Timestamptz))
                .into(),
            NestedField::optional(7, "duration_ms", Type::Primitive(PrimitiveType::Long)).into(),
            NestedField::optional(8, "service_name", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(9, "conversation_id", Type::Primitive(PrimitiveType::String))
                .into(),
            NestedField::optional(10, "tool_name", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(11, "tool_status", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(12, "input_tokens", Type::Primitive(PrimitiveType::Long)).into(),
            NestedField::optional(13, "output_tokens", Type::Primitive(PrimitiveType::Long)).into(),
            NestedField::optional(14, "attributes", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(
                15,
                "resource_attributes",
                Type::Primitive(PrimitiveType::String),
            )
            .into(),
        ])
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build spans schema: {e}"))
}

// -- Field IDs for assistant_logs ----------------------------------------

/// Field ID for `timestamp` in `assistant_logs` — used as the partition source.
pub const LOGS_TIMESTAMP_FIELD_ID: i32 = 2;

/// Build the Iceberg schema for the `assistant_logs` table.
pub fn logs_schema() -> Result<Schema> {
    Schema::builder()
        .with_schema_id(1)
        .with_fields(vec![
            NestedField::required(1, "id", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::required(
                LOGS_TIMESTAMP_FIELD_ID,
                "timestamp",
                Type::Primitive(PrimitiveType::Timestamptz),
            )
            .into(),
            NestedField::required(3, "severity_number", Type::Primitive(PrimitiveType::Int)).into(),
            NestedField::optional(4, "severity_text", Type::Primitive(PrimitiveType::String))
                .into(),
            NestedField::optional(5, "body", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(6, "trace_id", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(7, "span_id", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(8, "target", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(9, "service_name", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(10, "attributes", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(
                11,
                "resource_attributes",
                Type::Primitive(PrimitiveType::String),
            )
            .into(),
        ])
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build logs schema: {e}"))
}

// -- Field IDs for assistant_metric_points -------------------------------

/// Field ID for `recorded_at` in `assistant_metric_points` — used as the partition source.
pub const METRICS_RECORDED_AT_FIELD_ID: i32 = 2;

/// Build the Iceberg schema for the `assistant_metric_points` table.
pub fn metrics_schema() -> Result<Schema> {
    Schema::builder()
        .with_schema_id(1)
        .with_fields(vec![
            NestedField::required(1, "id", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::required(
                METRICS_RECORDED_AT_FIELD_ID,
                "recorded_at",
                Type::Primitive(PrimitiveType::Timestamptz),
            )
            .into(),
            NestedField::required(3, "metric_name", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::required(4, "metric_kind", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(5, "unit", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(6, "value", Type::Primitive(PrimitiveType::Double)).into(),
            NestedField::optional(7, "count", Type::Primitive(PrimitiveType::Long)).into(),
            NestedField::optional(8, "sum", Type::Primitive(PrimitiveType::Double)).into(),
            NestedField::optional(9, "min", Type::Primitive(PrimitiveType::Double)).into(),
            NestedField::optional(10, "max", Type::Primitive(PrimitiveType::Double)).into(),
            NestedField::optional(11, "bounds", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(12, "bucket_counts", Type::Primitive(PrimitiveType::String))
                .into(),
            NestedField::optional(13, "attributes", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(14, "model", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(15, "provider", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(16, "operation", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(17, "skill", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(18, "interface", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(19, "agent_id", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(20, "service_name", Type::Primitive(PrimitiveType::String))
                .into(),
            NestedField::optional(21, "scope_name", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::optional(
                22,
                "resource_attributes",
                Type::Primitive(PrimitiveType::String),
            )
            .into(),
        ])
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build metrics schema: {e}"))
}

/// Wrap a [`Schema`] in an [`Arc`] for use with the writer.
pub fn arc(schema: Schema) -> Arc<Schema> {
    Arc::new(schema)
}
