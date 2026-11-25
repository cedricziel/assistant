//! Without an OTLP endpoint, telemetry is a no-op: no providers, no guard.
//!
//! Own test binary because `init_tracing` installs a global subscriber.

use assistant_runtime::telemetry::init_tracing;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_otlp_endpoint_means_no_telemetry_pipeline() {
    // SAFETY: single-threaded setup phase of a dedicated test binary; no other
    // thread reads the environment until `init_tracing` runs below.
    unsafe {
        let otel_vars: Vec<String> = std::env::vars()
            .map(|(key, _)| key)
            .filter(|key| key.starts_with("OTEL_"))
            .collect();
        for key in otel_vars {
            std::env::remove_var(key);
        }
    }

    let guard = init_tracing().expect("init_tracing must succeed without any OTEL_* configuration");

    assert!(
        guard.is_none(),
        "no OtelGuard should be produced when no OTLP endpoint is configured"
    );
}
