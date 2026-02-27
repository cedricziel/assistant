//! OpenTelemetry metric exporter that persists data points into SQLite.
//!
//! Resources and instrumentation scopes are normalized into dedicated
//! join tables (`resources`, `metric_scopes`) to avoid duplicating
//! identical metadata on every data-point row.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use opentelemetry::metrics::MetricsError;
use opentelemetry::{KeyValue, Value};
use opentelemetry_sdk::metrics::data::{self, ResourceMetrics, Temporality};
use opentelemetry_sdk::metrics::exporter::PushMetricsExporter;
use opentelemetry_sdk::metrics::reader::{
    AggregationSelector, DefaultAggregationSelector, TemporalitySelector,
};
use opentelemetry_sdk::metrics::{Aggregation, InstrumentKind};
use serde_json::{Map, Number};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tracing::warn;

/// OpenTelemetry metric exporter that persists data points into the
/// `metric_points` SQLite table, with resources and scopes normalized
/// into join tables.
#[derive(Clone, Debug)]
pub struct SqliteMetricExporter {
    pool: SqlitePool,
}

impl SqliteMetricExporter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Upsert the resource and return its row id.
    async fn ensure_resource(
        &self,
        resource: &opentelemetry_sdk::Resource,
    ) -> Result<i64, sqlx::Error> {
        let fingerprint = resource_fingerprint(resource);
        let attrs_json = resource_to_json(resource);

        sqlx::query("INSERT OR IGNORE INTO resources (fingerprint, attributes) VALUES (?1, ?2)")
            .bind(&fingerprint)
            .bind(&attrs_json)
            .execute(&self.pool)
            .await?;

        let id: i64 = sqlx::query_scalar("SELECT id FROM resources WHERE fingerprint = ?1")
            .bind(&fingerprint)
            .fetch_one(&self.pool)
            .await?;

        Ok(id)
    }

    /// Upsert the instrumentation scope and return its row id.
    async fn ensure_scope(
        &self,
        scope: &opentelemetry::InstrumentationLibrary,
    ) -> Result<i64, sqlx::Error> {
        let name = scope.name.as_ref();
        let version = scope.version.as_deref().unwrap_or("");

        sqlx::query("INSERT OR IGNORE INTO metric_scopes (name, version) VALUES (?1, ?2)")
            .bind(name)
            .bind(version)
            .execute(&self.pool)
            .await?;

        let id: i64 =
            sqlx::query_scalar("SELECT id FROM metric_scopes WHERE name = ?1 AND version = ?2")
                .bind(name)
                .bind(version)
                .fetch_one(&self.pool)
                .await?;

        Ok(id)
    }

    /// Persist a single counter/gauge data point (f64).
    #[allow(clippy::too_many_arguments)]
    async fn insert_scalar(
        &self,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        kind: &str,
        unit: &str,
        description: &str,
        dp: &data::DataPoint<f64>,
    ) -> Result<(), sqlx::Error> {
        let da = extract_denormalized(&dp.attributes);
        let attrs_json = keyvalues_to_json(&dp.attributes);
        let start_time = non_epoch_time(dp.start_time);
        let recorded_at = resolve_time(dp.time);

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
        .bind(dp.value)
        .bind(&attrs_json)
        .bind(start_time)
        .bind(recorded_at)
        .bind(&da.model)
        .bind(&da.provider)
        .bind(&da.operation)
        .bind(&da.skill)
        .bind(&da.interface)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Persist a single counter/gauge data point (i64).
    #[allow(clippy::too_many_arguments)]
    async fn insert_scalar_i64(
        &self,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        kind: &str,
        unit: &str,
        description: &str,
        dp: &data::DataPoint<i64>,
    ) -> Result<(), sqlx::Error> {
        let da = extract_denormalized(&dp.attributes);
        let attrs_json = keyvalues_to_json(&dp.attributes);
        let start_time = non_epoch_time(dp.start_time);
        let recorded_at = resolve_time(dp.time);

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
        .bind(dp.value as f64)
        .bind(&attrs_json)
        .bind(start_time)
        .bind(recorded_at)
        .bind(&da.model)
        .bind(&da.provider)
        .bind(&da.operation)
        .bind(&da.skill)
        .bind(&da.interface)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Persist a histogram data point.
    async fn insert_histogram(
        &self,
        resource_id: i64,
        scope_id: i64,
        name: &str,
        unit: &str,
        description: &str,
        dp: &data::HistogramDataPoint<f64>,
    ) -> Result<(), sqlx::Error> {
        let da = extract_denormalized(&dp.attributes);
        let attrs_json = keyvalues_to_json(&dp.attributes);
        let start_time = non_epoch_time(Some(dp.start_time));
        let recorded_at = resolve_time(Some(dp.time));
        let bounds_json = serde_json::to_string(&dp.bounds).unwrap_or_default();
        let bucket_counts_json = serde_json::to_string(&dp.bucket_counts).unwrap_or_default();

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
        .bind(dp.count as i64)
        .bind(dp.sum)
        .bind(dp.min)
        .bind(dp.max)
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
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

impl AggregationSelector for SqliteMetricExporter {
    fn aggregation(&self, kind: InstrumentKind) -> Aggregation {
        DefaultAggregationSelector::new().aggregation(kind)
    }
}

impl TemporalitySelector for SqliteMetricExporter {
    fn temporality(&self, _kind: InstrumentKind) -> Temporality {
        // Delta: each export row represents a single collection interval,
        // mapping naturally to time-series storage.
        Temporality::Delta
    }
}

#[async_trait]
impl PushMetricsExporter for SqliteMetricExporter {
    async fn export(&self, metrics: &mut ResourceMetrics) -> opentelemetry::metrics::Result<()> {
        let resource_id = self
            .ensure_resource(&metrics.resource)
            .await
            .map_err(|e| MetricsError::Other(e.to_string()))?;

        for scope_metrics in &metrics.scope_metrics {
            let scope_id = self
                .ensure_scope(&scope_metrics.scope)
                .await
                .map_err(|e| MetricsError::Other(e.to_string()))?;

            for metric in &scope_metrics.metrics {
                let name = &*metric.name;
                let unit = &*metric.unit;
                let desc = &*metric.description;

                // -- Counters / UpDownCounters (Sum aggregation) --
                if let Some(sum) = metric.data.as_any().downcast_ref::<data::Sum<f64>>() {
                    let kind = if sum.is_monotonic {
                        "counter"
                    } else {
                        "up_down_counter"
                    };
                    for dp in &sum.data_points {
                        if let Err(e) = self
                            .insert_scalar(resource_id, scope_id, name, kind, unit, desc, dp)
                            .await
                        {
                            warn!(metric = name, error = %e, "failed to persist f64 sum");
                        }
                    }
                } else if let Some(sum) = metric.data.as_any().downcast_ref::<data::Sum<i64>>() {
                    let kind = if sum.is_monotonic {
                        "counter"
                    } else {
                        "up_down_counter"
                    };
                    for dp in &sum.data_points {
                        if let Err(e) = self
                            .insert_scalar_i64(resource_id, scope_id, name, kind, unit, desc, dp)
                            .await
                        {
                            warn!(metric = name, error = %e, "failed to persist i64 sum");
                        }
                    }
                // -- Gauges --
                } else if let Some(gauge) = metric.data.as_any().downcast_ref::<data::Gauge<f64>>()
                {
                    for dp in &gauge.data_points {
                        if let Err(e) = self
                            .insert_scalar(resource_id, scope_id, name, "gauge", unit, desc, dp)
                            .await
                        {
                            warn!(metric = name, error = %e, "failed to persist f64 gauge");
                        }
                    }
                } else if let Some(gauge) = metric.data.as_any().downcast_ref::<data::Gauge<i64>>()
                {
                    for dp in &gauge.data_points {
                        if let Err(e) = self
                            .insert_scalar_i64(resource_id, scope_id, name, "gauge", unit, desc, dp)
                            .await
                        {
                            warn!(metric = name, error = %e, "failed to persist i64 gauge");
                        }
                    }
                // -- Histograms --
                } else if let Some(hist) =
                    metric.data.as_any().downcast_ref::<data::Histogram<f64>>()
                {
                    for dp in &hist.data_points {
                        if let Err(e) = self
                            .insert_histogram(resource_id, scope_id, name, unit, desc, dp)
                            .await
                        {
                            warn!(metric = name, error = %e, "failed to persist histogram");
                        }
                    }
                } else {
                    warn!(metric = name, "unsupported metric data type, skipping");
                }
            }
        }

        Ok(())
    }

    async fn force_flush(&self) -> opentelemetry::metrics::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> opentelemetry::metrics::Result<()> {
        Ok(())
    }
}

// -- Helpers ------------------------------------------------------------------

/// SHA-256 fingerprint of sorted resource attributes for deduplication.
fn resource_fingerprint(resource: &opentelemetry_sdk::Resource) -> String {
    let mut pairs: Vec<(String, String)> = resource
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), format!("{v:?}")))
        .collect();
    pairs.sort();

    let mut hasher = Sha256::new();
    for (k, v) in &pairs {
        hasher.update(k.as_bytes());
        hasher.update(b"=");
        hasher.update(v.as_bytes());
        hasher.update(b",");
    }
    format!("{:x}", hasher.finalize())
}

