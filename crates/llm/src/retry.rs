//! Retry configuration and helpers for LLM provider HTTP calls.
//!
//! Provides [`RetryConfig`] for configuring exponential backoff with jitter,
//! and a generic [`with_retry`] helper for retrying async operations that may
//! encounter transient errors (429, 503, etc.).

use std::time::Duration;

use tracing::warn;

// ── RetryConfig ──────────────────────────────────────────────────────────────

/// Configuration for retry behaviour with exponential backoff.
///
/// Used by both the `reqwest-retry` HTTP middleware (for raw-HTTP providers)
/// and the manual [`with_retry`] loop (for SDK-based providers like OpenAI).
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 = no retries, 3 = up to 3 retries).
    pub max_retries: u32,
    /// Initial delay before the first retry (milliseconds).
    pub initial_delay_ms: u64,
    /// Multiplier applied to the delay after each retry.
    pub multiplier: f64,
    /// Maximum delay between retries (milliseconds).
    pub max_delay_ms: u64,
    /// Whether to add random jitter to the delay.
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 20,
            initial_delay_ms: 1_000,
            multiplier: 2.0,
            max_delay_ms: 60_000,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a config that disables retries entirely.
    pub fn disabled() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }
}

// ── Transient error classification ───────────────────────────────────────────

/// HTTP status codes that indicate a transient (retryable) error.
///
/// - 408: Request Timeout
/// - 429: Too Many Requests (rate-limited)
/// - 500: Internal Server Error
/// - 502: Bad Gateway
/// - 503: Service Unavailable
/// - 504: Gateway Timeout
/// - 529: Overloaded (Anthropic-specific)
pub fn is_transient_status(status: u16) -> bool {
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504 | 529)
}

/// Check whether an error message from an SDK (like `async-openai`) indicates
/// a transient failure that should be retried.
///
/// This is a heuristic fallback for SDKs that don't expose structured HTTP
/// status codes.  It scans the error string for known transient status codes
/// and keywords.
pub fn is_transient_error_message(msg: &str) -> bool {
    // Check for numeric status codes embedded in the error message.
    for code in ["408", "429", "500", "502", "503", "504", "529"] {
        if msg.contains(code) {
            return true;
        }
    }

    // Check for common transient-error keywords.
    let lower = msg.to_lowercase();
    lower.contains("rate limit")
        || lower.contains("too many requests")
        || lower.contains("overloaded")
        || lower.contains("temporarily unavailable")
        || lower.contains("service unavailable")
        || lower.contains("try again")
        || lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("connection reset")
        || lower.contains("connection refused")
}

// ── Generic async retry helper ───────────────────────────────────────────────

