//! End-to-end metric export tests for `SqliteMetricExporter`.
//!
//! Uses the same `SdkMeterProvider` + `PeriodicReader` pattern as the
//! metric tests. Forces a synchronous flush
//! via `provider.shutdown()` and validates rows land in the
//! `metric_points` SQLite table.

use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry::metrics::MeterProvider as _;
use opentelemetry_exporter_sqlite::SqliteMetricExporter;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_semantic_conventions::attribute::SERVICE_NAME;
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    // File-backed DB so that connections taken from the pool see the
    // same schema. `sqlite::memory:` gives each connection its own DB
    // which breaks tests that span multiple acquisitions.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metrics.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::raw_sql(include_str!("../../../migrations/015_metrics.sql"))
        .execute(&pool)
        .await
        .unwrap();
    // 019 adds the agent_id column referenced by the exporter's INSERTs.
    sqlx::raw_sql(include_str!(
        "../../../migrations/019_metrics_agent_scope.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    (pool, dir)
}

fn build_provider(pool: SqlitePool, resource: Resource) -> SdkMeterProvider {
    let exporter = SqliteMetricExporter::new(pool);
    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(3600))
        .build();
    SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build()
}

fn resource(name: &str) -> Resource {
    Resource::builder_empty()
        .with_attributes([KeyValue::new(SERVICE_NAME, name.to_string())])
        .build()
}

async fn flush_and_settle(provider: &SdkMeterProvider) {
    // Force a flush, then yield to let the PeriodicReader's task drain.
    let _ = provider.force_flush();
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        tokio::task::yield_now().await;
    }
    let _ = provider.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn counter_increments_persist_as_sum_rows() {
    let (pool, _dir) = test_pool().await;
    let provider = build_provider(pool.clone(), resource("counter-test"));

    let meter = provider.meter("test-scope");
    let counter = meter.u64_counter("requests").build();
    counter.add(1, &[KeyValue::new("route", "/a")]);
    counter.add(4, &[KeyValue::new("route", "/a")]);

    flush_and_settle(&provider).await;

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM metric_points WHERE metric_name = 'requests'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(count >= 1, "expected at least one metric row, got {count}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn histogram_records_persist_with_bounds_and_buckets() {
    let (pool, _dir) = test_pool().await;
    let provider = build_provider(pool.clone(), resource("hist-test"));

    let meter = provider.meter("test-scope");
    let hist = meter.f64_histogram("latency_seconds").build();
    hist.record(0.1, &[]);
    hist.record(0.5, &[]);
    hist.record(1.0, &[]);

    flush_and_settle(&provider).await;

    let count_opt: Option<(i64, Option<f64>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT count, sum, bounds, bucket_counts \
             FROM metric_points WHERE metric_name = 'latency_seconds' LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(count_opt.is_some(), "histogram row must exist");
    let (cnt, sum, bounds, buckets) = count_opt.unwrap();
    assert_eq!(cnt, 3);
    assert!(sum.unwrap() > 1.0);
    assert!(bounds.is_some());
    assert!(buckets.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn i64_up_down_counter_persists() {
    let (pool, _dir) = test_pool().await;
    let provider = build_provider(pool.clone(), resource("i64-counter-test"));

    let meter = provider.meter("test-scope");
    let counter = meter.i64_up_down_counter("delta").build();
    counter.add(5, &[]);
    counter.add(-2, &[]);

    flush_and_settle(&provider).await;

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM metric_points WHERE metric_name = 'delta'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(count >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metric_with_gen_ai_attrs_populates_denormalized_columns() {
    use opentelemetry_semantic_conventions::attribute::{
        GEN_AI_OPERATION_NAME, GEN_AI_REQUEST_MODEL,
    };

    let (pool, _dir) = test_pool().await;
    let provider = build_provider(pool.clone(), resource("gen-ai-test"));

    let meter = provider.meter("test-scope");
    let counter = meter.u64_counter("gen_ai.client.token.usage").build();
    counter.add(
        100,
        &[
            KeyValue::new(GEN_AI_REQUEST_MODEL, "gpt-4o"),
            KeyValue::new("gen_ai.provider.name", "openai"),
            KeyValue::new(GEN_AI_OPERATION_NAME, "chat"),
        ],
    );

    flush_and_settle(&provider).await;

    let row_opt: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT model, provider, operation FROM metric_points \
             WHERE metric_name = 'gen_ai.client.token.usage' LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(row_opt.is_some(), "gen-ai metric row must exist");
    let (model, provider_col, operation) = row_opt.unwrap();
    assert_eq!(model.as_deref(), Some("gpt-4o"));
    assert_eq!(provider_col.as_deref(), Some("openai"));
    assert_eq!(operation.as_deref(), Some("chat"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiple_metrics_share_resource_row() {
    let (pool, _dir) = test_pool().await;
    let provider = build_provider(pool.clone(), resource("multi-test"));

    let meter = provider.meter("test-scope");
    meter.u64_counter("m1").build().add(1, &[]);
    meter.u64_counter("m2").build().add(1, &[]);
    meter.u64_counter("m3").build().add(1, &[]);

    flush_and_settle(&provider).await;

    let resource_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resources")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(resource_count, 1);
}
