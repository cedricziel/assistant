//! OpenTelemetry metric exporter that writes to an Apache Iceberg table.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use arrow_array::{
    ArrayRef, Float64Array, Int64Array, RecordBatch, StringArray, TimestampMicrosecondArray,
};
use arrow_schema::Schema as ArrowSchema;
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::types::observability::IcebergConfig;
use iceberg::spec::DataFileFormat;
use iceberg::transaction::{ApplyTransactionAction, Transaction};
use iceberg::writer::IcebergWriter;
use iceberg::writer::IcebergWriterBuilder;
use iceberg::writer::base_writer::data_file_writer::DataFileWriterBuilder;
use iceberg::writer::file_writer::ParquetWriterBuilder;
use iceberg::writer::file_writer::location_generator::{
    DefaultFileNameGenerator, DefaultLocationGenerator,
};
use iceberg::writer::file_writer::rolling_writer::RollingFileWriterBuilder;
use iceberg::{TableCreation, TableIdent};
use opentelemetry::KeyValue;
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::metrics::Temporality;
use opentelemetry_sdk::metrics::data::{
    AggregatedMetrics, Gauge, Histogram, MetricData, ResourceMetrics, Sum,
};
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;
use parquet::file::properties::WriterProperties;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::catalog::{CatalogRef, build_catalog, build_file_io, ensure_namespace};
use crate::partition::{batch_partition_key, partition_spec};
use crate::schema::{METRICS_RECORDED_AT_FIELD_ID, arc, metrics_schema};

/// OpenTelemetry metric exporter that persists metric points into an Apache
/// Iceberg `assistant_metric_points` table using Parquet files.
#[derive(Clone, Debug)]
pub struct IcebergMetricExporter {
    catalog: CatalogRef,
    config: IcebergConfig,
    table: Arc<Mutex<iceberg::table::Table>>,
}

impl IcebergMetricExporter {
    /// Initialise the exporter, creating the Iceberg namespace and table if
    /// they do not already exist.
    ///
    /// # Panics
    /// Panics when called outside a Tokio runtime context.
    pub async fn new(config: IcebergConfig) -> Result<Self> {
        let catalog = build_catalog(&config).await?;
        let ns = ensure_namespace(catalog.as_ref(), &config.namespace).await?;
        let table_name = "assistant_metric_points";
        let table_ident = TableIdent::new(ns.clone(), table_name.to_string());

        let schema = metrics_schema()?;
        let partition = partition_spec(
            &config.partition,
            METRICS_RECORDED_AT_FIELD_ID,
            "recorded_at_day",
        )?;

        let table = if catalog.table_exists(&table_ident).await? {
            catalog.load_table(&table_ident).await?
        } else {
            let creation = if let Some(spec) = partition {
                TableCreation::builder()
                    .name(table_name.to_string())
                    .schema(schema)
                    .partition_spec(spec)
                    .build()
            } else {
                TableCreation::builder()
                    .name(table_name.to_string())
                    .schema(schema)
                    .build()
            };
            catalog.create_table(&ns, creation).await?
        };

        Ok(Self {
            catalog,
            config,
            table: Arc::new(Mutex::new(table)),
        })
    }

    async fn export_inner(&self, metrics: &ResourceMetrics) -> OTelSdkResult {
        // Serialize the resource attributes from the ResourceMetrics
        let resource_attrs: serde_json::Map<String, serde_json::Value> = metrics
            .resource()
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
            .collect();
        let resource_attrs_json = serde_json::Value::Object(resource_attrs).to_string();
        let resource_attrs_str = if resource_attrs_json == "{}" {
            None
        } else {
            Some(resource_attrs_json)
        };

        let rows = collect_rows(metrics);
        if rows.is_empty() {
            return Ok(());
        }

        // Capture representative timestamp from the first row before consuming.
        let first_recorded_at_micros = rows.first().map(|r| r.recorded_at_us).unwrap_or(0);

        let iceberg_schema =
            metrics_schema().map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;
        let arrow_schema = iceberg::arrow::schema_to_arrow_schema(&iceberg_schema)
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;
        let record_batch = rows_to_record_batch(&arrow_schema, rows, resource_attrs_str.as_deref())
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        let mut guard = self.table.lock().await;
        let table = &*guard;

        let partition_key =
            batch_partition_key(table, first_recorded_at_micros, &self.config.partition);

        let file_io = build_file_io(&self.config);
        let schema_ref =
            arc(metrics_schema().map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?);

        let location_gen = DefaultLocationGenerator::new(table.metadata().clone())
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;
        let filename_gen =
            DefaultFileNameGenerator::new("part".to_string(), None, DataFileFormat::Parquet);

        let parquet_builder = ParquetWriterBuilder::new(WriterProperties::default(), schema_ref);
        let rolling_builder = RollingFileWriterBuilder::new_with_default_file_size(
            parquet_builder,
            file_io,
            location_gen,
            filename_gen,
        );
        let data_file_builder = DataFileWriterBuilder::new(rolling_builder);
        let mut writer = data_file_builder
            .build(partition_key)
            .await
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        writer
            .write(record_batch)
            .await
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        let data_files = writer
            .close()
            .await
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        if data_files.is_empty() {
            return Ok(());
        }

        let tx = Transaction::new(table);
        let action = tx.fast_append().add_data_files(data_files);
        let tx = action
            .apply(tx)
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;
        let new_table = tx
            .commit(self.catalog.as_ref())
            .await
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        *guard = new_table;
        Ok(())
    }
}

