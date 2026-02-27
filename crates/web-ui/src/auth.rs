//! Token-based authentication for the web UI.
//!
//! Provides a login page (cookie-based sessions for browsers) and Bearer token
//! validation for A2A / API callers.  The server **requires** an auth token to
//! start — see [`AuthConfig`] and the `--auth-token` / `ASSISTANT_WEB_TOKEN`
//! environment variable.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Extension, Request};
use axum::http::header::{COOKIE, LOCATION, SET_COOKIE};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::common::html_escape;

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
    if let Some(cookie_header) = request.headers().get(COOKIE) {
        if let Ok(cookies) = cookie_header.to_str() {
            if extract_cookie(cookies, SESSION_COOKIE)
                .map(|v| constant_time_eq(v.as_bytes(), auth.session_value.as_bytes()))
                .unwrap_or(false)
            {
                return next.run(request).await;
            }
        }
    }

    // 2. Check Authorization: Bearer <token>.
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(value) = auth_header.to_str() {
            if let Some(bearer) = value.strip_prefix("Bearer ") {
                if constant_time_eq(bearer.trim().as_bytes(), auth.token.as_bytes()) {
                    return next.run(request).await;
                }
            }
        }
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

// -- Login page -------------------------------------------------------------

/// `GET /login` — render the login form.
pub async fn login_page() -> Response {
    // Always show the login form.  The auth middleware on protected pages
    // handles the case where the user already has a valid session.
    Html(render_login_page(None)).into_response()
}

/// `POST /login` — validate the submitted token and set a session cookie.
#[derive(Deserialize)]
pub struct LoginForm {
    token: String,
}

pub async fn login_submit(
    Extension(auth): Extension<AuthConfig>,
    Form(form): Form<LoginForm>,
) -> Response {
    if constant_time_eq(form.token.as_bytes(), auth.token.as_bytes()) {
        // Set session cookie and redirect to dashboard.
        let secure = if auth.secure_cookie { "; Secure" } else { "" };
        let cookie = format!(
            "{}={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800{}",
            SESSION_COOKIE, auth.session_value, secure,
        );
        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, "/")
            .header(SET_COOKIE, cookie)
            .body(Body::empty())
            .unwrap()
    } else {
        Html(render_login_page(Some("Invalid token."))).into_response()
    }
}

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

/// Render the login page HTML.
fn render_login_page(error: Option<&str>) -> String {
    let error_html = error
        .map(|msg| format!("<div class=\"login-error\">{}</div>", html_escape(msg)))
        .unwrap_or_default();

    format!(
        r#"<!DOCTYPE html>
<html><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Login - Assistant</title>
<style>{css}
{login_css}
</style>
</head><body>
<div class="login-container">
  <div class="login-card">
    <div class="login-brand">
      <p>assistant</p>
      <h2>Agent Manager</h2>
    </div>
    {error}
    <form method="POST" action="/login" class="login-form">
      <label for="token">Auth Token</label>
      <input type="password" id="token" name="token" placeholder="Enter your token" required autofocus>
      <button type="submit">Sign in</button>
    </form>
  </div>
</div>
</body></html>"#,
        css = crate::default_css(),
        login_css = login_css(),
        error = error_html,
    )
}

/// CSS specific to the login page.
fn login_css() -> &'static str {
    r#"
    .login-container {
        display: flex;
        align-items: center;
        justify-content: center;
        min-height: 100vh;
        padding: 2rem;
    }
    .login-card {
        background: #0a1628;
        border: 1px solid #0b1b32;
        border-radius: 12px;
        padding: 2.5rem;
        width: 100%;
        max-width: 400px;
    }
    .login-brand {
        margin-bottom: 2rem;
    }
    .login-brand p {
        text-transform: uppercase;
        letter-spacing: 0.2em;
        color: #7aa2ff;
        margin: 0 0 0.2rem;
        font-size: 0.75rem;
    }
    .login-brand h2 {
        margin: 0;
        color: #e5e9f0;
    }
    .login-error {
        background: rgba(239, 68, 68, 0.15);
        border: 1px solid rgba(239, 68, 68, 0.3);
        color: #fca5a5;
        padding: 0.75rem 1rem;
        border-radius: 8px;
        margin-bottom: 1.5rem;
        font-size: 0.9rem;
    }
    .login-form {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }
    .login-form label {
        font-size: 0.85rem;
        color: #8aa5d8;
        text-transform: uppercase;
        letter-spacing: 0.08em;
    }
    .login-form input {
        background: #030712;
        border: 1px solid #1a2744;
        border-radius: 8px;
        padding: 0.7rem 1rem;
        color: #e5e9f0;
        font-size: 1rem;
        outline: none;
        transition: border-color 0.15s;
    }
    .login-form input:focus {
        border-color: #6ec6ff;
    }
    .login-form button {
        background: #2563eb;
        color: #fff;
        border: none;
        border-radius: 8px;
        padding: 0.7rem;
        font-size: 1rem;
        cursor: pointer;
        margin-top: 0.5rem;
        transition: background 0.15s;
    }
    .login-form button:hover {
        background: #1d4ed8;
    }
    "#
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
