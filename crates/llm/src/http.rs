//! Shared HTTP client construction with `reqwest-tracing` and `reqwest-retry`
//! instrumentation.
//!
//! Every provider that makes raw HTTP calls should use [`build_http_client`]
//! rather than constructing a bare `reqwest::Client`.  This ensures all
//! outgoing requests emit `tracing` spans and are automatically retried on
//! transient failures (429, 503, etc.) with exponential backoff.

use std::time::Duration;

use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use reqwest_tracing::TracingMiddleware;

use crate::retry::RetryConfig;

/// Build an instrumented HTTP client with the given timeout and retry config.
///
/// The returned [`ClientWithMiddleware`] wraps a standard `reqwest::Client`
/// with:
/// - [`TracingMiddleware`] so every request/response is recorded as a `tracing`
///   span.
/// - [`RetryTransientMiddleware`] so transient failures (429, 500, 502, 503,
///   504) are automatically retried with exponential backoff.
pub fn build_http_client(
    timeout_secs: u64,
    retry_config: &RetryConfig,
) -> anyhow::Result<ClientWithMiddleware> {
    let inner = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))?;

    let mut builder =
        reqwest_middleware::ClientBuilder::new(inner).with(TracingMiddleware::default());

    if retry_config.max_retries > 0 {
        let retry_policy = ExponentialBackoff::builder()
            .retry_bounds(
                Duration::from_millis(retry_config.initial_delay_ms),
                Duration::from_millis(retry_config.max_delay_ms),
            )
            .build_with_max_retries(retry_config.max_retries);

        builder = builder.with(RetryTransientMiddleware::new_with_policy(retry_policy));
    }

    Ok(builder.build())
}
