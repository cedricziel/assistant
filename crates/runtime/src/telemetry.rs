use anyhow::Result;
use assistant_storage::{SqliteLogExporter, SqliteSpanExporter};
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    logs::{BatchLogProcessor, LoggerProvider},
    resource::Resource,
    runtime::Tokio,
    trace::{self, BatchSpanProcessor},
};
use sqlx::SqlitePool;
use tracing_subscriber::{
    filter::Targets, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

/// Guard that shuts down the global tracer provider and logger provider when
/// dropped.
pub struct OtelGuard {
    logger_provider: Option<LoggerProvider>,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Some(ref provider) = self.logger_provider {
            let _ = provider.shutdown();
        }
        global::shutdown_tracer_provider();
    }
}

/// Build the per-layer `Targets` filter for the OTel log bridge.
///
/// This filter suppresses all `sqlx*` targets to prevent a feedback loop:
///
///   tracing event → bridge → BatchLogProcessor → SqliteLogExporter
///     → sqlx INSERT INTO logs → sqlx emits tracing event → bridge → ∞
///
/// Application targets pass through at DEBUG and above.
pub(crate) fn otel_log_bridge_filter() -> Targets {
    Targets::new()
        .with_default(tracing::Level::DEBUG)
        .with_target("sqlx", tracing::Level::ERROR)
        .with_target("sqlx::query", tracing::metadata::LevelFilter::OFF)
        .with_target("sqlx_core", tracing::metadata::LevelFilter::OFF)
        .with_target("sqlx_sqlite", tracing::metadata::LevelFilter::OFF)
}