impl PushMetricExporter for IcebergMetricExporter {
    async fn export(&self, metrics: &ResourceMetrics) -> OTelSdkResult {
        self.export_inner(metrics).await
    }

    fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }

    fn shutdown_with_timeout(&self, _timeout: Duration) -> OTelSdkResult {
        Ok(())
    }

    fn temporality(&self) -> Temporality {
        Temporality::Delta
    }
}

// -- Row representation --------------------------------------------------

struct MetricRow {
    id: String,
    recorded_at_us: i64,
    metric_name: String,
    metric_kind: &'static str,
    unit: Option<String>,
    value: Option<f64>,
    count: Option<i64>,
    sum: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    bounds: Option<String>,
    bucket_counts: Option<String>,
    attributes: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    operation: Option<String>,
    skill: Option<String>,
    interface: Option<String>,
    agent_id: Option<String>,
    service_name: Option<String>,
    scope_name: Option<String>,
}

fn extract_hot_path(
    attrs: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    attrs.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn kv_attrs_map<'a>(
    kvs: impl Iterator<Item = &'a KeyValue>,
) -> serde_json::Map<String, serde_json::Value> {
    kvs.map(|kv| {
        (
            kv.key.to_string(),
            serde_json::Value::String(kv.value.to_string()),
        )
    })
    .collect()
}

fn sys_time_to_us(t: std::time::SystemTime, fallback: i64) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(fallback)
}

fn collect_rows(rm: &ResourceMetrics) -> Vec<MetricRow> {
    let mut rows = Vec::new();
    let now_us = SystemClock.now().timestamp_micros();

    for scope_metrics in rm.scope_metrics() {
        let scope_name = Some(scope_metrics.scope().name().to_string());

        for metric in scope_metrics.metrics() {
            let name = metric.name().to_string();
            let unit = {
                let u = metric.unit();
                if u.is_empty() {
                    None
                } else {
                    Some(u.to_string())
                }
            };

            match metric.data() {
                AggregatedMetrics::F64(MetricData::Sum(sum)) => {
                    append_sum_rows::<f64>(
                        sum,
                        &name,
                        "counter",
                        unit,
                        scope_name.clone(),
                        now_us,
                        &mut rows,
                    );
                }
                AggregatedMetrics::U64(MetricData::Sum(sum)) => {
                    let ts = sys_time_to_us(sum.time(), now_us);
                    for dp in sum.data_points() {
                        let attrs = kv_attrs_map(dp.attributes());
                        rows.push(make_scalar_row(
                            name.clone(),
                            "counter",
                            unit.clone(),
                            dp.value() as f64,
                            ts,
                            &attrs,
                            scope_name.clone(),
                        ));
                    }
                }
                AggregatedMetrics::I64(MetricData::Sum(sum)) => {
                    let ts = sys_time_to_us(sum.time(), now_us);
                    for dp in sum.data_points() {
                        let attrs = kv_attrs_map(dp.attributes());
                        rows.push(make_scalar_row(
                            name.clone(),
                            "counter",
                            unit.clone(),
                            dp.value() as f64,
                            ts,
                            &attrs,
                            scope_name.clone(),
                        ));
                    }
                }
                AggregatedMetrics::F64(MetricData::Gauge(gauge)) => {
                    append_gauge_rows::<f64>(
                        gauge,
                        &name,
                        unit,
                        scope_name.clone(),
                        now_us,
                        &mut rows,
                    );
                }
                AggregatedMetrics::U64(MetricData::Gauge(gauge)) => {
                    let ts = sys_time_to_us(gauge.time(), now_us);
                    for dp in gauge.data_points() {
                        let attrs = kv_attrs_map(dp.attributes());
                        rows.push(make_scalar_row(
                            name.clone(),
                            "gauge",
                            unit.clone(),
                            dp.value() as f64,
                            ts,
                            &attrs,
                            scope_name.clone(),
                        ));
                    }
                }
                AggregatedMetrics::I64(MetricData::Gauge(gauge)) => {
                    let ts = sys_time_to_us(gauge.time(), now_us);
                    for dp in gauge.data_points() {
                        let attrs = kv_attrs_map(dp.attributes());
                        rows.push(make_scalar_row(
                            name.clone(),
                            "gauge",
                            unit.clone(),
                            dp.value() as f64,
                            ts,
                            &attrs,
                            scope_name.clone(),
                        ));
                    }
                }
                AggregatedMetrics::F64(MetricData::Histogram(hist)) => {
                    append_histogram_rows(hist, &name, unit, scope_name.clone(), now_us, &mut rows);
                }
                // ExponentialHistogram and unhandled variants are skipped with a warning.
                _ => {
                    tracing::warn!(
                        metric_name = %name,
                        "unsupported metric aggregation shape, rows skipped"
                    );
                }
            }
        }
    }
    rows
}

