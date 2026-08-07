//! `OTEL_SDK_DISABLED=true` must win over a fully configured OTLP exporter.
//!
//! Separate test binary from `otlp_env_config.rs`: both install a global
//! tracer provider and mutate the process environment, so they cannot share
//! a process.

use std::time::Duration;

use assistant_core::types::observability::{ObservabilityConfig, OtelExporter};
use assistant_runtime::telemetry::init_tracing;
use opentelemetry::global;
use opentelemetry::trace::{Tracer, TracerProvider};
use sqlx::SqlitePool;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sdk_disabled_suppresses_otlp_export() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // SAFETY: single-threaded setup phase of a dedicated test binary; no other
    // thread reads the environment until `init_tracing` runs below.
    unsafe {
        // A complete, valid OTLP configuration...
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", server.uri());
        std::env::set_var("OTEL_BSP_SCHEDULE_DELAY", "100");
        // ...that the kill switch must override.
        std::env::set_var("OTEL_SDK_DISABLED", "true");
    }

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite pool");

    // `exporter = "sqlite"` too, so this also pins that OTEL_SDK_DISABLED
    // takes precedence over the config file, not just over the env vars.
    let observability = ObservabilityConfig {
        exporter: OtelExporter::Sqlite,
        trace_content: false,
    };

    let guard = init_tracing(pool, &observability)
        .await
        .expect("init_tracing must succeed when the SDK is disabled");

    assert!(
        guard.is_none(),
        "no OtelGuard should be produced when OTEL_SDK_DISABLED=true — \
         no providers were installed, so there is nothing to shut down"
    );

    // Emitting through the (no-op) global provider must not reach the wire.
    let tracer = global::tracer_provider().tracer("sdk-disabled-test");
    tracer.in_span("must-not-be-exported", |_cx| {});

    // Give any (incorrectly installed) batch processor ample time to flush.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let received = server.received_requests().await.unwrap_or_default();
    assert!(
        received.is_empty(),
        "OTEL_SDK_DISABLED=true must suppress all export, but the collector \
         received {} request(s)",
        received.len()
    );
}
