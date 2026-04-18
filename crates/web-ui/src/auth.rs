//! Token-based authentication for the web UI.
//!
//! Provides cookie-based sessions (browser flow) and Bearer token validation
//! (API / A2A callers).  The login UI is rendered by the Flutter SPA; this
//! module handles only the `POST /login` form submission and session management.
//! The server **requires** an auth token to start — see [`AuthConfig`] and the
//! `--auth-token` / `ASSISTANT_WEB_TOKEN` environment variable.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Extension, Request};
use axum::http::header::{COOKIE, HOST, LOCATION, ORIGIN, REFERER, SET_COOKIE};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

// -- Configuration ----------------------------------------------------------

/// Session cookie name.  `__Host-` prefix is only valid over HTTPS; since
/// the UI commonly runs over plain HTTP on localhost we use a simpler name.
const SESSION_COOKIE: &str = "assistant_session";

/// HMAC message used to derive the session token from the auth token.
const SESSION_HMAC_MSG: &[u8] = b"assistant-web-session-v1";

/// Shared authentication configuration injected via [`Extension`].
#[derive(Clone)]
pub struct AuthConfig {
    /// The raw auth token (for Bearer comparison).
    token: Arc<String>,
    /// Pre-computed HMAC-SHA256 hex digest used as the session cookie value.
    session_value: Arc<String>,
    /// When `true`, the `Secure` attribute is added to session cookies.
    /// Should be `true` whenever the server is *not* bound to a loopback
    /// address (cookies must only travel over HTTPS in that case).
    secure_cookie: bool,
}

impl AuthConfig {
    /// Create a new [`AuthConfig`] from the raw token string.
    ///
    /// Set `secure_cookie` to `true` when the server binds to a non-loopback
    /// address so that the session cookie gets the `Secure` attribute.
    pub fn new(token: String, secure_cookie: bool) -> Self {
        let session_value = compute_session_value(&token);
        Self {
            token: Arc::new(token),
            session_value: Arc::new(session_value),
            secure_cookie,
        }
    }
}

// -- Session token derivation -----------------------------------------------

type HmacSha256 = Hmac<Sha256>;

/// Derive a stable session cookie value from the auth token using HMAC-SHA256.
///
/// The cookie never contains the raw token — only this derived value.
fn compute_session_value(auth_token: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(auth_token.as_bytes()).expect("HMAC accepts any key length");
    mac.update(SESSION_HMAC_MSG);
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time comparison of two equal-length byte slices.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

// -- Middleware --------------------------------------------------------------

/// Axum middleware that enforces authentication on every matched route.
///
/// Accepts either:
/// - A valid session cookie (browser flow), or
/// - An `Authorization: Bearer <token>` header (API / A2A flow).
///
/// Unauthenticated browser requests are redirected to `/login`.
/// Unauthenticated API requests receive `401 Unauthorized`.
pub async fn require_auth(
    Extension(auth): Extension<AuthConfig>,
    request: Request,
    next: Next,
) -> Response {
    // 1. Check session cookie.
    if let Some(cookie_header) = request.headers().get(COOKIE)
        && let Ok(cookies) = cookie_header.to_str()
        && extract_cookie(cookies, SESSION_COOKIE)
            .map(|v| constant_time_eq(v.as_bytes(), auth.session_value.as_bytes()))
            .unwrap_or(false)
    {
        return next.run(request).await;
    }

    // 2. Check Authorization: Bearer <token>.
    if let Some(auth_header) = request.headers().get("authorization")
        && let Ok(value) = auth_header.to_str()
        && let Some(bearer) = value.strip_prefix("Bearer ")
        && constant_time_eq(bearer.trim().as_bytes(), auth.token.as_bytes())
    {
        return next.run(request).await;
    }

    // 3. Not authenticated — decide response type.
    let accepts_html = request
        .headers()
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false);

    if accepts_html {
        // Redirect browsers to the login page.
        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, "/login")
            .body(Body::empty())
            .unwrap()
    } else {
        // Return 401 for API callers.
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("WWW-Authenticate", "Bearer")
            .body(Body::from("Unauthorized"))
            .unwrap()
    }
}

/// Middleware that enforces same-origin checks for cookie-authenticated mutations.
///
/// Browser CSRF relies on ambient cookies on cross-site requests. For state-changing
/// methods we reject requests whose `Origin`/`Referer` do not match `Host`.
/// Bearer-authenticated API requests are exempt from this check.
pub async fn require_same_origin_mutation(request: Request, next: Next) -> Response {
    match *request.method() {
        Method::GET | Method::HEAD | Method::OPTIONS => return next.run(request).await,
        _ => {}
    }

    if request
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("Bearer "))
        .unwrap_or(false)
    {
        return next.run(request).await;
    }

    let Some(host) = request.headers().get(HOST).and_then(|h| h.to_str().ok()) else {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Forbidden"))
            .unwrap();
    };

    let origin_host = request
        .headers()
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_host_from_url)
        .or_else(|| {
            request
                .headers()
                .get(REFERER)
                .and_then(|value| value.to_str().ok())
                .and_then(parse_host_from_url)
        });

    let Some(origin_host) = origin_host else {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Forbidden"))
            .unwrap();
    };

    if !constant_time_eq(origin_host.as_bytes(), host.as_bytes()) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Forbidden"))
            .unwrap();
    }

    next.run(request).await
}

// -- Session ----------------------------------------------------------------

/// `POST /logout` — clear the session cookie and redirect to login.
pub async fn logout(Extension(auth): Extension<AuthConfig>) -> Response {
    let secure = if auth.secure_cookie { "; Secure" } else { "" };
    let cookie = format!(
        "{}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0{}",
        SESSION_COOKIE, secure,
    );
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(LOCATION, "/login")
        .header(SET_COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

// -- Helpers ----------------------------------------------------------------

/// Extract a named cookie value from a `Cookie` header string.
fn extract_cookie<'a>(cookies: &'a str, name: &str) -> Option<&'a str> {
    for pair in cookies.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix(name) {
            let value = value.trim_start();
            if let Some(value) = value.strip_prefix('=') {
                return Some(value.trim());
            }
        }
    }
    None
}

fn parse_host_from_url(value: &str) -> Option<String> {
    let value = value.trim();
    let value = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))?;
    let host = value.split('/').next()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_session_value_deterministic() {
        let a = compute_session_value("my-secret");
        let b = compute_session_value("my-secret");
        assert_eq!(a, b, "same token should produce same session value");
    }

    #[test]
    fn test_compute_session_value_differs_for_different_tokens() {
        let a = compute_session_value("token-a");
        let b = compute_session_value("token-b");
        assert_ne!(a, b);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"short", b"longer"));
    }

    #[test]
    fn test_extract_cookie() {
        let cookies = "foo=bar; assistant_session=abc123; other=val";
        assert_eq!(extract_cookie(cookies, "assistant_session"), Some("abc123"));
        assert_eq!(extract_cookie(cookies, "foo"), Some("bar"));
        assert_eq!(extract_cookie(cookies, "missing"), None);
    }

    #[test]
    fn test_extract_cookie_single() {
        assert_eq!(
            extract_cookie("assistant_session=xyz", "assistant_session"),
            Some("xyz")
        );
    }
}