/// Install tracing subscribers and OpenTelemetry exporters.
///
/// `enable_sqlite_export` controls whether spans **and logs** are persisted
/// locally via SQLite exporters. Setting the `OTEL_EXPORTER_OTLP_ENDPOINT`
/// environment variable additionally wires up a remote OTLP exporter for
/// traces.
///
/// The OTel log bridge uses a dedicated per-layer filter (see
/// [`otel_log_bridge_filter`]) that suppresses all `sqlx` targets. Without
/// this, the log exporter's own INSERT queries would emit tracing events that
/// get captured by the bridge, creating a feedback loop.
pub fn init_tracing(pool: SqlitePool, enable_sqlite_export: bool) -> Result<Option<OtelGuard>> {
    let fmt_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);

    // -- Trace provider --
    let resource = Resource::new(vec![KeyValue::new("service.name", "assistant")]);

    let mut trace_provider_builder = trace::TracerProvider::builder()
        .with_config(trace::Config::default().with_resource(resource.clone()));
    let mut have_trace_exporter = false;

    if enable_sqlite_export {
        let sqlite_exporter = SqliteSpanExporter::new(pool.clone());
        let processor = BatchSpanProcessor::builder(sqlite_exporter, Tokio).build();
        trace_provider_builder = trace_provider_builder.with_span_processor(processor);
        have_trace_exporter = true;
    }

    if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        let otlp_exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint)
            .build_span_exporter()?;
        let processor = BatchSpanProcessor::builder(otlp_exporter, Tokio).build();
        trace_provider_builder = trace_provider_builder.with_span_processor(processor);
        have_trace_exporter = true;
    }

    // -- Logger provider (OTel logs) --
    let mut logger_provider: Option<LoggerProvider> = None;

    if enable_sqlite_export {
        let sqlite_log_exporter = SqliteLogExporter::new(pool);
        let log_processor = BatchLogProcessor::builder(sqlite_log_exporter, Tokio).build();
        let provider = LoggerProvider::builder()
            .with_log_processor(log_processor)
            .with_resource(resource)
            .build();

        // Bridge tracing events → OTel log records with the anti-stampede filter.
        let otel_filter = otel_log_bridge_filter();
        let otel_log_layer = OpenTelemetryTracingBridge::new(&provider).with_filter(otel_filter);
        logger_provider = Some(provider);

        tracing_subscriber::registry()
            .with(fmt_layer.with_filter(fmt_filter))
            .with(otel_log_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(fmt_layer.with_filter(fmt_filter))
            .init();
    }

    if have_trace_exporter {
        let provider = trace_provider_builder.build();
        global::set_tracer_provider(provider);
        Ok(Some(OtelGuard { logger_provider }))
    } else if logger_provider.is_some() {
        Ok(Some(OtelGuard { logger_provider }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The filter must block all sqlx query-level events regardless of level.
    /// These are the events emitted by every SQL statement the exporter runs.
    #[test]
    fn filter_blocks_sqlx_query_at_all_levels() {
        let filter = otel_log_bridge_filter();

        assert!(
            !filter.would_enable("sqlx::query", &tracing::Level::TRACE),
            "sqlx::query TRACE must be blocked"
        );
        assert!(
            !filter.would_enable("sqlx::query", &tracing::Level::DEBUG),
            "sqlx::query DEBUG must be blocked"
        );
        assert!(
            !filter.would_enable("sqlx::query", &tracing::Level::INFO),
            "sqlx::query INFO must be blocked"
        );
        assert!(
            !filter.would_enable("sqlx::query", &tracing::Level::WARN),
            "sqlx::query WARN must be blocked (slow query path)"
        );
        assert!(
            !filter.would_enable("sqlx::query", &tracing::Level::ERROR),
            "sqlx::query ERROR must be blocked"
        );
    }

    /// sqlx_core and sqlx_sqlite internal modules must be fully suppressed.
    #[test]
    fn filter_blocks_sqlx_internals() {
        let filter = otel_log_bridge_filter();

        for target in &[
            "sqlx_core",
            "sqlx_core::pool::connection",
            "sqlx_core::pool::inner",
            "sqlx_sqlite",
            "sqlx_sqlite::connection::worker",
        ] {
            assert!(
                !filter.would_enable(target, &tracing::Level::WARN),
                "{target} WARN must be blocked"
            );
            assert!(
                !filter.would_enable(target, &tracing::Level::ERROR),
                "{target} ERROR must be blocked"
            );
        }
    }

    /// The top-level `sqlx` target only allows ERROR through (as a safety
    /// valve for truly catastrophic messages). Everything below ERROR is
    /// blocked.
    #[test]
    fn filter_blocks_sqlx_below_error() {
        let filter = otel_log_bridge_filter();

        assert!(
            !filter.would_enable("sqlx", &tracing::Level::DEBUG),
            "sqlx DEBUG must be blocked"
        );
        assert!(
            !filter.would_enable("sqlx", &tracing::Level::INFO),
            "sqlx INFO must be blocked"
        );
        assert!(
            !filter.would_enable("sqlx", &tracing::Level::WARN),
            "sqlx WARN must be blocked"
        );
    }

    /// Application targets must pass through at DEBUG and above.
    #[test]
    fn filter_passes_application_targets() {
        let filter = otel_log_bridge_filter();

        for target in &[
            "assistant_runtime",
            "assistant_runtime::orchestrator",
            "assistant_tool_executor",
            "assistant_storage::traces",
            "assistant_llm::client",
        ] {
            assert!(
                filter.would_enable(target, &tracing::Level::DEBUG),
                "{target} DEBUG must pass"
            );
            assert!(
                filter.would_enable(target, &tracing::Level::INFO),
                "{target} INFO must pass"
            );
            assert!(
                filter.would_enable(target, &tracing::Level::WARN),
                "{target} WARN must pass"
            );
            assert!(
                filter.would_enable(target, &tracing::Level::ERROR),
                "{target} ERROR must pass"
            );
        }
    }

    /// TRACE-level events from application targets are not forwarded (the
    /// default is DEBUG).
    #[test]
    fn filter_blocks_trace_level_for_app() {
        let filter = otel_log_bridge_filter();

        assert!(
            !filter.would_enable("assistant_runtime", &tracing::Level::TRACE),
            "TRACE should not pass (default is DEBUG)"
        );
    }
}