/// Serialize resource attributes to a JSON string (stored once per resource row).
fn resource_to_json(resource: &opentelemetry_sdk::Resource) -> String {
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

/// Resolve a data-point timestamp to a concrete `DateTime<Utc>`, falling
/// back to `Utc::now()` if the SDK left the field unset.
fn resolve_time(t: Option<std::time::SystemTime>) -> DateTime<Utc> {
    t.map(Into::into).unwrap_or_else(Utc::now)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    #[tokio::test]
    async fn resource_upsert_is_idempotent() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let exporter = SqliteMetricExporter::new(storage.pool.clone());

        let resource = opentelemetry_sdk::Resource::new(vec![
            KeyValue::new("service.name", "test"),
            KeyValue::new("service.version", "0.1.0"),
        ]);

        let id1 = exporter.ensure_resource(&resource).await.unwrap();
        let id2 = exporter.ensure_resource(&resource).await.unwrap();
        assert_eq!(id1, id2, "same resource must return same id");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resources")
            .fetch_one(&storage.pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "only one resource row should exist");
    }

    #[tokio::test]
    async fn scope_upsert_is_idempotent() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let exporter = SqliteMetricExporter::new(storage.pool.clone());

        let scope = opentelemetry::InstrumentationLibrary::builder("test-scope")
            .with_version("1.0")
            .build();

        let id1 = exporter.ensure_scope(&scope).await.unwrap();
        let id2 = exporter.ensure_scope(&scope).await.unwrap();
        assert_eq!(id1, id2, "same scope must return same id");
    }

    #[test]
    fn fingerprint_is_order_independent() {
        let r1 = opentelemetry_sdk::Resource::new(vec![
            KeyValue::new("a", "1"),
            KeyValue::new("b", "2"),
        ]);
        let r2 = opentelemetry_sdk::Resource::new(vec![
            KeyValue::new("b", "2"),
            KeyValue::new("a", "1"),
        ]);
        assert_eq!(
            resource_fingerprint(&r1),
            resource_fingerprint(&r2),
            "attribute order must not affect fingerprint"
        );
    }

    #[test]
    fn different_resources_get_different_fingerprints() {
        let r1 = opentelemetry_sdk::Resource::new(vec![KeyValue::new("service.name", "a")]);
        let r2 = opentelemetry_sdk::Resource::new(vec![KeyValue::new("service.name", "b")]);
        assert_ne!(resource_fingerprint(&r1), resource_fingerprint(&r2));
    }
}
