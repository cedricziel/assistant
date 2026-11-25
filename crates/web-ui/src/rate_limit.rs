//! Rate limiting for the unauthenticated credential endpoints.
//!
//! Two independent defences:
//!
//! - a per-client-IP token bucket in front of `/login`, `/oauth/authorize`,
//!   `/oauth/token`, the device flow, `/oauth/revoke` (one bucket) and
//!   `/oauth/register` (a stricter one), enforced by [`enforce_ip_limits`];
//! - a per-account failed-login lockout, used by `POST /oauth/authorize`.
//!
//! Rejections are `429 Too Many Requests` with a `Retry-After` header.
//! `X-Forwarded-For` is only honoured when the operator opts in, because a
//! direct client could otherwise pick its own bucket.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use assistant_core::clock::Clock;

/// Buckets are purged once the map grows past this many entries.
const PURGE_THRESHOLD: usize = 10_000;

/// A token-bucket shape: up to `burst` tokens, one refilled every `refill_every`.
#[derive(Debug, Clone, Copy)]
pub struct Limit {
    pub burst: u32,
    pub refill_every: Duration,
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last: Instant,
}

/// Keyed token-bucket limiter driven by an injectable [`Clock`].
pub struct RateLimiter {
    clock: Arc<dyn Clock>,
    limit: Limit,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimiter {
    pub fn new(clock: Arc<dyn Clock>, limit: Limit) -> Self {
        Self {
            clock,
            limit,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Take one token for `key`, or report how long until one is available.
    pub fn try_acquire(&self, key: &str) -> Result<(), Duration> {
        let now = self.clock.now_instant();
        let mut buckets = self.lock();
        if buckets.len() >= PURGE_THRESHOLD && !buckets.contains_key(key) {
            self.purge(&mut buckets, now);
        }
        let bucket = buckets.entry(key.to_owned()).or_insert(Bucket {
            tokens: f64::from(self.limit.burst),
            last: now,
        });
        self.refill(bucket, now);
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err(self.wait_for_token(bucket))
        }
    }

    /// Report whether `key` has a token without taking it.
    pub fn check(&self, key: &str) -> Result<(), Duration> {
        let now = self.clock.now_instant();
        let mut buckets = self.lock();
        let Some(bucket) = buckets.get_mut(key) else {
            return Ok(());
        };
        self.refill(bucket, now);
        if bucket.tokens >= 1.0 {
            Ok(())
        } else {
            Err(self.wait_for_token(bucket))
        }
    }

    /// Forget `key`, restoring its full allowance.
    pub fn reset(&self, key: &str) {
        self.lock().remove(key);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Bucket>> {
        self.buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn refill(&self, bucket: &mut Bucket, now: Instant) {
        let elapsed = now.saturating_duration_since(bucket.last);
        let gained = elapsed.as_secs_f64() / self.limit.refill_every.as_secs_f64();
        bucket.tokens = (bucket.tokens + gained).min(f64::from(self.limit.burst));
        bucket.last = now;
    }

    fn wait_for_token(&self, bucket: &Bucket) -> Duration {
        let missing = 1.0 - bucket.tokens;
        let secs = (missing * self.limit.refill_every.as_secs_f64()).ceil();
        Duration::from_secs((secs as u64).max(1))
    }

    /// Drop buckets that have fully refilled; they carry no state.
    fn purge(&self, buckets: &mut HashMap<String, Bucket>, now: Instant) {
        let burst = f64::from(self.limit.burst);
        buckets.retain(|_, bucket| {
            self.refill(bucket, now);
            bucket.tokens < burst
        });
    }
}

const DEFAULT_AUTH_PER_MINUTE: u32 = 30;
const DEFAULT_REGISTER_PER_HOUR: u32 = 10;
const DEFAULT_LOGIN_MAX_FAILURES: u32 = 10;

/// Operator-facing rate-limit settings.
#[derive(clap::Args, Debug, Clone)]
pub struct RateLimitArgs {
    /// Credential-endpoint requests allowed per minute per client IP
    /// (`/login`, `/oauth/authorize`, `/oauth/token`, device flow, revoke).
    #[arg(
        long,
        env = "ASSISTANT_WEB_AUTH_RATE_LIMIT",
        default_value_t = DEFAULT_AUTH_PER_MINUTE,
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    pub auth_rate_limit_per_minute: u32,

    /// Dynamic client registrations allowed per hour per client IP.
    #[arg(
        long,
        env = "ASSISTANT_WEB_REGISTER_RATE_LIMIT",
        default_value_t = DEFAULT_REGISTER_PER_HOUR,
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    pub register_rate_limit_per_hour: u32,

    /// Failed password attempts per account before it is locked out. One
    /// attempt is restored per minute.
    #[arg(
        long,
        env = "ASSISTANT_WEB_LOGIN_MAX_FAILURES",
        default_value_t = DEFAULT_LOGIN_MAX_FAILURES,
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    pub login_max_failures: u32,

    /// Take the client IP from the rightmost `X-Forwarded-For` entry. Enable
    /// only behind a reverse proxy that appends the real peer address;
    /// otherwise clients could choose their own rate-limit bucket.
    #[arg(long, env = "ASSISTANT_WEB_TRUST_FORWARDED_FOR")]
    pub trust_forwarded_for: bool,
}

impl Default for RateLimitArgs {
    fn default() -> Self {
        Self {
            auth_rate_limit_per_minute: DEFAULT_AUTH_PER_MINUTE,
            register_rate_limit_per_hour: DEFAULT_REGISTER_PER_HOUR,
            login_max_failures: DEFAULT_LOGIN_MAX_FAILURES,
            trust_forwarded_for: false,
        }
    }
}

/// Limiters shared by the router middleware and the OAuth handlers.
pub struct RateLimits {
    pub(crate) auth: RateLimiter,
    pub(crate) register: RateLimiter,
    /// Failed password attempts, keyed by normalised account email.
    pub(crate) login_failures: RateLimiter,
    pub(crate) trust_forwarded_for: bool,
}

impl RateLimits {
    pub fn new(clock: Arc<dyn Clock>, args: &RateLimitArgs) -> Self {
        let per = |window_secs: f64, count: u32| Limit {
            burst: count,
            refill_every: Duration::from_secs_f64(window_secs / f64::from(count)),
        };
        Self {
            auth: RateLimiter::new(clock.clone(), per(60.0, args.auth_rate_limit_per_minute)),
            register: RateLimiter::new(
                clock.clone(),
                per(3600.0, args.register_rate_limit_per_hour),
            ),
            login_failures: RateLimiter::new(
                clock,
                Limit {
                    burst: args.login_max_failures,
                    refill_every: Duration::from_secs(60),
                },
            ),
            trust_forwarded_for: args.trust_forwarded_for,
        }
    }
}

/// Resolve the client IP: the rightmost `X-Forwarded-For` entry when trusted
/// and parseable, otherwise the socket peer.
fn client_ip(headers: &HeaderMap, peer: Option<IpAddr>, trust_forwarded_for: bool) -> IpAddr {
    if trust_forwarded_for
        && let Some(forwarded) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse::<IpAddr>().ok())
    {
        return forwarded;
    }
    peer.unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
}

/// IPv6 clients usually own a whole /64, so bucket them by that prefix.
fn bucket_key(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
        }
    }
}

#[derive(Clone, Copy)]
enum Class {
    Auth,
    Register,
}

fn classify(method: &Method, path: &str) -> Option<Class> {
    if method != Method::POST {
        return None;
    }
    match path {
        "/login"
        | "/oauth/authorize"
        | "/oauth/token"
        | "/oauth/device"
        | "/oauth/device/verify"
        | "/oauth/revoke" => Some(Class::Auth),
        "/oauth/register" => Some(Class::Register),
        _ => None,
    }
}

/// Build the `429` response for a rejected request.
pub(crate) fn too_many_requests(retry_after: Duration) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, retry_after.as_secs().to_string())],
        "Too many requests",
    )
        .into_response()
}

