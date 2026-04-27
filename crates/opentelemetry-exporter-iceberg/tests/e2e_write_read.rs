//! End-to-end tests: write OTel spans/logs to an Iceberg warehouse on the
//! local filesystem, then scan the Parquet data files back and verify the
//! expected rows are present.

use std::borrow::Cow;
use std::time::SystemTime;

use arrow_array::Array;
use assistant_core::{IcebergConfig, PartitionGranularity};
use opentelemetry::InstrumentationScope;
use opentelemetry::trace::{
    SpanContext, SpanId, SpanKind, Status, TraceFlags, TraceId, TraceState,
};
use opentelemetry_exporter_iceberg::IcebergSpanExporter;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{SpanData, SpanEvents, SpanExporter, SpanLinks};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_config(warehouse: &str) -> IcebergConfig {
    IcebergConfig {
        warehouse: Some(warehouse.to_string()),
        namespace: "test".to_string(),
        partition: PartitionGranularity::None,
        catalog_uri: None,
    }
}

fn make_span(name: &'static str) -> SpanData {
    let trace_id = TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").unwrap();
    let span_id = SpanId::from_hex("00f067aa0ba902b7").unwrap();
    SpanData {
        span_context: SpanContext::new(
            trace_id,
            span_id,
            TraceFlags::SAMPLED,
            false,
            TraceState::NONE,
        ),
        parent_span_id: SpanId::INVALID,
        parent_span_is_remote: false,
        span_kind: SpanKind::Internal,
        name: Cow::Borrowed(name),
        start_time: SystemTime::UNIX_EPOCH,
        end_time: SystemTime::UNIX_EPOCH,
        attributes: vec![],
        dropped_attributes_count: 0,
        events: SpanEvents::default(),
        links: SpanLinks::default(),
        status: Status::Ok,
        instrumentation_scope: InstrumentationScope::builder("test").build(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Exports one span via `IcebergSpanExporter`, then scans the Parquet file
/// that was written to the temp warehouse and asserts the span name is present.
#[tokio::test]
async fn span_is_written_and_queryable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let warehouse = dir.path().to_str().unwrap();
    let config = test_config(warehouse);

    let exporter = IcebergSpanExporter::new(config)
        .await
        .expect("IcebergSpanExporter::new");

    let span = make_span("my-test-span");
    let result: OTelSdkResult = exporter.export(vec![span]).await;
    assert!(result.is_ok(), "export must succeed: {result:?}");

    // Scan the written Parquet files from the warehouse directory.
    let parquet_files = find_parquet_files(warehouse);
    assert!(
        !parquet_files.is_empty(),
        "at least one Parquet file must be written to the warehouse"
    );

    let mut found = false;
    for path in &parquet_files {
        let file = std::fs::File::open(path).unwrap();
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap();

        for batch in reader {
            let batch = batch.unwrap();
            let col = batch.column_by_name("name").unwrap();
            let arr = col
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap();
            for i in 0..arr.len() {
                if arr.value(i) == "my-test-span" {
                    found = true;
                }
            }
        }
    }

    assert!(
        found,
        "exported span 'my-test-span' must be readable from Parquet"
    );
}

/// Exporting a span with `partition = Day` must succeed end-to-end.
///
/// This is a regression test for the bug where `DataFileWriterBuilder::build(None)`
/// produced data files with an empty partition struct while the table had a
/// non-empty partition spec, causing `fast_append` to reject the commit with
/// "Partition value is not compatible with partition type".
#[tokio::test]
async fn span_exported_with_day_partition_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let warehouse = dir.path().to_str().unwrap();
    let config = IcebergConfig {
        warehouse: Some(warehouse.to_string()),
        namespace: "test".to_string(),
        partition: PartitionGranularity::Day,
        catalog_uri: None,
    };

    let exporter = IcebergSpanExporter::new(config)
        .await
        .expect("IcebergSpanExporter::new");

    let span = make_span("partitioned-span");
    let result: OTelSdkResult = exporter.export(vec![span]).await;
    assert!(
        result.is_ok(),
        "export with day partition must succeed: {result:?}"
    );

    let parquet_files = find_parquet_files(warehouse);
    assert!(
        !parquet_files.is_empty(),
        "at least one Parquet file must be written for the partitioned export"
    );
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn find_parquet_files(root: &str) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    collect_parquet(std::path::Path::new(root), &mut results);
    results
}

fn collect_parquet(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_parquet(&path, out);
        } else if path.extension().is_some_and(|e| e == "parquet") {
            out.push(path);
        }
    }
}
