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
            NestedField::optional(16, "active_skill", Type::Primitive(PrimitiveType::String))
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{
        ArrayRef, Int32Array, Int64Array, RecordBatch, StringArray, TimestampMicrosecondArray,
    };
    use arrow_schema::{DataType, TimeUnit};

    use super::*;

    /// Helper: convert an Iceberg schema to an Arrow schema.
    fn to_arrow(iceberg: &Schema) -> arrow_schema::Schema {
        iceberg::arrow::schema_to_arrow_schema(iceberg)
            .expect("failed to convert Iceberg schema to Arrow schema")
    }

    /// The Iceberg `Timestamptz` type must map to Arrow
    /// `Timestamp(Microsecond, Some("+00:00"))`.  Arrow treats `"UTC"` and
    /// `"+00:00"` as distinct values, so using `"UTC"` in the array builder
    /// causes `RecordBatch::try_new` to fail with a type-mismatch error.
    #[test]
    fn timestamptz_maps_to_utc_offset_string() {
        let schema = logs_schema().unwrap();
        let arrow = to_arrow(&schema);

        let ts_field = arrow.field_with_name("timestamp").unwrap();
        assert_eq!(
            ts_field.data_type(),
            &DataType::Timestamp(TimeUnit::Microsecond, Some("+00:00".into())),
            "Timestamptz must use the literal '+00:00' offset string, not 'UTC'"
        );
    }

    /// A `RecordBatch` for the logs schema succeeds when the timestamp array
    /// uses `"+00:00"` (the fix).
    #[test]
    fn logs_record_batch_succeeds_with_utc_offset() {
        let schema = logs_schema().unwrap();
        let arrow = Arc::new(to_arrow(&schema));

        let cols: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(vec!["id-1"])),
            Arc::new(TimestampMicrosecondArray::from(vec![0_i64]).with_timezone("+00:00")),
            Arc::new(Int32Array::from(vec![9_i32])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![Some("hello")])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
        ];

        RecordBatch::try_new(arrow, cols).expect("RecordBatch with '+00:00' timezone must succeed");
    }

    /// The same construction fails when the timestamp array uses `"UTC"`,
    /// reproducing the production error before the fix.
    #[test]
    fn logs_record_batch_fails_with_utc_string() {
        let schema = logs_schema().unwrap();
        let arrow = Arc::new(to_arrow(&schema));

        let cols: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(vec!["id-1"])),
            // "UTC" triggers the pre-fix schema mismatch
            Arc::new(TimestampMicrosecondArray::from(vec![0_i64]).with_timezone("UTC")),
            Arc::new(Int32Array::from(vec![9_i32])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![Some("hello")])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
        ];

        assert!(
            RecordBatch::try_new(arrow, cols).is_err(),
            "RecordBatch with 'UTC' timezone string must fail (pre-fix behaviour)"
        );
    }

    /// A `RecordBatch` for the spans schema succeeds with `"+00:00"`.
    #[test]
    fn spans_record_batch_succeeds_with_utc_offset() {
        let schema = spans_schema().unwrap();
        let arrow = Arc::new(to_arrow(&schema));

        let cols: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(vec!["trace-1"])),
            Arc::new(StringArray::from(vec!["span-1"])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec!["my-span"])),
            Arc::new(TimestampMicrosecondArray::from(vec![0_i64]).with_timezone("+00:00")),
            Arc::new(TimestampMicrosecondArray::from(vec![Some(1000_i64)]).with_timezone("+00:00")),
            Arc::new(Int64Array::from(vec![Some(1_i64)])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(Int64Array::from(vec![None::<i64>])),
            Arc::new(Int64Array::from(vec![None::<i64>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])), // active_skill
        ];

        RecordBatch::try_new(arrow, cols).expect("RecordBatch with '+00:00' timezone must succeed");
    }

    /// A `RecordBatch` for the metrics schema succeeds with `"+00:00"`.
    #[test]
    fn metrics_record_batch_succeeds_with_utc_offset() {
        let schema = metrics_schema().unwrap();
        let arrow = Arc::new(to_arrow(&schema));

        let none_f64: Vec<Option<f64>> = vec![None];
        let none_i64: Vec<Option<i64>> = vec![None];

        use arrow_array::Float64Array;
        let cols: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(vec!["metric-id-1"])),
            Arc::new(TimestampMicrosecondArray::from(vec![0_i64]).with_timezone("+00:00")),
            Arc::new(StringArray::from(vec!["my.metric"])),
            Arc::new(StringArray::from(vec!["gauge"])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(Float64Array::from(none_f64.clone())),
            Arc::new(Int64Array::from(none_i64.clone())),
            Arc::new(Float64Array::from(none_f64.clone())),
            Arc::new(Float64Array::from(none_f64.clone())),
            Arc::new(Float64Array::from(none_f64.clone())),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![None::<&str>])),
        ];

        RecordBatch::try_new(arrow, cols).expect("RecordBatch with '+00:00' timezone must succeed");
    }
}
