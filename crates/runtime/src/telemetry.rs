use std::time::Duration;

use anyhow::Result;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_exporter_sqlite::{SqliteLogExporter, SqliteMetricExporter, SqliteSpanExporter};
use opentelemetry_sdk::{
    logs::{BatchLogProcessor, SdkLoggerProvider},
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::{BatchSpanProcessor, SdkTracerProvider},
    Resource,
};
use sqlx::SqlitePool;
use tracing::info;
use tracing_subscriber::{
    filter::Targets, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

/// Guard that shuts down all OTel providers when dropped.
pub struct OtelGuard {
    tracer_provider: Option<SdkTracerProvider>,
    logger_provider: Option<SdkLoggerProvider>,
    meter_provider: Option<SdkMeterProvider>,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Some(ref provider) = self.logger_provider {
            let _ = provider.shutdown();
        }
        if let Some(ref provider) = self.meter_provider {
            let _ = provider.shutdown();
        }
        if let Some(ref provider) = self.tracer_provider {
            let _ = provider.shutdown();
        }
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
/// `enable_sqlite_export` controls whether spans, logs, and metrics are
/// persisted locally via SQLite exporters.
///
/// Setting **any** `OTEL_EXPORTER_OTLP_*` environment variable wires up
/// remote OTLP exporters for all three signals (traces, logs, metrics).
/// The `opentelemetry-otlp` crate reads the standard env vars internally,
/// so all of the following are supported without additional code:
///
/// | Variable | Per-signal overrides |
/// |----------|---------------------|
/// | `OTEL_EXPORTER_OTLP_ENDPOINT` | `_TRACES_ENDPOINT`, `_LOGS_ENDPOINT`, `_METRICS_ENDPOINT` |
/// | `OTEL_EXPORTER_OTLP_HEADERS` | `_TRACES_HEADERS`, `_LOGS_HEADERS`, `_METRICS_HEADERS` |
/// | `OTEL_EXPORTER_OTLP_TIMEOUT` | `_TRACES_TIMEOUT`, `_LOGS_TIMEOUT`, `_METRICS_TIMEOUT` |
/// | `OTEL_EXPORTER_OTLP_COMPRESSION` | `_TRACES_COMPRESSION`, `_LOGS_COMPRESSION`, `_METRICS_COMPRESSION` |
///
/// Both SQLite and OTLP backends can run side-by-side — each OTel
/// provider simply gets multiple processors/readers.
///
/// The OTel log bridge uses a dedicated per-layer filter (see
/// [`otel_log_bridge_filter`]) that suppresses all `sqlx` targets. Without
/// this, the log exporter's own INSERT queries would emit tracing events that
/// get captured by the bridge, creating a feedback loop.
pub fn init_tracing(pool: SqlitePool, enable_sqlite_export: bool) -> Result<Option<OtelGuard>> {
    let fmt_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);

    // -- Shared resource (used by traces, logs, and metrics) --
    let resource = build_resource();

    // Detect whether the user wants OTLP export by checking for any of the
    // standard `OTEL_EXPORTER_OTLP_*` env vars.  We intentionally do NOT
    // read the endpoint value ourselves — the crate resolves per-signal
    // overrides, timeouts, headers, and compression internally.
    let enable_otlp = otlp_env_is_set();
    let need_otel = enable_sqlite_export || enable_otlp;

    if enable_otlp {
        info!("OTLP export enabled — the opentelemetry-otlp crate will read endpoint, headers, timeout, and compression from OTEL_EXPORTER_OTLP_* env vars");
    }

    // -- Trace provider --------------------------------------------------
    let mut trace_provider_builder = SdkTracerProvider::builder().with_resource(resource.clone());
    let mut have_trace_exporter = false;

    if enable_sqlite_export {
        let sqlite_exporter = SqliteSpanExporter::new(pool.clone());
        let processor = BatchSpanProcessor::builder(sqlite_exporter).build();
        trace_provider_builder = trace_provider_builder.with_span_processor(processor);
        have_trace_exporter = true;
    }

    if enable_otlp {
        // Let the crate resolve OTEL_EXPORTER_OTLP_TRACES_ENDPOINT (or the
        // generic fallback), headers, timeout, and compression from env vars.
        let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .build()?;
        trace_provider_builder = trace_provider_builder.with_batch_exporter(otlp_exporter);
        have_trace_exporter = true;
    }

    // -- Logger provider (OTel logs) -------------------------------------
    let mut logger_provider: Option<SdkLoggerProvider> = None;

    // -- Meter provider (OTel metrics) -----------------------------------
    let mut meter_provider: Option<SdkMeterProvider> = None;

    if need_otel {
        // Logs — attach SQLite and/or OTLP processors to the same provider.
        let mut log_builder = SdkLoggerProvider::builder().with_resource(resource.clone());

        if enable_sqlite_export {
            let sqlite_log_exporter = SqliteLogExporter::new(pool.clone());
            let processor = BatchLogProcessor::builder(sqlite_log_exporter).build();
            log_builder = log_builder.with_log_processor(processor);
        }

        if enable_otlp {
            let otlp_log_exporter = opentelemetry_otlp::LogExporter::builder()
                .with_tonic()
                .build()?;
            log_builder = log_builder.with_batch_exporter(otlp_log_exporter);
        }

        let log_prov = log_builder.build();

        // Bridge tracing events → OTel log records with the anti-stampede filter.
        let otel_filter = otel_log_bridge_filter();
        let otel_log_layer = OpenTelemetryTracingBridge::new(&log_prov).with_filter(otel_filter);
        logger_provider = Some(log_prov);

        // Metrics — attach SQLite and/or OTLP readers to the same provider.
        let mut meter_builder = SdkMeterProvider::builder().with_resource(resource);

        if enable_sqlite_export {
            let sqlite_metric_exporter = SqliteMetricExporter::new(pool);
            let reader = PeriodicReader::builder(sqlite_metric_exporter)
                .with_interval(Duration::from_secs(60))
                .build();
            meter_builder = meter_builder.with_reader(reader);
        }

        if enable_otlp {
            let otlp_metric_exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_tonic()
                .build()?;
            let reader = PeriodicReader::builder(otlp_metric_exporter)
                .with_interval(Duration::from_secs(60))
                .build();
            meter_builder = meter_builder.with_reader(reader);
        }

        let meter_prov = meter_builder.build();
        global::set_meter_provider(meter_prov.clone());
        meter_provider = Some(meter_prov);

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
        global::set_tracer_provider(provider.clone());
        Ok(Some(OtelGuard {
            tracer_provider: Some(provider),
            logger_provider,
            meter_provider,
        }))
    } else if logger_provider.is_some() || meter_provider.is_some() {
        Ok(Some(OtelGuard {
            tracer_provider: None,
            logger_provider,
            meter_provider,
        }))
    } else {
        Ok(None)
    }
}

/// Returns `true` when any `OTEL_EXPORTER_OTLP_*` env var is set, indicating
/// the user wants remote OTLP export.
///
/// We check the generic endpoint plus per-signal overrides so that setting
/// *only* `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` (without the generic one)
/// still activates the OTLP pipeline.
fn otlp_env_is_set() -> bool {
    const VARS: &[&str] = &[
        // Generic
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        // Per-signal endpoints
        "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        "OTEL_EXPORTER_OTLP_LOGS_ENDPOINT",
        "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
        // Headers (sometimes the only thing set, e.g. for auth tokens)
        "OTEL_EXPORTER_OTLP_HEADERS",
        "OTEL_EXPORTER_OTLP_TRACES_HEADERS",
        "OTEL_EXPORTER_OTLP_LOGS_HEADERS",
        "OTEL_EXPORTER_OTLP_METRICS_HEADERS",
    ];
    VARS.iter()
        .any(|var| std::env::var_os(var).is_some_and(|v| !v.is_empty()))
}

/// Build a shared OTel [`Resource`] with service, OS, process, and SDK
/// attributes.  The same resource is attached to traces, logs, and metrics
/// so all signals can be correlated.
fn build_resource() -> Resource {
    let mut attrs = vec![
        KeyValue::new(
            "service.name",
            std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "assistant".to_string()),
        ),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("os.type", std::env::consts::OS),
        KeyValue::new("host.arch", std::env::consts::ARCH),
        KeyValue::new("process.pid", std::process::id() as i64),
        KeyValue::new("process.runtime.name", "rust"),
        KeyValue::new("telemetry.sdk.name", "opentelemetry"),
        KeyValue::new("telemetry.sdk.language", "rust"),
    ];

    // Parse OTEL_RESOURCE_ATTRIBUTES (key1=val1,key2=val2,…) per the spec.
    if let Ok(env_attrs) = std::env::var("OTEL_RESOURCE_ATTRIBUTES") {
        for pair in env_attrs.split(',') {
            if let Some((key, val)) = pair.trim().split_once('=') {
                attrs.push(KeyValue::new(key.to_string(), val.to_string()));
            }
        }
    }

    Resource::builder_empty().with_attributes(attrs).build()
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
