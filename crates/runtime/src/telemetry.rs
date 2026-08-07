use std::time::Duration;

use anyhow::Result;
use assistant_core::types::observability::{ObservabilityConfig, OtelExporter};
use opentelemetry::{Key, KeyValue, global};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_exporter_sqlite::{SqliteLogExporter, SqliteMetricExporter, SqliteSpanExporter};
use opentelemetry_sdk::{
    Resource,
    logs::{BatchLogProcessor, SdkLoggerProvider},
    metrics::{PeriodicReader, SdkMeterProvider},
    propagation::TraceContextPropagator,
    trace::{BatchSpanProcessor, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::attribute::{SERVICE_NAME, SERVICE_VERSION};
use sqlx::SqlitePool;
use tracing::info;
use tracing_subscriber::{
    EnvFilter, Layer, filter::Targets, layer::SubscriberExt, util::SubscriberInitExt,
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
/// Application targets pass through at INFO and above.
/// Noisy third-party crates (async_nats, h2, hyper_util) are suppressed below WARN.
pub(crate) fn otel_log_bridge_filter() -> Targets {
    Targets::new()
        .with_default(tracing::Level::INFO)
        .with_target("sqlx", tracing::Level::ERROR)
        .with_target("sqlx::query", tracing::metadata::LevelFilter::OFF)
        .with_target("sqlx_core", tracing::metadata::LevelFilter::OFF)
        .with_target("sqlx_sqlite", tracing::metadata::LevelFilter::OFF)
        .with_target("async_nats", tracing::Level::WARN)
        .with_target("h2", tracing::metadata::LevelFilter::OFF)
        .with_target("hyper_util", tracing::metadata::LevelFilter::OFF)
}

/// Install tracing subscribers and OpenTelemetry exporters.
///
/// Two independent destinations exist, and they can run side by side — each
/// OTel provider simply gets multiple processors/readers:
///
/// 1. **Local SQLite** (`[observability] exporter = "sqlite"`, the default).
///    A small on-disk store that powers the built-in trace/log/metric viewers.
///    Set `exporter = "none"` to turn it off.
/// 2. **OTLP**, enabled by setting *any* non-empty `OTEL_EXPORTER_OTLP_*`
///    environment variable. This is the supported path to a real
///    observability stack (Grafana/Tempo/Loki/Mimir, Honeycomb, SignalDB, …).
///
/// The `opentelemetry-otlp` crate reads the standard env vars internally, so
/// all of the following are supported without additional code:
///
/// | Variable | Per-signal overrides |
/// |----------|---------------------|
/// | `OTEL_EXPORTER_OTLP_ENDPOINT` | `_TRACES_ENDPOINT`, `_LOGS_ENDPOINT`, `_METRICS_ENDPOINT` |
/// | `OTEL_EXPORTER_OTLP_PROTOCOL` | `_TRACES_PROTOCOL`, `_LOGS_PROTOCOL`, `_METRICS_PROTOCOL` |
/// | `OTEL_EXPORTER_OTLP_HEADERS` | `_TRACES_HEADERS`, `_LOGS_HEADERS`, `_METRICS_HEADERS` |
/// | `OTEL_EXPORTER_OTLP_TIMEOUT` | `_TRACES_TIMEOUT`, `_LOGS_TIMEOUT`, `_METRICS_TIMEOUT` |
/// | `OTEL_EXPORTER_OTLP_COMPRESSION` | `_TRACES_COMPRESSION`, `_LOGS_COMPRESSION`, `_METRICS_COMPRESSION` |
///
/// Only `http/protobuf` — the OTel default protocol — is compiled in, so
/// endpoints are the `:4318`-style HTTP ones. gRPC (`:4317`) would pull the
/// whole tonic stack in and is deliberately not built; `http/json` is left
/// out because enabling it would silently become the default protocol.
///
/// Additional SDK-level variables honoured here or by the SDK itself:
/// `OTEL_SDK_DISABLED`, `OTEL_SERVICE_NAME`, `OTEL_RESOURCE_ATTRIBUTES`,
/// `OTEL_METRIC_EXPORT_INTERVAL`.
///
/// The OTel log bridge uses a dedicated per-layer filter (see
/// [`otel_log_bridge_filter`]) that suppresses all `sqlx` targets. Without
/// this, the log exporter's own INSERT queries would emit tracing events that
/// get captured by the bridge, creating a feedback loop.
pub async fn init_tracing(
    pool: SqlitePool,
    observability: &ObservabilityConfig,
) -> Result<Option<OtelGuard>> {
    let fmt_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);

    // -- Shared resource (used by traces, logs, and metrics) --
    let resource = build_resource();

    // `OTEL_SDK_DISABLED=true` is the spec-defined kill switch: no providers,
    // no exporters, console logging only.
    let disabled = sdk_disabled(std::env::var(OTEL_SDK_DISABLED).ok().as_deref());

    let enable_sqlite = !disabled && matches!(observability.exporter, OtelExporter::Sqlite);

    // Detect whether the user wants OTLP export by checking for any of the
    // standard `OTEL_EXPORTER_OTLP_*` env vars.  We intentionally do NOT
    // read the values ourselves — the crate resolves per-signal overrides,
    // protocol, timeouts, headers, and compression internally.
    let enable_otlp = !disabled && otlp_configured(std::env::vars());
    let need_otel = enable_sqlite || enable_otlp;

    if disabled {
        info!("{OTEL_SDK_DISABLED} is set — OpenTelemetry export is disabled");
    }

    if enable_otlp {
        info!(
            "OTLP export enabled — the opentelemetry-otlp crate will read endpoint, protocol, headers, timeout, and compression from OTEL_EXPORTER_OTLP_* env vars"
        );
    }

    if need_otel {
        global::set_text_map_propagator(TraceContextPropagator::new());
    }

    // -- Trace provider --------------------------------------------------
    let mut trace_provider_builder = SdkTracerProvider::builder().with_resource(resource.clone());
    let mut have_trace_exporter = false;

    if enable_sqlite {
        let sqlite_exporter = SqliteSpanExporter::new(pool.clone());
        let processor = BatchSpanProcessor::builder(sqlite_exporter).build();
        trace_provider_builder = trace_provider_builder.with_span_processor(processor);
        have_trace_exporter = true;
    }

    if enable_otlp {
        // `build()` without an explicit transport lets the crate resolve the
        // protocol from OTEL_EXPORTER_OTLP_TRACES_PROTOCOL (or the generic
        // fallback), along with endpoint, headers, timeout, and compression.
        let otlp_exporter = opentelemetry_otlp::SpanExporter::builder().build()?;
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

        if enable_sqlite {
            let sqlite_log_exporter = SqliteLogExporter::new(pool.clone());
            let processor = BatchLogProcessor::builder(sqlite_log_exporter).build();
            log_builder = log_builder.with_log_processor(processor);
        }

        if enable_otlp {
            let otlp_log_exporter = opentelemetry_otlp::LogExporter::builder().build()?;
            log_builder = log_builder.with_batch_exporter(otlp_log_exporter);
        }

        // Metrics — attach SQLite and/or OTLP readers to the same provider.
        let mut meter_builder = SdkMeterProvider::builder().with_resource(resource);

        if enable_sqlite {
            let sqlite_metric_exporter = SqliteMetricExporter::new(pool);
            let reader = PeriodicReader::builder(sqlite_metric_exporter)
                .with_interval(Duration::from_secs(60))
                .build();
            meter_builder = meter_builder.with_reader(reader);
        }

        if enable_otlp {
            let otlp_metric_exporter = opentelemetry_otlp::MetricExporter::builder().build()?;
            // No explicit interval — `PeriodicReader` picks up
            // `OTEL_METRIC_EXPORT_INTERVAL` (milliseconds) and falls back to
            // 60s. Setting one here would override the env var.
            let reader = PeriodicReader::builder(otlp_metric_exporter).build();
            meter_builder = meter_builder.with_reader(reader);
        }

        let log_prov = log_builder.build();

        // Bridge tracing events → OTel log records with the anti-stampede filter.
        let otel_filter = otel_log_bridge_filter();
        let otel_log_layer = OpenTelemetryTracingBridge::new(&log_prov).with_filter(otel_filter);
        logger_provider = Some(log_prov);

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

/// Prefix shared by every standard OTLP exporter environment variable.
const OTLP_ENV_PREFIX: &str = "OTEL_EXPORTER_OTLP_";

/// Spec-defined kill switch that disables the OpenTelemetry SDK entirely.
const OTEL_SDK_DISABLED: &str = "OTEL_SDK_DISABLED";

/// Returns `true` when any non-empty `OTEL_EXPORTER_OTLP_*` variable is
/// present, indicating the user wants remote OTLP export.
///
/// Takes the environment as an iterator so the decision can be tested without
/// mutating the process environment.
fn otlp_configured(vars: impl Iterator<Item = (String, String)>) -> bool {
    vars.into_iter()
        .any(|(key, value)| key.starts_with(OTLP_ENV_PREFIX) && !value.trim().is_empty())
}

/// Returns `true` when `OTEL_SDK_DISABLED` requests that telemetry be off.
///
/// Per the OpenTelemetry specification only the literal `true` (case
/// insensitive) disables the SDK; every other value — including unrecognised
/// ones — leaves it enabled.
fn sdk_disabled(value: Option<&str>) -> bool {
    value.is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

/// Service name used when nothing in the environment supplies one.
const DEFAULT_SERVICE_NAME: &str = "assistant";

/// Prefix of the SDK's spec-mandated placeholder when no service name is
/// configured (`unknown_service` or `unknown_service:<executable>`).
const SDK_UNKNOWN_SERVICE_PREFIX: &str = "unknown_service";

/// Decide whether to replace the service name the SDK detectors resolved.
///
/// The SDK already implements the specified precedence — `OTEL_SERVICE_NAME`,
/// then `service.name` inside `OTEL_RESOURCE_ATTRIBUTES`, then an
/// `unknown_service` placeholder. Overriding unconditionally (as this code
/// previously did) would discard a `service.name` set via
/// `OTEL_RESOURCE_ATTRIBUTES`, so only the placeholder is replaced.
///
/// Returns `Some(name)` to force a name, or `None` to keep what was detected.
fn service_name_override(detected: Option<&str>) -> Option<&'static str> {
    match detected {
        Some(name) if !name.starts_with(SDK_UNKNOWN_SERVICE_PREFIX) => None,
        _ => Some(DEFAULT_SERVICE_NAME),
    }
}

/// Build a shared OTel [`Resource`] with service, OS, process, and SDK
/// attributes.  The same resource is attached to traces, logs, and metrics
/// so all signals can be correlated.
///
/// `Resource::builder()` runs the SDK's default detectors, which pick up
/// `OTEL_SERVICE_NAME` and `OTEL_RESOURCE_ATTRIBUTES` on their own.
fn build_resource() -> Resource {
    let detected_name = Resource::builder()
        .build()
        .get(&Key::from_static_str(SERVICE_NAME))
        .map(|value| value.to_string());

    let mut attrs = vec![KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))];
    if let Some(name) = service_name_override(detected_name.as_deref()) {
        attrs.push(KeyValue::new(SERVICE_NAME, name));
    }

    Resource::builder().with_attributes(attrs).build()
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

    /// Application targets must pass through at INFO and above.
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
                !filter.would_enable(target, &tracing::Level::DEBUG),
                "{target} DEBUG must be blocked"
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

    /// TRACE and DEBUG events from application targets are not forwarded (the
    /// default is INFO).
    #[test]
    fn filter_blocks_trace_level_for_app() {
        let filter = otel_log_bridge_filter();

        assert!(
            !filter.would_enable("assistant_runtime", &tracing::Level::TRACE),
            "TRACE should not pass (default is INFO)"
        );
    }

    /// async_nats must be silenced below WARN — it's the primary log volume offender.
    #[test]
    fn filter_blocks_async_nats_below_warn() {
        let filter = otel_log_bridge_filter();

        assert!(!filter.would_enable("async_nats", &tracing::Level::TRACE));
        assert!(!filter.would_enable("async_nats", &tracing::Level::DEBUG));
        assert!(!filter.would_enable("async_nats", &tracing::Level::INFO));
        assert!(filter.would_enable("async_nats", &tracing::Level::WARN));
        assert!(filter.would_enable("async_nats", &tracing::Level::ERROR));
    }

    /// h2 and hyper_util must be fully silenced.
    #[test]
    fn filter_blocks_http_internals() {
        let filter = otel_log_bridge_filter();

        for target in &["h2", "hyper_util"] {
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

    // -- OTLP env-var gating --------------------------------------------

    fn vars(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    /// Any non-empty `OTEL_EXPORTER_OTLP_*` variable turns OTLP export on.
    #[test]
    fn otlp_enabled_by_any_exporter_var() {
        for key in [
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            "OTEL_EXPORTER_OTLP_LOGS_ENDPOINT",
            "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
            "OTEL_EXPORTER_OTLP_HEADERS",
            "OTEL_EXPORTER_OTLP_PROTOCOL",
        ] {
            assert!(
                otlp_configured(vars(&[(key, "x")]).into_iter()),
                "{key} must enable OTLP export"
            );
        }
    }

    /// Empty values and unrelated OTEL variables must not enable OTLP.
    #[test]
    fn otlp_not_enabled_by_unrelated_or_empty_vars() {
        assert!(
            !otlp_configured(vars(&[("OTEL_EXPORTER_OTLP_ENDPOINT", "")]).into_iter()),
            "an empty endpoint must not enable OTLP"
        );
        assert!(
            !otlp_configured(
                vars(&[
                    ("OTEL_SERVICE_NAME", "assistant"),
                    ("OTEL_RESOURCE_ATTRIBUTES", "deployment.environment=dev"),
                    ("OTEL_METRIC_EXPORT_INTERVAL", "5000"),
                ])
                .into_iter()
            ),
            "resource/SDK variables alone must not enable OTLP"
        );
        assert!(
            !otlp_configured(std::iter::empty()),
            "no variables means no OTLP"
        );
    }

    // -- OTEL_SDK_DISABLED ----------------------------------------------

    /// The spec-defined `OTEL_SDK_DISABLED=true` kill switch must be honoured,
    /// case-insensitively and tolerant of surrounding whitespace.
    #[test]
    fn sdk_disabled_recognises_truthy_values() {
        for value in ["true", "TRUE", "True", " true "] {
            assert!(
                sdk_disabled(Some(value)),
                "OTEL_SDK_DISABLED={value:?} must disable the SDK"
            );
        }
    }

    /// Anything other than `true` leaves the SDK enabled — per the OTel spec,
    /// unrecognised values fall back to the default (`false`).
    #[test]
    fn sdk_enabled_for_other_values() {
        for value in ["false", "", "0", "1", "yes", "no"] {
            assert!(
                !sdk_disabled(Some(value)),
                "OTEL_SDK_DISABLED={value:?} must leave the SDK enabled"
            );
        }
        assert!(!sdk_disabled(None), "unset must leave the SDK enabled");
    }

    // -- service.name precedence ----------------------------------------

    /// When the SDK detectors resolved a real service name — from
    /// `OTEL_SERVICE_NAME` or from `service.name` inside
    /// `OTEL_RESOURCE_ATTRIBUTES` — we must leave it alone. Forcing our own
    /// default here would silently discard the operator's configuration.
    #[test]
    fn detected_service_name_is_not_overridden() {
        for detected in ["my-assistant", "assistant-prod", "unknown-but-explicit"] {
            assert_eq!(
                service_name_override(Some(detected)),
                None,
                "a configured service.name ({detected:?}) must survive untouched"
            );
        }
    }

    /// When nothing configured a service name the SDK falls back to
    /// `unknown_service` / `unknown_service:<exe>`. That is when — and only
    /// when — we substitute our own default.
    #[test]
    fn sdk_fallback_service_name_is_replaced_with_default() {
        for detected in [
            None,
            Some("unknown_service"),
            Some("unknown_service:assistant"),
            Some("unknown_service:some-test-binary"),
        ] {
            assert_eq!(
                service_name_override(detected),
                Some(DEFAULT_SERVICE_NAME),
                "the SDK fallback ({detected:?}) must become {DEFAULT_SERVICE_NAME:?}"
            );
        }
    }

    /// `build_resource` injects SERVICE_NAME and SERVICE_VERSION from env / Cargo.
    #[test]
    fn build_resource_includes_service_attributes() {
        let resource = build_resource();
        // Resource exposes attributes via `iter`.
        let names: Vec<String> = resource
            .iter()
            .map(|(k, _)| k.as_str().to_string())
            .collect();
        assert!(
            names
                .iter()
                .any(|n| n == opentelemetry_semantic_conventions::attribute::SERVICE_NAME),
            "service.name should be present: {names:?}"
        );
        assert!(
            names
                .iter()
                .any(|n| n == opentelemetry_semantic_conventions::attribute::SERVICE_VERSION),
            "service.version should be present: {names:?}"
        );
    }
}
