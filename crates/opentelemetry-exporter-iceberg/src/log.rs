//! OpenTelemetry log exporter that writes to an Apache Iceberg table.

use std::sync::{Arc, RwLock};

use anyhow::Result;
use arrow_array::{ArrayRef, Int32Array, RecordBatch, StringArray, TimestampMicrosecondArray};
use arrow_schema::Schema as ArrowSchema;
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
use opentelemetry::Key;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::logs::{LogBatch, LogExporter};
use opentelemetry_semantic_conventions::attribute::SERVICE_NAME;
use parquet::file::properties::WriterProperties;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::catalog::{CatalogRef, build_catalog, build_file_io, ensure_namespace};
use crate::partition::{batch_partition_key, partition_spec};
use crate::schema::{LOGS_TIMESTAMP_FIELD_ID, arc, logs_schema};

/// OpenTelemetry log exporter that persists log records into an Apache Iceberg
/// `assistant_logs` table using Parquet files.
///
/// `BatchLogProcessor` runs on a plain OS thread (not a Tokio task), so we
/// capture the runtime [`Handle`] at construction time and use
/// [`Handle::block_on`] when there is no ambient executor.
#[derive(Clone, Debug)]
pub struct IcebergLogExporter {
    catalog: CatalogRef,
    config: IcebergConfig,
    resource_attributes: Arc<RwLock<Option<String>>>,
    service_name: Arc<RwLock<Option<String>>>,
    rt_handle: Handle,
    table: Arc<Mutex<iceberg::table::Table>>,
}

impl IcebergLogExporter {
    /// Initialise the exporter, creating the Iceberg namespace and table if
    /// they do not already exist.
    ///
    /// # Panics
    /// Panics when called outside a Tokio runtime context.
    pub async fn new(config: IcebergConfig) -> Result<Self> {
        let catalog = build_catalog(&config).await?;
        let ns = ensure_namespace(catalog.as_ref(), &config.namespace).await?;
        let table_name = "assistant_logs";
        let table_ident = TableIdent::new(ns.clone(), table_name.to_string());

        let schema = logs_schema()?;
        let partition =
            partition_spec(&config.partition, LOGS_TIMESTAMP_FIELD_ID, "timestamp_day")?;

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
            resource_attributes: Arc::new(RwLock::new(None)),
            service_name: Arc::new(RwLock::new(None)),
            rt_handle: Handle::current(),
            table: Arc::new(Mutex::new(table)),
        })
    }

    async fn export_inner(&self, batch: LogBatch<'_>) -> OTelSdkResult {
        let mut count = 0usize;
        let mut first_timestamp_micros: i64 = 0;
        for (record, _) in batch.iter() {
            if count == 0 {
                first_timestamp_micros = record
                    .timestamp()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_micros() as i64)
                            .unwrap_or(0)
                    })
                    .unwrap_or(0);
            }
            count += 1;
        }
        if count == 0 {
            return Ok(());
        }

        let resource_attrs = self.resource_attributes.read().ok().and_then(|g| g.clone());
        let service_name = self.service_name.read().ok().and_then(|g| g.clone());

        let iceberg_schema =
            logs_schema().map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;
        let arrow_schema = iceberg::arrow::schema_to_arrow_schema(&iceberg_schema)
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;
        let record_batch = logs_to_record_batch(
            &arrow_schema,
            batch,
            resource_attrs.as_deref(),
            service_name.as_deref(),
        )
        .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        let mut guard = self.table.lock().await;
        let table = &*guard;

        let partition_key =
            batch_partition_key(table, first_timestamp_micros, &self.config.partition);

        let file_io = build_file_io(&self.config);
        let schema_ref =
            arc(logs_schema().map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?);

        let location_gen = DefaultLocationGenerator::new(table.metadata())
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

impl LogExporter for IcebergLogExporter {
    async fn export(&self, batch: LogBatch<'_>) -> OTelSdkResult {
        if Handle::try_current().is_ok() {
            self.export_inner(batch).await
        } else {
            // BatchLogProcessor runs on a plain OS thread — use captured handle.
            self.rt_handle.block_on(self.export_inner(batch))
        }
    }

