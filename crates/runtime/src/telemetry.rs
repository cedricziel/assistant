use anyhow::Result;
use assistant_storage::SqliteSpanExporter;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    resource::Resource,
    runtime::Tokio,
    trace::{self, BatchSpanProcessor},
};
use sqlx::SqlitePool;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Guard that shuts down the global tracer provider when dropped.
pub struct OtelGuard;

impl Drop for OtelGuard {
    fn drop(&mut self) {
        global::shutdown_tracer_provider();
    }
}

/// Install tracing subscribers and OpenTelemetry exporters.
///
/// `enable_sqlite_export` controls whether spans are persisted locally via the
/// SQLite exporter. Setting the `OTEL_EXPORTER_OTLP_ENDPOINT` environment
/// variable additionally wires up a remote OTLP exporter.
pub fn init_tracing(pool: SqlitePool, enable_sqlite_export: bool) -> Result<Option<OtelGuard>> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    let mut provider_builder =
        trace::TracerProvider::builder().with_config(trace::Config::default().with_resource(
            Resource::new(vec![KeyValue::new("service.name", "assistant")]),
        ));
    let mut have_exporter = false;

    if enable_sqlite_export {
        let sqlite_exporter = SqliteSpanExporter::new(pool);
        let processor = BatchSpanProcessor::builder(sqlite_exporter, Tokio).build();
        provider_builder = provider_builder.with_span_processor(processor);
        have_exporter = true;
    }

    if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        let otlp_exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint)
            .build_span_exporter()?;
        let processor = BatchSpanProcessor::builder(otlp_exporter, Tokio).build();
        provider_builder = provider_builder.with_span_processor(processor);
        have_exporter = true;
    }

    if have_exporter {
        let provider = provider_builder.build();
        global::set_tracer_provider(provider);
        Ok(Some(OtelGuard))
    } else {
        Ok(None)
    }
}
