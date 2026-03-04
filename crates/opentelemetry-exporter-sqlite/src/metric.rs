//! OpenTelemetry metric exporter that persists data points into SQLite.
//!
//! Resources and instrumentation scopes are normalized into dedicated
//! join tables (`resources`, `metric_scopes`) to avoid duplicating
//! identical metadata on every data-point row.

use chrono::{DateTime, Utc};
use opentelemetry::InstrumentationScope;
use opentelemetry::{KeyValue, Value};
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::metrics::data::{
    AggregatedMetrics, GaugeDataPoint, HistogramDataPoint, MetricData, ResourceMetrics,
    SumDataPoint,
};
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;
use opentelemetry_sdk::metrics::Temporality;
use opentelemetry_sdk::Resource;
use serde_json::{Map, Number};
use sha2::{Digest, Sha256};
use sqlx::{SqliteConnection, SqlitePool};
use tokio::runtime::Handle;
use tracing::warn;

/// OpenTelemetry metric exporter that persists data points into the
/// `metric_points` SQLite table, with resources and scopes normalized
/// into join tables.
///
/// See [`SqliteLogExporter`](crate::SqliteLogExporter) for details on why
/// we capture a Tokio [`Handle`].
#[derive(Clone, Debug)]
pub struct SqliteMetricExporter {
    pool: SqlitePool,
    rt_handle: Handle,
}

impl SqliteMetricExporter {
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
    async fn export_inner(&self, metrics: &ResourceMetrics) -> OTelSdkResult {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| OTelSdkError::InternalFailure(format!("SQLite begin failed: {e}")))?;

        let resource_id = Self::ensure_resource(&mut tx, metrics.resource())
            .await
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        for scope_metrics in metrics.scope_metrics() {
            let scope_id = Self::ensure_scope(&mut tx, scope_metrics.scope())
                .await
                .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

            for metric in scope_metrics.metrics() {
                let name = metric.name();
                let unit = metric.unit();
                let desc = metric.description();

                match metric.data() {
                    AggregatedMetrics::F64(data) => {
                        Self::process_f64_metric(
                            &mut tx,
                            resource_id,
                            scope_id,
                            name,
                            unit,
                            desc,
                            data,
                        )
                        .await;
                    }
                    AggregatedMetrics::I64(data) => {
                        Self::process_i64_metric(
                            &mut tx,
                            resource_id,
                            scope_id,
                            name,
                            unit,
                            desc,
                            data,
                        )
                        .await;
                    }
                    AggregatedMetrics::U64(_) => {
                        warn!(metric = name, "u64 metrics not yet supported, skipping");
                    }
                }
            }
        }

        tx.commit()
            .await
            .map_err(|e| OTelSdkError::InternalFailure(format!("SQLite commit failed: {e}")))?;