    fn set_resource(&mut self, resource: &Resource) {
        let svc = resource.get(&Key::new(SERVICE_NAME)).map(|v| v.to_string());
        if let Ok(mut guard) = self.service_name.write() {
            *guard = svc;
        }
        let attrs: serde_json::Map<String, serde_json::Value> = resource
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
            .collect();
        let json = serde_json::Value::Object(attrs).to_string();
        if let Ok(mut guard) = self.resource_attributes.write() {
            *guard = Some(json);
        }
    }
}

fn logs_to_record_batch(
    schema: &ArrowSchema,
    batch: LogBatch<'_>,
    resource_attributes: Option<&str>,
    service_name: Option<&str>,
) -> Result<RecordBatch> {
    use opentelemetry::logs::AnyValue;

    let mut ids = Vec::new();
    let mut timestamps: Vec<i64> = Vec::new();
    let mut severity_numbers: Vec<i32> = Vec::new();
    let mut severity_texts: Vec<Option<String>> = Vec::new();
    let mut bodies: Vec<Option<String>> = Vec::new();
    let mut trace_ids: Vec<Option<String>> = Vec::new();
    let mut span_ids: Vec<Option<String>> = Vec::new();
    let mut targets: Vec<Option<String>> = Vec::new();
    let mut service_names: Vec<Option<String>> = Vec::new();
    let mut attributes_vals: Vec<Option<String>> = Vec::new();
    let mut resource_attrs_vals: Vec<Option<String>> = Vec::new();

    for (record, _instrumentation) in batch.iter() {
        ids.push(Uuid::new_v4().to_string());

        let ts_us = record
            .timestamp()
            .or_else(|| record.observed_timestamp())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_micros() as i64)
            .unwrap_or(0);
        timestamps.push(ts_us);

        let sev_num = record.severity_number().map(|s| s as i32).unwrap_or(0);
        severity_numbers.push(sev_num);

        severity_texts.push(record.severity_text().map(str::to_string));

        let body = record.body().map(|v| match v {
            AnyValue::String(s) => s.to_string(),
            other => format!("{other:?}"),
        });
        bodies.push(body);

        let (trace_id, span_id) = record
            .trace_context()
            .map(|tc| {
                (
                    Some(format!("{:032x}", tc.trace_id)),
                    Some(format!("{:016x}", tc.span_id)),
                )
            })
            .unwrap_or((None, None));
        trace_ids.push(trace_id);
        span_ids.push(span_id);

        // Extract target from attributes (tracing bridge sets `code.namespace`)
        let attrs_map: serde_json::Map<String, serde_json::Value> = record
            .attributes_iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::String(format!("{v:?}"))))
            .collect();
        let target = attrs_map
            .get("code.namespace")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        targets.push(target);
        service_names.push(service_name.map(str::to_string));
        attributes_vals.push(serde_json::to_string(&attrs_map).ok());
        resource_attrs_vals.push(resource_attributes.map(str::to_string));
    }

    let schema_ref = Arc::new(schema.clone());
    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(ids)),
        Arc::new(TimestampMicrosecondArray::from(timestamps).with_timezone("+00:00")),
        Arc::new(Int32Array::from(severity_numbers)),
        Arc::new(StringArray::from(severity_texts)),
        Arc::new(StringArray::from(bodies)),
        Arc::new(StringArray::from(trace_ids)),
        Arc::new(StringArray::from(span_ids)),
        Arc::new(StringArray::from(targets)),
        Arc::new(StringArray::from(service_names)),
        Arc::new(StringArray::from(attributes_vals)),
        Arc::new(StringArray::from(resource_attrs_vals)),
    ];

    RecordBatch::try_new(schema_ref, columns)
        .map_err(|e| anyhow::anyhow!("failed to create logs RecordBatch: {e}"))
}
