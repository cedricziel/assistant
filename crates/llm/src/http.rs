//! Shared HTTP client construction with `reqwest-tracing` instrumentation.
//!
//! Every provider that makes raw HTTP calls should use [`build_http_client`]
//! rather than constructing a bare `reqwest::Client`.  This ensures all
//! outgoing requests emit `tracing` spans with method, URL, status, and
//! duration — useful for debugging and observability.

use std::time::Duration;

use reqwest_middleware::ClientWithMiddleware;
use reqwest_tracing::TracingMiddleware;

/// Build an instrumented HTTP client with the given timeout.
///
/// The returned [`ClientWithMiddleware`] wraps a standard `reqwest::Client`
/// with [`TracingMiddleware`] so every request/response is recorded as a
/// `tracing` span.
pub fn build_http_client(timeout_secs: u64) -> anyhow::Result<ClientWithMiddleware> {
    let inner = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))?;

    Ok(reqwest_middleware::ClientBuilder::new(inner)
        .with(TracingMiddleware::default())
        .build())
}