        Ok(())
    }

    /// Upsert the resource and return its row id.
    async fn ensure_resource(
        conn: &mut SqliteConnection,
        resource: &Resource,
    ) -> Result<i64, sqlx::Error> {
        let fingerprint = resource_fingerprint(resource);
        let attrs_json = resource_to_json(resource);

        sqlx::query("INSERT OR IGNORE INTO resources (fingerprint, attributes) VALUES (?1, ?2)")
            .bind(&fingerprint)
            .bind(&attrs_json)
            .execute(&mut *conn)
            .await?;

        let id: i64 = sqlx::query_scalar("SELECT id FROM resources WHERE fingerprint = ?1")
            .bind(&fingerprint)
            .fetch_one(&mut *conn)
            .await?;

        Ok(id)
    }

    /// Upsert the instrumentation scope and return its row id.
    async fn ensure_scope(
        conn: &mut SqliteConnection,
        scope: &InstrumentationScope,
    ) -> Result<i64, sqlx::Error> {
        let name = scope.name();
        let version = scope.version().unwrap_or("");

        sqlx::query("INSERT OR IGNORE INTO metric_scopes (name, version) VALUES (?1, ?2)")
            .bind(name)
            .bind(version)
            .execute(&mut *conn)
            .await?;

        let id: i64 =
            sqlx::query_scalar("SELECT id FROM metric_scopes WHERE name = ?1 AND version = ?2")
                .bind(name)
                .bind(version)
                .fetch_one(&mut *conn)
                .await?;

        Ok(id)
    }

    /// Persist a single sum data point (f64).
    #[allow(clippy::too_many_arguments)]
    async fn insert_sum_scalar(
        conn: &mut SqliteConnection,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        kind: &str,
        unit: &str,
        description: &str,
        dp: &SumDataPoint<f64>,
        sum_start_time: std::time::SystemTime,
        sum_time: std::time::SystemTime,
    ) -> Result<(), sqlx::Error> {
        let attrs: Vec<KeyValue> = dp.attributes().cloned().collect();
        let da = extract_denormalized(&attrs);
        let attrs_json = keyvalues_to_json(&attrs);
        let start_time = non_epoch_time(Some(sum_start_time));
        let recorded_at: DateTime<Utc> = sum_time.into();

        sqlx::query(
            "INSERT INTO metric_points \
                (resource_id, scope_id, metric_name, metric_kind, unit, description, \
                 value, attributes, start_time, recorded_at, \
                 model, provider, operation, skill, interface) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        )
        .bind(resource_id)
        .bind(scope_id)
        .bind(name)
        .bind(kind)
        .bind(unit)
        .bind(description)
        .bind(dp.value())
        .bind(&attrs_json)
        .bind(start_time)
        .bind(recorded_at)
        .bind(&da.model)
        .bind(&da.provider)
        .bind(&da.operation)
        .bind(&da.skill)
        .bind(&da.interface)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Persist a single sum data point (i64).
    #[allow(clippy::too_many_arguments)]
    async fn insert_sum_scalar_i64(
        conn: &mut SqliteConnection,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        kind: &str,
        unit: &str,
        description: &str,
        dp: &SumDataPoint<i64>,
        sum_start_time: std::time::SystemTime,
        sum_time: std::time::SystemTime,
    ) -> Result<(), sqlx::Error> {
        let attrs: Vec<KeyValue> = dp.attributes().cloned().collect();
        let da = extract_denormalized(&attrs);
        let attrs_json = keyvalues_to_json(&attrs);
        let start_time = non_epoch_time(Some(sum_start_time));
        let recorded_at: DateTime<Utc> = sum_time.into();

        sqlx::query(
            "INSERT INTO metric_points \
                (resource_id, scope_id, metric_name, metric_kind, unit, description, \
                 value, attributes, start_time, recorded_at, \
                 model, provider, operation, skill, interface) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        )
        .bind(resource_id)
        .bind(scope_id)
        .bind(name)
        .bind(kind)
        .bind(unit)
        .bind(description)
        .bind(dp.value() as f64)
        .bind(&attrs_json)
        .bind(start_time)
        .bind(recorded_at)
        .bind(&da.model)
        .bind(&da.provider)
        .bind(&da.operation)
        .bind(&da.skill)
        .bind(&da.interface)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Persist a single gauge data point (f64).
    #[allow(clippy::too_many_arguments)]
    async fn insert_gauge_scalar(
        conn: &mut SqliteConnection,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        unit: &str,
        description: &str,
        dp: &GaugeDataPoint<f64>,
    ) -> Result<(), sqlx::Error> {
        let attrs: Vec<KeyValue> = dp.attributes().cloned().collect();
        let da = extract_denormalized(&attrs);
        let attrs_json = keyvalues_to_json(&attrs);

        sqlx::query(
            "INSERT INTO metric_points \
                (resource_id, scope_id, metric_name, metric_kind, unit, description, \
                 value, attributes, recorded_at, \
                 model, provider, operation, skill, interface) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        )
        .bind(resource_id)
        .bind(scope_id)
        .bind(name)
        .bind("gauge")
        .bind(unit)
        .bind(description)
        .bind(dp.value())
        .bind(&attrs_json)
        .bind(Utc::now())
        .bind(&da.model)
        .bind(&da.provider)
        .bind(&da.operation)
        .bind(&da.skill)
        .bind(&da.interface)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Persist a single gauge data point (i64).
    #[allow(clippy::too_many_arguments)]
    async fn insert_gauge_scalar_i64(
        conn: &mut SqliteConnection,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        unit: &str,
        description: &str,
        dp: &GaugeDataPoint<i64>,
    ) -> Result<(), sqlx::Error> {
        let attrs: Vec<KeyValue> = dp.attributes().cloned().collect();
        let da = extract_denormalized(&attrs);
        let attrs_json = keyvalues_to_json(&attrs);

        sqlx::query(
            "INSERT INTO metric_points \
                (resource_id, scope_id, metric_name, metric_kind, unit, description, \
                 value, attributes, recorded_at, \
                 model, provider, operation, skill, interface) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        )
        .bind(resource_id)
        .bind(scope_id)
        .bind(name)
        .bind("gauge")
        .bind(unit)
        .bind(description)
        .bind(dp.value() as f64)
        .bind(&attrs_json)
        .bind(Utc::now())
        .bind(&da.model)
        .bind(&da.provider)
        .bind(&da.operation)
        .bind(&da.skill)
        .bind(&da.interface)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Persist a histogram data point.
    #[allow(clippy::too_many_arguments)]
    async fn insert_histogram(
        conn: &mut SqliteConnection,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        unit: &str,
        description: &str,
        dp: &HistogramDataPoint<f64>,
        hist_start_time: std::time::SystemTime,
        hist_time: std::time::SystemTime,
    ) -> Result<(), sqlx::Error> {
        let attrs: Vec<KeyValue> = dp.attributes().cloned().collect();
        let da = extract_denormalized(&attrs);
        let attrs_json = keyvalues_to_json(&attrs);
        let start_time = non_epoch_time(Some(hist_start_time));
        let recorded_at: DateTime<Utc> = hist_time.into();
        let bounds: Vec<f64> = dp.bounds().collect();
        let bucket_counts: Vec<u64> = dp.bucket_counts().collect();
        let bounds_json = serde_json::to_string(&bounds).unwrap_or_default();
        let bucket_counts_json = serde_json::to_string(&bucket_counts).unwrap_or_default();

        sqlx::query(
            "INSERT INTO metric_points \
                (resource_id, scope_id, metric_name, metric_kind, unit, description, \
                 count, sum, min, max, bounds, bucket_counts, \
                 attributes, start_time, recorded_at, \
                 model, provider, operation, skill, interface) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        )
        .bind(resource_id)
        .bind(scope_id)
        .bind(name)
        .bind("histogram")
        .bind(unit)
        .bind(description)
        .bind(dp.count() as i64)
        .bind(dp.sum())
        .bind(dp.min())
        .bind(dp.max())
        .bind(&bounds_json)
        .bind(&bucket_counts_json)
        .bind(&attrs_json)
        .bind(start_time)
        .bind(recorded_at)
        .bind(&da.model)
        .bind(&da.provider)
        .bind(&da.operation)
        .bind(&da.skill)
        .bind(&da.interface)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Process a single `MetricData<f64>` value.
    async fn process_f64_metric(
        conn: &mut SqliteConnection,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        unit: &str,
        desc: &str,
        metric_data: &MetricData<f64>,
    ) {
        match metric_data {
            MetricData::Sum(sum) => {
                let kind = if sum.is_monotonic() {
                    "counter"
                } else {
                    "up_down_counter"
                };
                for dp in sum.data_points() {
                    if let Err(e) = Self::insert_sum_scalar(
                        &mut *conn,
                        resource_id,
                        scope_id,
                        name,
                        kind,
                        unit,
                        desc,
                        dp,
                        sum.start_time(),
                        sum.time(),
                    )
                    .await
                    {
                        warn!(metric = name, error = %e, "failed to persist f64 sum");
                    }
                }
            }
            MetricData::Gauge(gauge) => {
                for dp in gauge.data_points() {
                    if let Err(e) = Self::insert_gauge_scalar(
                        &mut *conn,
                        resource_id,
                        scope_id,
                        name,
                        unit,
                        desc,
                        dp,
                    )
                    .await
                    {
                        warn!(metric = name, error = %e, "failed to persist f64 gauge");
                    }
                }
            }
            MetricData::Histogram(hist) => {
                for dp in hist.data_points() {
                    if let Err(e) = Self::insert_histogram(
                        &mut *conn,
                        resource_id,
                        scope_id,
                        name,
                        unit,
                        desc,
                        dp,
                        hist.start_time(),
                        hist.time(),
                    )
                    .await
                    {
                        warn!(metric = name, error = %e, "failed to persist f64 histogram");
                    }
                }
            }
            _ => {
                warn!(metric = name, "unsupported f64 metric data type, skipping");
            }
        }
    }

    /// Process a single `MetricData<i64>` value.
    async fn process_i64_metric(
        conn: &mut SqliteConnection,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        unit: &str,
        desc: &str,
        metric_data: &MetricData<i64>,
    ) {
        match metric_data {
            MetricData::Sum(sum) => {
                let kind = if sum.is_monotonic() {
                    "counter"
                } else {
                    "up_down_counter"
                };
                for dp in sum.data_points() {
                    if let Err(e) = Self::insert_sum_scalar_i64(
                        &mut *conn,
                        resource_id,
                        scope_id,
                        name,
                        kind,
                        unit,
                        desc,
                        dp,
                        sum.start_time(),
                        sum.time(),
                    )
                    .await
                    {
                        warn!(metric = name, error = %e, "failed to persist i64 sum");
                    }
                }
            }
            MetricData::Gauge(gauge) => {
                for dp in gauge.data_points() {
                    if let Err(e) = Self::insert_gauge_scalar_i64(
                        &mut *conn,
                        resource_id,
                        scope_id,
                        name,
                        unit,
                        desc,
                        dp,
                    )
                    .await
                    {
                        warn!(metric = name, error = %e, "failed to persist i64 gauge");
                    }
                }
            }
            _ => {
                warn!(metric = name, "unsupported i64 metric data type, skipping");
            }
        }
    }
}

impl PushMetricExporter for SqliteMetricExporter {
    async fn export(&self, metrics: &ResourceMetrics) -> OTelSdkResult {
        if Handle::try_current().is_ok() {
            self.export_inner(metrics).await
        } else {
            self.rt_handle.block_on(self.export_inner(metrics))
        }
    }

    fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }

    fn shutdown_with_timeout(&self, _timeout: std::time::Duration) -> OTelSdkResult {
        Ok(())
    }

    fn temporality(&self) -> Temporality {
        // Delta: each export row represents a single collection interval,
        // mapping naturally to time-series storage.
        Temporality::Delta
    }
}

// -- Helpers ------------------------------------------------------------------

/// SHA-256 fingerprint of sorted resource attributes for deduplication.
fn resource_fingerprint(resource: &Resource) -> String {
    let mut pairs: Vec<(String, String)> = resource
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), format!("{v:?}")))
        .collect();
    pairs.sort();

    let mut hasher = Sha256::new();
    for (k, v) in &pairs {
        hasher.update(k.as_bytes());
        hasher.update(v.as_bytes());
        hasher.update(b",");
    }
    format!("{:x}", hasher.finalize())
}

/// Serialize resource attributes to a JSON string (stored once per resource row).
fn resource_to_json(resource: &Resource) -> String {
    let mut map = Map::new();
    for (key, value) in resource.iter() {
        map.insert(key.as_str().to_string(), otel_value_to_json(value));
    }
    serde_json::Value::Object(map).to_string()
}

/// Serialize a `KeyValue` slice to a JSON string.
fn keyvalues_to_json(attrs: &[KeyValue]) -> String {
    let mut map = Map::new();
    for kv in attrs {
        map.insert(kv.key.as_str().to_string(), otel_value_to_json(&kv.value));
    }
    serde_json::Value::Object(map).to_string()
}

/// Denormalized attribute values extracted from data-point attributes.
struct DenormalizedAttrs {
    model: Option<String>,
    provider: Option<String>,
    operation: Option<String>,
    skill: Option<String>,
    interface: Option<String>,
}

/// Extract well-known attribute values into denormalized columns.
fn extract_denormalized(attrs: &[KeyValue]) -> DenormalizedAttrs {
    let mut da = DenormalizedAttrs {
        model: None,
        provider: None,
        operation: None,
        skill: None,
        interface: None,
    };

    for kv in attrs {
        match kv.key.as_str() {
            "gen_ai.request.model" => da.model = value_as_string(&kv.value),
            "gen_ai.provider.name" => da.provider = value_as_string(&kv.value),
            "gen_ai.operation.name" => da.operation = value_as_string(&kv.value),
            "skill" => da.skill = value_as_string(&kv.value),
            "interface" => da.interface = value_as_string(&kv.value),
            _ => {}
        }
    }

    da
}

fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(v) => Some(v.as_str().to_string()),
        _ => None,
    }
}

/// Convert an `Option<SystemTime>` to `Option<DateTime<Utc>>`, collapsing
/// the UNIX epoch sentinel to `None`.
fn non_epoch_time(t: Option<std::time::SystemTime>) -> Option<DateTime<Utc>> {
    t.filter(|t| *t != std::time::UNIX_EPOCH).map(Into::into)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resource_upsert_is_idempotent() {
        let pool = crate::test_utils::test_pool().await;

        let resource = Resource::builder_empty()
            .with_attributes([
                KeyValue::new("service.name", "test"),
                KeyValue::new("service.version", "0.1.0"),
            ])
            .build();

        let mut conn = pool.acquire().await.unwrap();
        let id1 = SqliteMetricExporter::ensure_resource(&mut *conn, &resource)
            .await
            .unwrap();
        let id2 = SqliteMetricExporter::ensure_resource(&mut *conn, &resource)
            .await
            .unwrap();
        assert_eq!(id1, id2, "same resource must return same id");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resources")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "only one resource row should exist");
    }

    #[tokio::test]
    async fn scope_upsert_is_idempotent() {
        let pool = crate::test_utils::test_pool().await;

        let scope = InstrumentationScope::builder("test-scope")
            .with_version("1.0")
            .build();

        let mut conn = pool.acquire().await.unwrap();
        let id1 = SqliteMetricExporter::ensure_scope(&mut *conn, &scope)
            .await
            .unwrap();
        let id2 = SqliteMetricExporter::ensure_scope(&mut *conn, &scope)
            .await
            .unwrap();
        assert_eq!(id1, id2, "same scope must return same id");
    }

    #[test]
    fn fingerprint_is_order_independent() {
        let r1 = Resource::builder_empty()
            .with_attributes([KeyValue::new("a", "1"), KeyValue::new("b", "2")])
            .build();
        let r2 = Resource::builder_empty()
            .with_attributes([KeyValue::new("b", "2"), KeyValue::new("a", "1")])
            .build();
        assert_eq!(
            resource_fingerprint(&r1),
            resource_fingerprint(&r2),
            "attribute order must not affect fingerprint"
        );
    }

    #[test]
    fn different_resources_get_different_fingerprints() {
        let r1 = Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "a")])
            .build();
        let r2 = Resource::builder_empty()
            .with_attributes([KeyValue::new("service.name", "b")])
            .build();
        assert_ne!(resource_fingerprint(&r1), resource_fingerprint(&r2));
    }
}
