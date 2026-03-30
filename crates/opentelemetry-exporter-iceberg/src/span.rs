//! OpenTelemetry span exporter that writes to an Apache Iceberg table.

use std::sync::{Arc, RwLock};

use anyhow::Result;
use arrow_array::{ArrayRef, Int64Array, RecordBatch, StringArray, TimestampMicrosecondArray};
use arrow_schema::{DataType, Field, Schema as ArrowSchema, TimeUnit};
use assistant_core::IcebergConfig;
use iceberg::spec::DataFileFormat;
use iceberg::transaction::ApplyTransactionAction;
use iceberg::transaction::Transaction;
use iceberg::writer::base_writer::data_file_writer::DataFileWriterBuilder;
use iceberg::writer::file_writer::location_generator::{
    DefaultFileNameGenerator, DefaultLocationGenerator,
};
use iceberg::writer::file_writer::rolling_writer::RollingFileWriterBuilder;
use iceberg::writer::file_writer::ParquetWriterBuilder;
use iceberg::writer::IcebergWriter;
use iceberg::writer::IcebergWriterBuilder;
use iceberg::{TableCreation, TableIdent};
use opentelemetry::{Key, Value};
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::trace::{SpanData, SpanExporter};
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::attribute::{
    GEN_AI_USAGE_INPUT_TOKENS, GEN_AI_USAGE_OUTPUT_TOKENS, SERVICE_NAME,
};
use parquet::file::properties::WriterProperties;
use tokio::runtime::Handle;
use tokio::sync::Mutex;

use crate::catalog::{build_catalog, build_file_io, ensure_namespace, CatalogRef};
use crate::partition::partition_spec;
use crate::schema::{arc, spans_schema, SPANS_START_TIME_FIELD_ID};

/// OpenTelemetry span exporter that persists spans into an Apache Iceberg
/// `assistant_spans` table using Parquet files.
#[derive(Clone, Debug)]
pub struct IcebergSpanExporter {
    catalog: CatalogRef,
    config: IcebergConfig,
    resource_attributes: Arc<RwLock<Option<String>>>,
    service_name: Arc<RwLock<Option<String>>>,
    rt_handle: Handle,
    /// Mutex guards mutable access to the `Table` handle, which is updated
    /// after every committed transaction.
    table: Arc<Mutex<iceberg::table::Table>>,
}