/// Retry an async operation with exponential backoff.
///
/// Calls `operation` up to `config.max_retries + 1` times total.  On each
/// failure, `classify` is called to determine whether the error is transient.
/// If transient, the helper sleeps with exponential backoff (plus optional
/// jitter) before retrying.  Permanent errors are returned immediately.
///
/// # Parameters
///
/// * `config` – retry timing configuration.
/// * `provider_name` – used in log messages (e.g. `"OpenAI"`, `"Moonshot"`).
/// * `classify` – returns `true` if the error is transient and should be retried.
/// * `operation` – the async closure to retry.
pub async fn with_retry<F, Fut, T, E, C>(
    config: &RetryConfig,
    provider_name: &str,
    classify: C,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
    C: Fn(&E) -> bool,
{
    let mut attempt = 0u32;
    let mut delay_ms = config.initial_delay_ms as f64;

    loop {
        match operation().await {
            Ok(val) => return Ok(val),
            Err(err) => {
                attempt += 1;
                if attempt > config.max_retries || !classify(&err) {
                    if attempt > config.max_retries && config.max_retries > 0 {
                        warn!(
                            provider = provider_name,
                            attempts = attempt,
                            "All {} retry attempts exhausted: {err}",
                            config.max_retries
                        );
                    }
                    return Err(err);
                }

                // Compute delay with optional jitter.
                let actual_delay = if config.jitter {
                    // Simple jitter: random value between 50% and 100% of delay.
                    let jitter_factor = 0.5
                        + 0.5
                            * (std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .subsec_nanos() as f64
                                / u32::MAX as f64);
                    delay_ms * jitter_factor
                } else {
                    delay_ms
                };

                let capped_delay = actual_delay.min(config.max_delay_ms as f64);

                warn!(
                    provider = provider_name,
                    attempt,
                    max_retries = config.max_retries,
                    delay_ms = capped_delay as u64,
                    "Transient error, retrying: {err}"
                );

                tokio::time::sleep(Duration::from_millis(capped_delay as u64)).await;

                // Increase delay for next attempt.
                delay_ms *= config.multiplier;
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 20);
        assert_eq!(cfg.initial_delay_ms, 1_000);
        assert!((cfg.multiplier - 2.0).abs() < f64::EPSILON);
        assert_eq!(cfg.max_delay_ms, 60_000);
        assert!(cfg.jitter);
    }

    #[test]
    fn disabled_config_has_zero_retries() {
        let cfg = RetryConfig::disabled();
        assert_eq!(cfg.max_retries, 0);
    }

    #[test]
    fn transient_status_codes() {
        assert!(is_transient_status(429));
        assert!(is_transient_status(500));
        assert!(is_transient_status(502));
        assert!(is_transient_status(503));
        assert!(is_transient_status(504));
        assert!(is_transient_status(529));
        assert!(is_transient_status(408));

        assert!(!is_transient_status(200));
        assert!(!is_transient_status(400));
        assert!(!is_transient_status(401));
        assert!(!is_transient_status(403));
        assert!(!is_transient_status(404));
    }

    #[test]
    fn transient_error_messages() {
        assert!(is_transient_error_message("429 Too Many Requests"));
        assert!(is_transient_error_message("rate limit exceeded"));
        assert!(is_transient_error_message(
            "The engine is currently overloaded, please try again later"
        ));
        assert!(is_transient_error_message("503 Service Unavailable"));
        assert!(is_transient_error_message("connection timed out"));
        assert!(is_transient_error_message("connection refused"));

        assert!(!is_transient_error_message("Invalid API key"));
        assert!(!is_transient_error_message("400 Bad Request"));
        assert!(!is_transient_error_message("permission denied"));
    }

    #[tokio::test]
    async fn with_retry_succeeds_immediately() {
        let cfg = RetryConfig {
            max_retries: 3,
            jitter: false,
            ..Default::default()
        };
        let result: Result<&str, String> =
            with_retry(&cfg, "test", |_| true, || async { Ok("ok") }).await;
        assert_eq!(result.unwrap(), "ok");
    }

    #[tokio::test]
    async fn with_retry_retries_then_succeeds() {
        let cfg = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            jitter: false,
            ..Default::default()
        };
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();

        let result: Result<&str, String> = with_retry(
            &cfg,
            "test",
            |_| true,
            || {
                let c = c.clone();
                async move {
                    let n = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if n < 2 {
                        Err("transient".to_string())
                    } else {
                        Ok("ok")
                    }
                }
            },
        )
        .await;
        assert_eq!(result.unwrap(), "ok");
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn with_retry_permanent_error_not_retried() {
        let cfg = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            jitter: false,
            ..Default::default()
        };
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();

        let result: Result<&str, String> = with_retry(
            &cfg,
            "test",
            |_| false, // classify all as permanent
            || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err("permanent".to_string())
                }
            },
        )
        .await;
        assert!(result.is_err());
        // Should have been called exactly once (no retries).
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn with_retry_exhausts_retries() {
        let cfg = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 1,
            jitter: false,
            ..Default::default()
        };
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();

        let result: Result<&str, String> = with_retry(
            &cfg,
            "test",
            |_| true, // all transient
            || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err("still failing".to_string())
                }
            },
        )
        .await;
        assert!(result.is_err());
        // 1 initial + 2 retries = 3 total attempts.
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn with_retry_disabled_no_retries() {
        let cfg = RetryConfig::disabled();
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();

        let result: Result<&str, String> = with_retry(
            &cfg,
            "test",
            |_| true,
            || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err("fail".to_string())
                }
            },
        )
        .await;
        assert!(result.is_err());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