fn append_sum_rows<T: Copy + Into<f64>>(
    sum: &Sum<T>,
    name: &str,
    kind: &'static str,
    unit: Option<String>,
    scope_name: Option<String>,
    now_us: i64,
    rows: &mut Vec<MetricRow>,
) {
    let ts = sys_time_to_us(sum.time(), now_us);
    for dp in sum.data_points() {
        let attrs = kv_attrs_map(dp.attributes());
        rows.push(make_scalar_row(
            name.to_string(),
            kind,
            unit.clone(),
            dp.value().into(),
            ts,
            &attrs,
            scope_name.clone(),
        ));
    }
}

fn append_gauge_rows<T: Copy + Into<f64>>(
    gauge: &Gauge<T>,
    name: &str,
    unit: Option<String>,
    scope_name: Option<String>,
    now_us: i64,
    rows: &mut Vec<MetricRow>,
) {
    let ts = sys_time_to_us(gauge.time(), now_us);
    for dp in gauge.data_points() {
        let attrs = kv_attrs_map(dp.attributes());
        rows.push(make_scalar_row(
            name.to_string(),
            "gauge",
            unit.clone(),
            dp.value().into(),
            ts,
            &attrs,
            scope_name.clone(),
        ));
    }
}

fn append_histogram_rows(
    hist: &Histogram<f64>,
    name: &str,
    unit: Option<String>,
    scope_name: Option<String>,
    now_us: i64,
    rows: &mut Vec<MetricRow>,
) {
    let ts = sys_time_to_us(hist.time(), now_us);
    for dp in hist.data_points() {
        let attrs = kv_attrs_map(dp.attributes());
        let bounds: Vec<f64> = dp.bounds().collect();
        let bucket_counts: Vec<u64> = dp.bucket_counts().collect();
        rows.push(MetricRow {
            id: Uuid::new_v4().to_string(),
            recorded_at_us: ts,
            metric_name: name.to_string(),
            metric_kind: "histogram",
            unit: unit.clone(),
            value: None,
            count: Some(dp.count() as i64),
            sum: Some(dp.sum()),
            min: dp.min(),
            max: dp.max(),
            bounds: serde_json::to_string(&bounds).ok(),
            bucket_counts: serde_json::to_string(&bucket_counts).ok(),
            attributes: serde_json::to_string(&attrs).ok(),
            model: extract_hot_path(&attrs, "gen_ai.request.model"),
            provider: extract_hot_path(&attrs, "gen_ai.provider.name"),
            operation: extract_hot_path(&attrs, "gen_ai.operation.name"),
            skill: extract_hot_path(&attrs, "skill"),
            interface: extract_hot_path(&attrs, "interface"),
            agent_id: extract_hot_path(&attrs, "agent.id"),
            service_name: None,
            scope_name: scope_name.clone(),
        });
    }
}

fn make_scalar_row(
    name: String,
    kind: &'static str,
    unit: Option<String>,
    value: f64,
    recorded_at_us: i64,
    attrs: &serde_json::Map<String, serde_json::Value>,
    scope_name: Option<String>,
) -> MetricRow {
    MetricRow {
        id: Uuid::new_v4().to_string(),
        recorded_at_us,
        metric_name: name,
        metric_kind: kind,
        unit,
        value: Some(value),
        count: None,
        sum: None,
        min: None,
        max: None,
        bounds: None,
        bucket_counts: None,
        attributes: serde_json::to_string(attrs).ok(),
        model: extract_hot_path(attrs, "gen_ai.request.model"),
        provider: extract_hot_path(attrs, "gen_ai.provider.name"),
        operation: extract_hot_path(attrs, "gen_ai.operation.name"),
        skill: extract_hot_path(attrs, "skill"),
        interface: extract_hot_path(attrs, "interface"),
        agent_id: extract_hot_path(attrs, "agent.id"),
        service_name: None,
        scope_name,
    }
}