impl IcebergSpanExporter {
    /// Initialise the exporter, creating the Iceberg namespace and table if
    /// they do not already exist.
    ///
    /// # Panics
    /// Panics when called outside a Tokio runtime context.
    pub async fn new(config: IcebergConfig) -> Result<Self> {
        let catalog = build_catalog(&config).await?;
        let ns = ensure_namespace(catalog.as_ref(), &config.namespace).await?;
        let table_name = "assistant_spans";
        let table_ident = TableIdent::new(ns.clone(), table_name.to_string());

        let schema = spans_schema()?;
        let partition = partition_spec(
            &config.partition,
            SPANS_START_TIME_FIELD_ID,
            "start_time_day",
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
            resource_attributes: Arc::new(RwLock::new(None)),
            service_name: Arc::new(RwLock::new(None)),
            rt_handle: Handle::current(),
            table: Arc::new(Mutex::new(table)),
        })
    }

    async fn export_inner(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        if batch.is_empty() {
            return Ok(());
        }

        let resource_attrs = self.resource_attributes.read().ok().and_then(|g| g.clone());
        let service_name = self.service_name.read().ok().and_then(|g| g.clone());

        let arrow_schema = arrow_schema();
        let record_batch = spans_to_record_batch(
            &arrow_schema,
            batch,
            resource_attrs.as_deref(),
            service_name.as_deref(),
        )
        .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        let mut guard = self.table.lock().await;
        let table = &*guard;

        let file_io = build_file_io(&self.config);
        let schema_ref =
            arc(spans_schema().map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?);

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
            .build(None)
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

impl SpanExporter for IcebergSpanExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        if Handle::try_current().is_ok() {
            self.export_inner(batch).await
        } else {
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

// -- Arrow schema (must match spans_schema field order) ------------------

fn arrow_schema() -> ArrowSchema {
    ArrowSchema::new(vec![
        Field::new("trace_id", DataType::Utf8, false),
        Field::new("span_id", DataType::Utf8, false),
        Field::new("parent_span_id", DataType::Utf8, true),
        Field::new("name", DataType::Utf8, false),
        Field::new(
            "start_time",
            DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new(
            "end_time",
            DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            true,
        ),
        Field::new("duration_ms", DataType::Int64, true),
        Field::new("service_name", DataType::Utf8, true),
        Field::new("conversation_id", DataType::Utf8, true),
        Field::new("tool_name", DataType::Utf8, true),
        Field::new("tool_status", DataType::Utf8, true),
        Field::new("input_tokens", DataType::Int64, true),
        Field::new("output_tokens", DataType::Int64, true),
        Field::new("attributes", DataType::Utf8, true),
        Field::new("resource_attributes", DataType::Utf8, true),
    ])
}

fn spans_to_record_batch(
    schema: &ArrowSchema,
    spans: Vec<SpanData>,
    resource_attributes: Option<&str>,
    service_name: Option<&str>,
) -> Result<RecordBatch> {
    use opentelemetry::trace::SpanId;

    let mut trace_ids = Vec::with_capacity(spans.len());
    let mut span_ids = Vec::with_capacity(spans.len());
    let mut parent_span_ids: Vec<Option<String>> = Vec::with_capacity(spans.len());
    let mut names = Vec::with_capacity(spans.len());
    let mut start_times: Vec<i64> = Vec::with_capacity(spans.len());
    let mut end_times: Vec<Option<i64>> = Vec::with_capacity(spans.len());
    let mut duration_ms_vals: Vec<Option<i64>> = Vec::with_capacity(spans.len());
    let mut service_names: Vec<Option<String>> = Vec::with_capacity(spans.len());
    let mut conversation_ids: Vec<Option<String>> = Vec::with_capacity(spans.len());
    let mut tool_names: Vec<Option<String>> = Vec::with_capacity(spans.len());
    let mut tool_statuses: Vec<Option<String>> = Vec::with_capacity(spans.len());
    let mut input_tokens_vals: Vec<Option<i64>> = Vec::with_capacity(spans.len());
    let mut output_tokens_vals: Vec<Option<i64>> = Vec::with_capacity(spans.len());
    let mut attributes_vals: Vec<Option<String>> = Vec::with_capacity(spans.len());
    let mut resource_attrs_vals: Vec<Option<String>> = Vec::with_capacity(spans.len());

    for span in spans {
        // Extract numeric token counts directly from OTel Value before stringifying.
        let input_tokens: Option<i64> = span
            .attributes
            .iter()
            .find(|kv| kv.key.as_str() == GEN_AI_USAGE_INPUT_TOKENS)
            .and_then(|kv| match &kv.value {
                Value::I64(v) => Some(*v),
                Value::F64(v) => Some(*v as i64),
                _ => None,
            });
        let output_tokens: Option<i64> = span
            .attributes
            .iter()
            .find(|kv| kv.key.as_str() == GEN_AI_USAGE_OUTPUT_TOKENS)
            .and_then(|kv| match &kv.value {
                Value::I64(v) => Some(*v),
                Value::F64(v) => Some(*v as i64),
                _ => None,
            });

        // Build the JSON attrs map for storage (all values stringified).
        let attrs_json_map: serde_json::Map<String, serde_json::Value> = span
            .attributes
            .iter()
            .map(|kv| {
                (
                    kv.key.to_string(),
                    serde_json::Value::String(kv.value.to_string()),
                )
            })
            .collect();

        let start_us = span
            .start_time
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as i64)
            .unwrap_or(0);
        let end_us = span
            .end_time
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as i64)
            .ok();
        let dur_ms = span
            .end_time
            .duration_since(span.start_time)
            .map(|d| d.as_millis() as i64)
            .ok();

        let parent = if span.parent_span_id == SpanId::INVALID {
            None
        } else {
            Some(span.parent_span_id.to_string())
        };

        let conversation_id = attrs_json_map
            .get("conversation_id")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let tool_name = attrs_json_map
            .get("tool_name")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let tool_status = attrs_json_map
            .get("tool_status")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let attrs_json = serde_json::to_string(&attrs_json_map).ok();

        trace_ids.push(span.span_context.trace_id().to_string());
        span_ids.push(span.span_context.span_id().to_string());
        parent_span_ids.push(parent);
        names.push(span.name.to_string());
        start_times.push(start_us);
        end_times.push(end_us);
        duration_ms_vals.push(dur_ms);
        service_names.push(service_name.map(str::to_string));
        conversation_ids.push(conversation_id);
        tool_names.push(tool_name);
        tool_statuses.push(tool_status);
        input_tokens_vals.push(input_tokens);
        output_tokens_vals.push(output_tokens);
        attributes_vals.push(attrs_json);
        resource_attrs_vals.push(resource_attributes.map(str::to_string));
    }

    let schema_ref = Arc::new(schema.clone());
    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(trace_ids)),
        Arc::new(StringArray::from(span_ids)),
        Arc::new(StringArray::from(parent_span_ids)),
        Arc::new(StringArray::from(names)),
        Arc::new(TimestampMicrosecondArray::from(start_times).with_timezone("UTC")),
        Arc::new(TimestampMicrosecondArray::from(end_times).with_timezone("UTC")),
        Arc::new(Int64Array::from(duration_ms_vals)),
        Arc::new(StringArray::from(service_names)),
        Arc::new(StringArray::from(conversation_ids)),
        Arc::new(StringArray::from(tool_names)),
        Arc::new(StringArray::from(tool_statuses)),
        Arc::new(Int64Array::from(input_tokens_vals)),
        Arc::new(Int64Array::from(output_tokens_vals)),
        Arc::new(StringArray::from(attributes_vals)),
        Arc::new(StringArray::from(resource_attrs_vals)),
    ];

    RecordBatch::try_new(schema_ref, columns)
        .map_err(|e| anyhow::anyhow!("failed to create spans RecordBatch: {e}"))
}
