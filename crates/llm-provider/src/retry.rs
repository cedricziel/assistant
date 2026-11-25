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
            max_retries: 5,
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
/// - 524: Origin timeout (Cloudflare, returned by OpenRouter)
/// - 529: Overloaded (Anthropic-specific)
pub fn is_transient_status(status: u16) -> bool {
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504 | 524 | 529)
}

/// Markers that identify *permanent* upstream failures even when the message
/// also looks transient (e.g. an HTTP 429 from a billing-suspended account).
///
/// Match is case-insensitive substring against the error message. The list is
/// intentionally narrow — entries should originate from real provider error
/// bodies seen in production, not speculation. Adding too many markers risks
/// classifying a genuinely transient hiccup as permanent.
const PERMANENT_BILLING_MARKERS: &[&str] = &[
    "insufficient balance",   // Moonshot
    "insufficient quota",     // OpenAI (with space)
    "insufficient_quota",     // OpenAI (snake-case)
    "account is suspended",   // Moonshot variant
    "account suspended",      // generic
    "quota exceeded",         // OpenAI / generic
    "please recharge",        // Moonshot
    "payment required",       // HTTP 402 in spirit
    "billing required",       // OpenAI billing prompts
    "billing hard limit",     // OpenAI hard cap
    "billing limit exceeded", // generic phrasing
];

/// Returns `true` when the (already lowercased) error message contains any
/// marker indicating a permanent billing/quota/suspension failure that must
/// not be retried.
fn contains_permanent_billing_marker(lower: &str) -> bool {
    PERMANENT_BILLING_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Returns `true` when `msg` indicates a permanent billing/quota/suspension
/// failure from the upstream provider. Used to short-circuit retry of errors
/// whose HTTP code (e.g. `429`) would otherwise be classified as transient
/// but whose body identifies a permanent state (account suspended, quota
/// exhausted, payment required, etc.).
pub fn is_permanent_billing_error(msg: &str) -> bool {
    contains_permanent_billing_marker(&msg.to_lowercase())
}

/// Check whether an error message from an SDK (like `async-openai`) indicates
/// a transient failure that should be retried.
///
/// This is a heuristic fallback for SDKs that don't expose structured HTTP
/// status codes.  It scans the error string for known transient status codes
/// and keywords. Permanent billing/quota errors short-circuit to `false` even
/// when the message also contains a transient-looking status code such as
/// `429` — see [`is_permanent_billing_error`].
pub fn is_transient_error_message(msg: &str) -> bool {
    let lower = msg.to_lowercase();

    // Permanent billing errors must never be retried, regardless of the
    // accompanying HTTP status code.
    if contains_permanent_billing_marker(&lower) {
        return false;
    }

    if contains_transient_status_code(msg) {
        return true;
    }

    // Check for common transient-error keywords.
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

/// Returns `true` when `msg` contains a transient HTTP status code as a
/// standalone token. Digits inside identifiers such as UUIDs (`529ce9e8-…`)
/// or numbers (`1500ms`) do not count.
fn contains_transient_status_code(msg: &str) -> bool {
    msg.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| token.len() == 3)
        .filter_map(|token| token.parse::<u16>().ok())
        .any(is_transient_status)
}

// ── Typed provider errors ────────────────────────────────────────────────────

/// A non-2xx response from a provider, kept typed so retries can honor the
/// status and `Retry-After` instead of guessing from the message text.
#[derive(Debug, Clone)]
pub struct ProviderHttpError {
    pub status: u16,
    pub message: String,
    /// Provider-specific error class, e.g. OpenRouter's `metadata.error_type`.
    pub error_type: Option<String>,
    pub retry_after: Option<Duration>,
}

impl std::fmt::Display for ProviderHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.error_type {
            Some(kind) => write!(f, "API error ({}, {kind}): {}", self.status, self.message),
            None => write!(f, "API error ({}): {}", self.status, self.message),
        }
    }
}

