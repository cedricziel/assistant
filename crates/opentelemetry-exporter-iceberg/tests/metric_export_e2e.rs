//! End-to-end metric export tests for `IcebergMetricExporter`.
//!
//! Uses `SdkMeterProvider` + `PeriodicReader` to drive real instruments
//! through the SDK pipeline and asserts the resulting Parquet files
//! land in the warehouse.

use std::path::Path;
use std::time::Duration;

use assistant_core::types::observability::{IcebergConfig, PartitionGranularity};
use opentelemetry::KeyValue;
use opentelemetry::metrics::MeterProvider as _;
use opentelemetry_exporter_iceberg::IcebergMetricExporter;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_semantic_conventions::attribute::SERVICE_NAME;

fn test_config(warehouse: &str) -> IcebergConfig {
    IcebergConfig {
        warehouse: Some(warehouse.to_string()),
        namespace: "test".to_string(),
        partition: PartitionGranularity::None,
        catalog_uri: None,
    }
}

fn find_parquet_files(root: &str) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    collect_parquet(Path::new(root), &mut results);
    results
}

fn collect_parquet(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
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

async fn make_provider(warehouse: &str) -> SdkMeterProvider {
    let exporter = IcebergMetricExporter::new(test_config(warehouse))
        .await
        .expect("IcebergMetricExporter::new");
    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(3600))
        .build();
    let resource = Resource::builder_empty()
        .with_attributes([KeyValue::new(SERVICE_NAME, "metric-test")])
        .build();
    SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build()
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_increments_write_parquet() {
    let dir = tempfile::tempdir().expect("tempdir");
    let warehouse = dir.path().to_str().unwrap();
    let provider = make_provider(warehouse).await;

    let meter = provider.meter("test-scope");
    let counter = meter.u64_counter("requests").build();
    counter.add(1, &[KeyValue::new("route", "/a")]);
    counter.add(4, &[KeyValue::new("route", "/a")]);

    provider.shutdown().unwrap();

    let parquet_files = find_parquet_files(warehouse);
    assert!(
        !parquet_files.is_empty(),
        "counter metric must produce at least one Parquet file"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn histogram_records_write_parquet() {
    let dir = tempfile::tempdir().expect("tempdir");
    let warehouse = dir.path().to_str().unwrap();
    let provider = make_provider(warehouse).await;

    let meter = provider.meter("test-scope");
    let hist = meter.f64_histogram("latency_seconds").build();
    hist.record(0.1, &[]);
    hist.record(0.5, &[]);
    hist.record(1.0, &[]);

    provider.shutdown().unwrap();

    let parquet_files = find_parquet_files(warehouse);
    assert!(
        !parquet_files.is_empty(),
        "histogram metric must produce at least one Parquet file"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn day_partitioned_metric_export_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let warehouse = dir.path().to_str().unwrap();

    let mut config = test_config(warehouse);
    config.partition = PartitionGranularity::Day;
    let exporter = IcebergMetricExporter::new(config)
        .await
        .expect("IcebergMetricExporter::new");
    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(3600))
        .build();
    let resource = Resource::builder_empty()
        .with_attributes([KeyValue::new(SERVICE_NAME, "day-partition")])
        .build();
    let provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    let meter = provider.meter("scope");
    let counter = meter.u64_counter("requests").build();
    counter.add(1, &[]);
    provider.shutdown().unwrap();

    let parquet_files = find_parquet_files(warehouse);
    assert!(
        !parquet_files.is_empty(),
        "day-partitioned metric export must write at least one Parquet file"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn multiple_metric_kinds_share_warehouse() {
    let dir = tempfile::tempdir().expect("tempdir");
    let warehouse = dir.path().to_str().unwrap();
    let provider = make_provider(warehouse).await;

    let meter = provider.meter("scope");
    meter.u64_counter("c").build().add(1, &[]);
    meter.i64_up_down_counter("ud").build().add(-2, &[]);
    meter.f64_histogram("h").build().record(0.5, &[]);

    provider.shutdown().unwrap();

    let parquet_files = find_parquet_files(warehouse);
    assert!(
        !parquet_files.is_empty(),
        "mixed-kind metrics must produce at least one Parquet file"
    );
}
