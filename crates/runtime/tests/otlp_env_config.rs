//! End-to-end proof that OTLP export is driven entirely by the standard
//! `OTEL_*` environment variables.
//!
//! This is an integration test (its own process) for two reasons: `init_tracing`
//! installs a *global* tracing subscriber and tracer provider, and the test has
//! to mutate the process environment before that happens. Both make it
//! unsafe to run alongside other tests in the same binary.
//!
//! What it pins down, against a real HTTP server:
//!
//! - `OTEL_EXPORTER_OTLP_ENDPOINT` is honoured, and the generic form gets the
//!   spec-mandated `/v1/traces` path appended.
//! - `OTEL_EXPORTER_OTLP_HEADERS` reaches the wire, comma-separated,
//!   including auth-style headers.
//! - `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf` produces a
//!   `content-type: application/x-protobuf` request.
//! - `OTEL_EXPORTER_OTLP_COMPRESSION=gzip` is actually supported (the feature
//!   is compiled in) and marks the request `content-encoding: gzip`.
//! - Spans reach the collector at all — i.e. the exporter's HTTP client is
//!   compatible with the batch processor driving it. This is the part that
//!   silently breaks: the SDK's default `BatchSpanProcessor` runs exports on a
//!   plain background thread with `futures_executor::block_on`, so an async
//!   HTTP client (`reqwest-client`) has no reactor and never completes.

use std::time::Duration;

use assistant_core::types::observability::{ObservabilityConfig, OtelExporter};
use assistant_runtime::telemetry::init_tracing;
use opentelemetry::global;
use opentelemetry::trace::{Tracer, TracerProvider};
use sqlx::SqlitePool;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Header names the collector must receive, parsed out of a single
/// `OTEL_EXPORTER_OTLP_HEADERS` value.
const API_KEY_HEADER: &str = "x-api-key";
const TENANT_HEADER: &str = "x-tenant";

/// Path the generic `OTEL_EXPORTER_OTLP_ENDPOINT` must be extended with for
/// the trace signal, per the OTLP spec.
const TRACES_PATH: &str = "/v1/traces";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn otlp_export_is_configured_entirely_from_env_vars() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(TRACES_PATH))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // SAFETY: single-threaded setup phase of a dedicated test binary; no other
    // thread reads the environment until `init_tracing` runs below.
    unsafe {
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", server.uri());
        std::env::set_var(
            "OTEL_EXPORTER_OTLP_HEADERS",
            format!("{API_KEY_HEADER}=secret-token,{TENANT_HEADER}=prod"),
        );
        std::env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        std::env::set_var("OTEL_EXPORTER_OTLP_COMPRESSION", "gzip");
        std::env::set_var("OTEL_SERVICE_NAME", "assistant-under-test");
        std::env::set_var(
            "OTEL_RESOURCE_ATTRIBUTES",
            "deployment.environment=staging,service.namespace=acme",
        );
        // Export promptly instead of waiting out the 5s default batch delay.
        std::env::set_var("OTEL_BSP_SCHEDULE_DELAY", "100");
    }

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite pool");

    // `exporter = "none"` isolates the OTLP path: anything the mock receives
    // got there through the env-var-configured OTLP exporter.
    let observability = ObservabilityConfig {
        exporter: OtelExporter::None,
        trace_content: false,
    };

    let guard = init_tracing(pool, &observability)
        .await
        .expect("init_tracing must succeed with OTLP env vars set")
        .expect("an OtelGuard must be returned when OTLP is configured");

    // Emit a span through the globally installed provider.
    let tracer = global::tracer_provider().tracer("otlp-env-config-test");
    tracer.in_span("exported-span", |_cx| {});

    // Dropping the guard shuts the providers down, which flushes the batch.
    // Do it off the async worker: shutdown blocks on the exporter thread.
    tokio::task::spawn_blocking(move || drop(guard))
        .await
        .expect("shutdown must not panic");

    // Batch export happens on a background thread; give it a moment to land.
    // Filter by path: logs and metrics go to the same server on sibling
    // endpoints, and `received_requests` returns every request, matched or not.
    let requests = wait_for_requests(&server, TRACES_PATH, Duration::from_secs(10)).await;

    assert!(
        !requests.is_empty(),
        "no span export reached the collector — the OTLP HTTP client is likely \
         incompatible with the batch processor driving it (async client on a \
         non-tokio export thread)"
    );

    let request = &requests[0];

    let header = |name: &str| -> String {
        request
            .headers
            .get(name)
            .unwrap_or_else(|| panic!("request must carry a `{name}` header"))
            .to_str()
            .unwrap_or_default()
            .to_string()
    };

    assert_eq!(
        header(API_KEY_HEADER),
        "secret-token",
        "OTEL_EXPORTER_OTLP_HEADERS must put `{API_KEY_HEADER}` on the wire"
    );
    assert_eq!(
        header(TENANT_HEADER),
        "prod",
        "comma-separated headers must all be applied, not just the first"
    );
    assert_eq!(
        header("content-type"),
        "application/x-protobuf",
        "http/protobuf must produce a protobuf content type"
    );
    assert_eq!(
        header("content-encoding"),
        "gzip",
        "OTEL_EXPORTER_OTLP_COMPRESSION=gzip must compress the payload"
    );
    assert!(
        !request.body.is_empty(),
        "the exported request must carry a payload"
    );

    // Protobuf encodes string fields verbatim, so a substring search over the
    // decompressed payload is enough to prove the resource attributes made it
    // onto the wire — no protobuf decoder needed.
    let payload = gunzip(&request.body);

    for expected in [
        "assistant-under-test", // OTEL_SERVICE_NAME
        "staging",              // OTEL_RESOURCE_ATTRIBUTES
        "acme",                 // OTEL_RESOURCE_ATTRIBUTES
        "exported-span",        // the span itself
    ] {
        assert!(
            contains(&payload, expected.as_bytes()),
            "exported payload must contain {expected:?}"
        );
    }
}

/// Decompress a gzip payload, failing the test with a clear message if the
/// body is not actually gzipped.
fn gunzip(body: &[u8]) -> Vec<u8> {
    use std::io::Read;

    let mut decoder = flate2::read::GzDecoder::new(body);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .expect("body advertised `content-encoding: gzip` and must decompress");
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Poll the mock server until it has recorded at least one request for
/// `path`, or the deadline passes. Returns the matching requests.
async fn wait_for_requests(server: &MockServer, path: &str, timeout: Duration) -> Vec<Request> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let matching: Vec<Request> = server
            .received_requests()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|request| request.url.path() == path)
            .collect();

        if !matching.is_empty() || std::time::Instant::now() >= deadline {
            return matching;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