impl std::error::Error for ProviderHttpError {}

/// Parse a `Retry-After` header value. Only the delay-seconds form is
/// supported; the HTTP-date form would need the wall clock, so it yields
/// `None` and the caller's backoff applies.
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    value.trim().parse::<u64>().ok().map(Duration::from_secs)
}

/// Whether a failed attempt may be retried, and the earliest time to do so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryVerdict {
    Permanent,
    Transient { after: Option<Duration> },
}

/// Classify an error for [`with_retry_hinted`]. A [`ProviderHttpError`] is
/// judged by its status and carries its `Retry-After`; any other error falls
/// back to [`is_transient_error_message`]. Billing failures are permanent
/// either way.
pub fn classify_provider_error(err: &anyhow::Error) -> RetryVerdict {
    if let Some(http) = err.downcast_ref::<ProviderHttpError>() {
        if is_transient_status(http.status) && !is_permanent_billing_error(&http.message) {
            return RetryVerdict::Transient {
                after: http.retry_after,
            };
        }
        return RetryVerdict::Permanent;
    }
    if is_transient_error_message(&err.to_string()) {
        RetryVerdict::Transient { after: None }
    } else {
        RetryVerdict::Permanent
    }
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
    operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
    C: Fn(&E) -> bool,
{
    with_retry_hinted(
        config,
        provider_name,
        |err| {
            if classify(err) {
                RetryVerdict::Transient { after: None }
            } else {
                RetryVerdict::Permanent
            }
        },
        operation,
    )
    .await
}

/// Like [`with_retry`], but `classify` may also say how long the provider
/// asked us to wait. The sleep is the larger of the backoff and that hint. A
/// hint longer than `config.max_delay_ms` ends the retries: waiting that long
/// inside one call would stall the turn.
pub async fn with_retry_hinted<F, Fut, T, E, C>(
    config: &RetryConfig,
    provider_name: &str,
    classify: C,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
    C: Fn(&E) -> RetryVerdict,
{
    let mut attempt = 0u32;
    let mut delay_ms = config.initial_delay_ms as f64;
    let max_delay = Duration::from_millis(config.max_delay_ms);

    loop {
        let err = match operation().await {
            Ok(val) => return Ok(val),
            Err(err) => err,
        };
        attempt += 1;
        let after = match classify(&err) {
            RetryVerdict::Permanent => return Err(err),
            RetryVerdict::Transient { after } => after,
        };
        if attempt > config.max_retries {
            if config.max_retries > 0 {
                warn!(
                    provider = provider_name,
                    attempts = attempt,
                    "All {} retry attempts exhausted: {err}",
                    config.max_retries
                );
            }
            return Err(err);
        }
        if after.is_some_and(|after| after > max_delay) {
            warn!(
                provider = provider_name,
                retry_after_secs = after.map(|a| a.as_secs()),
                "Provider asked to wait longer than the retry cap; giving up: {err}"
            );
            return Err(err);
        }

        let backoff = Duration::from_millis(
            jittered(config, delay_ms).min(config.max_delay_ms as f64) as u64,
        );
        let wait = after.map_or(backoff, |after| after.max(backoff));

        warn!(
            provider = provider_name,
            attempt,
            max_retries = config.max_retries,
            delay_ms = wait.as_millis() as u64,
            "Transient error, retrying: {err}"
        );

        tokio::time::sleep(wait).await;
        delay_ms *= config.multiplier;
    }
}

/// `delay_ms`, scaled to between 50% and 100% when jitter is on.
fn jittered(config: &RetryConfig, delay_ms: f64) -> f64 {
    if !config.jitter {
        return delay_ms;
    }
    // getrandom rather than the system clock: the workspace clock lint bans
    // SystemTime::now() in production code.
    let mut buf = [0u8; 4];
    let unit = if getrandom::fill(&mut buf).is_ok() {
        u32::from_le_bytes(buf) as f64 / u32::MAX as f64
    } else {
        0.5
    };
    delay_ms * (0.5 + 0.5 * unit)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 5);
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

    #[test]
    fn status_code_digits_inside_identifiers_are_not_transient() {
        let msg = "conversation 529ce9e8-d204-40c8-8975-dc64b0aee0d8 belongs to agent \
                   'reviewer' (requested 'default')";
        assert!(
            !is_transient_error_message(msg),
            "a UUID containing '529' must not mark a permanent error as transient"
        );
        assert!(!is_transient_error_message("message 1a2b5003 not found"));
        assert!(!is_transient_error_message("took 1500ms"));
        assert!(!is_transient_error_message(
            "message 8d82c1b9-0529-40c8-8975-dc64b0aee0d8 not found"
        ));
    }

    #[test]
    fn status_codes_delimited_by_punctuation_remain_transient() {
        assert!(is_transient_error_message("HTTP status: 502"));
        assert!(is_transient_error_message("upstream returned (529)"));
        assert!(is_transient_error_message("status=504, body empty"));
    }

    #[test]
    fn moonshot_insufficient_balance_429_is_permanent() {
        let msg = "API error (429 Too Many Requests): Your account org-abc \
                   is suspended due to insufficient balance, please recharge";
        assert!(
            is_permanent_billing_error(msg),
            "Moonshot insufficient-balance must classify as permanent"
        );
        assert!(
            !is_transient_error_message(msg),
            "Moonshot insufficient-balance must not be retried even though it carries 429"
        );
    }

    #[test]
    fn openai_insufficient_quota_429_is_permanent() {
        let msg = "API error: 429 — insufficient_quota: You exceeded your current quota";
        assert!(is_permanent_billing_error(msg));
        assert!(!is_transient_error_message(msg));
    }

    #[test]
    fn real_transient_429_remains_transient() {
        let msg = "API error 429: Too Many Requests — please retry after 30s";
        assert!(!is_permanent_billing_error(msg));
        assert!(is_transient_error_message(msg));
    }

    #[test]
    fn real_transient_503_unaffected() {
        let msg = "503 Service Unavailable";
        assert!(!is_permanent_billing_error(msg));
        assert!(is_transient_error_message(msg));
    }

    #[test]
    fn permanent_marker_case_insensitive() {
        assert!(is_permanent_billing_error("ACCOUNT IS SUSPENDED"));
        assert!(is_permanent_billing_error("Insufficient_Quota"));
        assert!(is_permanent_billing_error("Quota Exceeded"));
        assert!(is_permanent_billing_error("Payment Required"));
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

    // -- Retry-After aware retries --

    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn no_jitter(initial_delay_ms: u64, max_delay_ms: u64) -> RetryConfig {
        RetryConfig {
            max_retries: 3,
            initial_delay_ms,
            max_delay_ms,
            jitter: false,
            ..Default::default()
        }
    }

    /// Fails once with `first_error`, then succeeds. Returns the result, the
    /// number of attempts and the virtual time that passed.
    async fn fail_once(
        cfg: &RetryConfig,
        first_error: anyhow::Error,
    ) -> (anyhow::Result<&'static str>, u32, Duration) {
        let attempts = Arc::new(AtomicU32::new(0));
        let first = std::sync::Mutex::new(Some(first_error));
        let start = tokio::time::Instant::now();
        let result = with_retry_hinted(cfg, "test", classify_provider_error, || {
            let attempts = attempts.clone();
            let err = first.lock().unwrap().take();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                match err {
                    Some(e) => Err(e),
                    None => Ok("ok"),
                }
            }
        })
        .await;
        (result, attempts.load(Ordering::SeqCst), start.elapsed())
    }

    fn http_error(status: u16, retry_after: Option<u64>) -> anyhow::Error {
        anyhow::Error::new(ProviderHttpError {
            status,
            message: "upstream says no".into(),
            error_type: None,
            retry_after: retry_after.map(Duration::from_secs),
        })
    }

    #[tokio::test(start_paused = true)]
    async fn retry_after_longer_than_backoff_is_honored() {
        let (result, attempts, elapsed) =
            fail_once(&no_jitter(100, 60_000), http_error(429, Some(2))).await;
        assert_eq!(result.unwrap(), "ok", "the second attempt succeeds");
        assert_eq!(attempts, 2, "one retry");
        assert!(
            elapsed >= Duration::from_secs(2),
            "waited at least Retry-After, not just the 100ms backoff: {elapsed:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn backoff_longer_than_retry_after_wins() {
        let (_, _, elapsed) = fail_once(&no_jitter(5_000, 60_000), http_error(503, Some(1))).await;
        assert!(
            elapsed >= Duration::from_secs(5),
            "the larger of backoff and Retry-After is used: {elapsed:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn retry_after_beyond_max_delay_gives_up() {
        let (result, attempts, elapsed) =
            fail_once(&no_jitter(100, 10_000), http_error(429, Some(120))).await;
        let err = result.unwrap_err();
        assert_eq!(attempts, 1, "no retry when Retry-After exceeds max_delay");
        assert_eq!(elapsed, Duration::ZERO, "no sleep at all");
        assert_eq!(
            err.downcast_ref::<ProviderHttpError>().map(|e| e.status),
            Some(429),
            "the original error is returned"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn permanent_statuses_are_not_retried() {
        for status in [400, 401, 402, 403, 404, 413] {
            let (result, attempts, _) =
                fail_once(&no_jitter(1, 60_000), http_error(status, None)).await;
            assert!(result.is_err(), "{status} fails");
            assert_eq!(attempts, 1, "{status} must not be retried");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn transient_statuses_are_retried() {
        for status in [408, 429, 500, 502, 503, 504, 524, 529] {
            let (result, attempts, _) =
                fail_once(&no_jitter(1, 60_000), http_error(status, None)).await;
            assert!(result.is_ok(), "{status} is retried and recovers");
            assert_eq!(attempts, 2, "{status} gets one retry");
        }
    }

    #[test]
    fn untyped_errors_fall_back_to_message_heuristics() {
        assert_eq!(
            classify_provider_error(&anyhow::anyhow!("API error (503): busy")),
            RetryVerdict::Transient { after: None },
            "plain errors still use is_transient_error_message"
        );
        assert_eq!(
            classify_provider_error(&anyhow::anyhow!("API error (429): insufficient_quota")),
            RetryVerdict::Permanent,
            "billing markers stay permanent"
        );
    }

    #[test]
    fn typed_429_with_billing_message_is_permanent() {
        let err = anyhow::Error::new(ProviderHttpError {
            status: 429,
            message: "Insufficient balance, please recharge".into(),
            error_type: None,
            retry_after: Some(Duration::from_secs(1)),
        });
        assert_eq!(classify_provider_error(&err), RetryVerdict::Permanent);
    }

    #[test]
    fn parse_retry_after_accepts_delay_seconds_only() {
        assert_eq!(parse_retry_after("2"), Some(Duration::from_secs(2)));
        assert_eq!(parse_retry_after(" 30 "), Some(Duration::from_secs(30)));
        assert_eq!(
            parse_retry_after("Wed, 21 Oct 2026 07:28:00 GMT"),
            None,
            "HTTP-date needs the wall clock; fall back to backoff"
        );
        assert_eq!(parse_retry_after("-1"), None);
        assert_eq!(parse_retry_after(""), None);
    }

    #[test]
    fn provider_http_error_display_names_status_type_and_message() {
        let err = ProviderHttpError {
            status: 502,
            message: "Provider disconnected".into(),
            error_type: Some("provider_unavailable".into()),
            retry_after: None,
        };
        let text = err.to_string();
        assert!(text.contains("502"), "{text}");
        assert!(text.contains("provider_unavailable"), "{text}");
        assert!(text.contains("Provider disconnected"), "{text}");
    }
}