/// Router middleware applying the per-IP limits to credential endpoints.
pub async fn enforce_ip_limits(
    State(limits): State<Arc<RateLimits>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(class) = classify(request.method(), request.uri().path()) else {
        return next.run(request).await;
    };
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip());
    let ip = client_ip(request.headers(), peer, limits.trust_forwarded_for);
    let limiter = match class {
        Class::Auth => &limits.auth,
        Class::Register => &limits.register,
    };
    match limiter.try_acquire(&bucket_key(ip)) {
        Ok(()) => next.run(request).await,
        Err(retry_after) => {
            tracing::warn!(%ip, path = request.uri().path(), "rate limit exceeded");
            too_many_requests(retry_after)
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::routing::post;
    use tower::ServiceExt;

    use assistant_core::clock::FakeClock;

    use super::*;

    fn limiter(burst: u32, refill_secs: u64) -> (RateLimiter, FakeClock) {
        let clock = FakeClock::new_at_seed();
        let limiter = RateLimiter::new(
            Arc::new(clock.clone()),
            Limit {
                burst,
                refill_every: Duration::from_secs(refill_secs),
            },
        );
        (limiter, clock)
    }

    #[test]
    fn allows_a_burst_then_rejects_with_retry_after() {
        let (limiter, _clock) = limiter(3, 10);
        for _ in 0..3 {
            assert!(limiter.try_acquire("a").is_ok());
        }
        let retry = limiter.try_acquire("a").unwrap_err();
        assert_eq!(retry, Duration::from_secs(10));
    }

    #[test]
    fn keys_are_independent() {
        let (limiter, _clock) = limiter(1, 10);
        assert!(limiter.try_acquire("a").is_ok());
        assert!(limiter.try_acquire("a").is_err());
        assert!(limiter.try_acquire("b").is_ok());
    }

    #[test]
    fn tokens_refill_over_time() {
        let (limiter, clock) = limiter(1, 10);
        assert!(limiter.try_acquire("a").is_ok());
        clock.advance(Duration::from_secs(4));
        assert_eq!(
            limiter.try_acquire("a").unwrap_err(),
            Duration::from_secs(6),
            "retry-after shrinks as time passes"
        );
        clock.advance(Duration::from_secs(6));
        assert!(limiter.try_acquire("a").is_ok());
    }

    #[test]
    fn refill_never_exceeds_burst() {
        let (limiter, clock) = limiter(2, 10);
        clock.advance(Duration::from_secs(3600));
        assert!(limiter.try_acquire("a").is_ok());
        assert!(limiter.try_acquire("a").is_ok());
        assert!(limiter.try_acquire("a").is_err());
    }

    #[test]
    fn retry_after_rounds_up_to_whole_seconds() {
        let (limiter, clock) = limiter(1, 10);
        assert!(limiter.try_acquire("a").is_ok());
        clock.advance(Duration::from_millis(9_500));
        assert_eq!(
            limiter.try_acquire("a").unwrap_err(),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn check_does_not_consume() {
        let (limiter, _clock) = limiter(1, 10);
        assert!(limiter.check("a").is_ok());
        assert!(limiter.check("a").is_ok());
        assert!(limiter.try_acquire("a").is_ok());
        assert!(limiter.check("a").is_err());
    }

    #[test]
    fn reset_restores_the_allowance() {
        let (limiter, _clock) = limiter(1, 10);
        assert!(limiter.try_acquire("a").is_ok());
        assert!(limiter.try_acquire("a").is_err());
        limiter.reset("a");
        assert!(limiter.try_acquire("a").is_ok());
    }

    #[test]
    fn idle_buckets_are_purged_when_the_map_grows() {
        let (limiter, clock) = limiter(1, 1);
        for i in 0..PURGE_THRESHOLD {
            let _ = limiter.try_acquire(&format!("k{i}"));
        }
        clock.advance(Duration::from_secs(5));
        let _ = limiter.try_acquire("trigger");
        assert!(
            limiter.buckets.lock().unwrap().len() < PURGE_THRESHOLD,
            "fully refilled buckets should be dropped"
        );
    }

    // -- client_ip ---------------------------------------------------------

    fn headers_with_xff(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", value.parse().unwrap());
        h
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn forwarded_for_is_ignored_unless_trusted() {
        let got = client_ip(&headers_with_xff("9.9.9.9"), Some(ip("10.0.0.1")), false);
        assert_eq!(got, ip("10.0.0.1"));
    }

    #[test]
    fn trusted_forwarded_for_uses_the_rightmost_entry() {
        let got = client_ip(
            &headers_with_xff("1.1.1.1, 2.2.2.2, 3.3.3.3"),
            Some(ip("10.0.0.1")),
            true,
        );
        assert_eq!(got, ip("3.3.3.3"), "the proxy appends the real peer last");
    }

    #[test]
    fn trusted_but_garbage_forwarded_for_falls_back_to_the_peer() {
        let got = client_ip(&headers_with_xff("not-an-ip"), Some(ip("10.0.0.1")), true);
        assert_eq!(got, ip("10.0.0.1"));
    }

    #[test]
    fn ipv6_clients_share_a_bucket_per_64_prefix() {
        let a = bucket_key(ip("2001:db8:1:2:aaaa::1"));
        let b = bucket_key(ip("2001:db8:1:2:bbbb::2"));
        let c = bucket_key(ip("2001:db8:1:3::1"));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(bucket_key(ip("192.0.2.7")), "192.0.2.7");
    }

    // -- middleware ---------------------------------------------------------

    fn limits(clock: &FakeClock, trust_forwarded_for: bool) -> Arc<RateLimits> {
        Arc::new(RateLimits::new(
            Arc::new(clock.clone()),
            &RateLimitArgs {
                auth_rate_limit_per_minute: 3,
                register_rate_limit_per_hour: 2,
                login_max_failures: 5,
                trust_forwarded_for,
            },
        ))
    }

    fn app(limits: Arc<RateLimits>, peer: &str) -> Router {
        async fn ok() -> &'static str {
            "ok"
        }
        let peer: SocketAddr = format!("{peer}:4000").parse().unwrap();
        Router::new()
            .route("/login", post(ok).get(ok))
            .route("/oauth/token", post(ok))
            .route("/oauth/register", post(ok))
            .route("/api/things", post(ok))
            .layer(axum::middleware::from_fn_with_state(
                limits,
                enforce_ip_limits,
            ))
            .layer(axum::Extension(ConnectInfo(peer)))
    }

    fn req(method: &str, uri: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    async fn status(app: &Router, method: &str, uri: &str) -> StatusCode {
        app.clone()
            .oneshot(req(method, uri))
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn auth_endpoints_return_429_with_retry_after_once_the_burst_is_spent() {
        let clock = FakeClock::new_at_seed();
        let app = app(limits(&clock, false), "10.0.0.1");
        for _ in 0..3 {
            assert_eq!(status(&app, "POST", "/oauth/token").await, StatusCode::OK);
        }
        let resp = app.clone().oneshot(req("POST", "/login")).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "/login shares the auth bucket"
        );
        let retry: u64 = resp
            .headers()
            .get(header::RETRY_AFTER)
            .expect("Retry-After header")
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!((1..=20).contains(&retry), "unexpected Retry-After {retry}");
    }

    #[tokio::test]
    async fn the_bucket_refills() {
        let clock = FakeClock::new_at_seed();
        let app = app(limits(&clock, false), "10.0.0.1");
        for _ in 0..3 {
            status(&app, "POST", "/oauth/token").await;
        }
        assert_eq!(
            status(&app, "POST", "/oauth/token").await,
            StatusCode::TOO_MANY_REQUESTS
        );
        clock.advance(Duration::from_secs(20));
        assert_eq!(status(&app, "POST", "/oauth/token").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn registration_has_its_own_stricter_bucket() {
        let clock = FakeClock::new_at_seed();
        let app = app(limits(&clock, false), "10.0.0.1");
        assert_eq!(
            status(&app, "POST", "/oauth/register").await,
            StatusCode::OK
        );
        assert_eq!(
            status(&app, "POST", "/oauth/register").await,
            StatusCode::OK
        );
        assert_eq!(
            status(&app, "POST", "/oauth/register").await,
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            status(&app, "POST", "/oauth/token").await,
            StatusCode::OK,
            "registration does not drain the auth bucket"
        );
    }

    #[tokio::test]
    async fn safe_methods_and_other_routes_are_not_limited() {
        let clock = FakeClock::new_at_seed();
        let app = app(limits(&clock, false), "10.0.0.1");
        for _ in 0..10 {
            assert_eq!(status(&app, "GET", "/login").await, StatusCode::OK);
            assert_eq!(status(&app, "POST", "/api/things").await, StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn peers_are_limited_independently() {
        let clock = FakeClock::new_at_seed();
        let shared = limits(&clock, false);
        let first = app(shared.clone(), "10.0.0.1");
        let second = app(shared, "10.0.0.2");
        for _ in 0..3 {
            status(&first, "POST", "/oauth/token").await;
        }
        assert_eq!(
            status(&first, "POST", "/oauth/token").await,
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            status(&second, "POST", "/oauth/token").await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn spoofed_forwarded_for_does_not_dodge_the_limit_by_default() {
        let clock = FakeClock::new_at_seed();
        let app = app(limits(&clock, false), "10.0.0.1");
        for i in 0..3 {
            let r = Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("x-forwarded-for", format!("8.8.8.{i}"))
                .body(Body::empty())
                .unwrap();
            assert_eq!(
                app.clone().oneshot(r).await.unwrap().status(),
                StatusCode::OK
            );
        }
        let r = Request::builder()
            .method("POST")
            .uri("/oauth/token")
            .header("x-forwarded-for", "8.8.8.99")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.oneshot(r).await.unwrap().status(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[tokio::test]
    async fn trusted_forwarded_for_buckets_by_the_forwarded_client() {
        let clock = FakeClock::new_at_seed();
        let app = app(limits(&clock, true), "10.0.0.1");
        let via = |client: &str| {
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("x-forwarded-for", client.to_string())
                .body(Body::empty())
                .unwrap()
        };
        for _ in 0..3 {
            app.clone().oneshot(via("8.8.8.8")).await.unwrap();
        }
        assert_eq!(
            app.clone().oneshot(via("8.8.8.8")).await.unwrap().status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            app.oneshot(via("9.9.9.9")).await.unwrap().status(),
            StatusCode::OK,
            "a different forwarded client has its own bucket"
        );
    }
}