fn rows_to_record_batch(
    schema: &ArrowSchema,
    rows: Vec<MetricRow>,
    resource_attributes: Option<&str>,
) -> Result<RecordBatch> {
    let len = rows.len();
    let mut ids = Vec::with_capacity(len);
    let mut recorded_ats: Vec<i64> = Vec::with_capacity(len);
    let mut metric_names = Vec::with_capacity(len);
    let mut metric_kinds = Vec::with_capacity(len);
    let mut units: Vec<Option<String>> = Vec::with_capacity(len);
    let mut values: Vec<Option<f64>> = Vec::with_capacity(len);
    let mut counts: Vec<Option<i64>> = Vec::with_capacity(len);
    let mut sums: Vec<Option<f64>> = Vec::with_capacity(len);
    let mut mins: Vec<Option<f64>> = Vec::with_capacity(len);
    let mut maxs: Vec<Option<f64>> = Vec::with_capacity(len);
    let mut bounds_vals: Vec<Option<String>> = Vec::with_capacity(len);
    let mut bucket_counts_vals: Vec<Option<String>> = Vec::with_capacity(len);
    let mut attributes_vals: Vec<Option<String>> = Vec::with_capacity(len);
    let mut models: Vec<Option<String>> = Vec::with_capacity(len);
    let mut providers: Vec<Option<String>> = Vec::with_capacity(len);
    let mut operations: Vec<Option<String>> = Vec::with_capacity(len);
    let mut skills: Vec<Option<String>> = Vec::with_capacity(len);
    let mut interfaces: Vec<Option<String>> = Vec::with_capacity(len);
    let mut agent_ids: Vec<Option<String>> = Vec::with_capacity(len);
    let mut service_names: Vec<Option<String>> = Vec::with_capacity(len);
    let mut scope_names: Vec<Option<String>> = Vec::with_capacity(len);
    let mut resource_attrs_vals: Vec<Option<String>> = Vec::with_capacity(len);

    for row in rows {
        ids.push(row.id);
        recorded_ats.push(row.recorded_at_us);
        metric_names.push(row.metric_name);
        metric_kinds.push(row.metric_kind.to_string());
        units.push(row.unit);
        values.push(row.value);
        counts.push(row.count);
        sums.push(row.sum);
        mins.push(row.min);
        maxs.push(row.max);
        bounds_vals.push(row.bounds);
        bucket_counts_vals.push(row.bucket_counts);
        attributes_vals.push(row.attributes);
        models.push(row.model);
        providers.push(row.provider);
        operations.push(row.operation);
        skills.push(row.skill);
        interfaces.push(row.interface);
        agent_ids.push(row.agent_id);
        service_names.push(row.service_name);
        scope_names.push(row.scope_name);
        resource_attrs_vals.push(resource_attributes.map(str::to_string));
    }

    let schema_ref = Arc::new(schema.clone());
    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(ids)),
        Arc::new(TimestampMicrosecondArray::from(recorded_ats).with_timezone("+00:00")),
        Arc::new(StringArray::from(metric_names)),
        Arc::new(StringArray::from(metric_kinds)),
        Arc::new(StringArray::from(units)),
        Arc::new(Float64Array::from(values)),
        Arc::new(Int64Array::from(counts)),
        Arc::new(Float64Array::from(sums)),
        Arc::new(Float64Array::from(mins)),
        Arc::new(Float64Array::from(maxs)),
        Arc::new(StringArray::from(bounds_vals)),
        Arc::new(StringArray::from(bucket_counts_vals)),
        Arc::new(StringArray::from(attributes_vals)),
        Arc::new(StringArray::from(models)),
        Arc::new(StringArray::from(providers)),
        Arc::new(StringArray::from(operations)),
        Arc::new(StringArray::from(skills)),
        Arc::new(StringArray::from(interfaces)),
        Arc::new(StringArray::from(agent_ids)),
        Arc::new(StringArray::from(service_names)),
        Arc::new(StringArray::from(scope_names)),
        Arc::new(StringArray::from(resource_attrs_vals)),
    ];

    RecordBatch::try_new(schema_ref, columns)
        .map_err(|e| anyhow::anyhow!("failed to create metrics RecordBatch: {e}"))
}
