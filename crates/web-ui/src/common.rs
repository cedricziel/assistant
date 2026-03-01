//! Shared helpers used across web-ui page modules.

use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use tracing::warn;

// -- Static asset URL trait --------------------------------------------------

/// Provides fingerprinted static asset URLs to Askama templates.
///
/// Implement this (empty `impl`) on every template struct that extends
/// `base.html`.  The template can then use `{{ self.app_css_url() }}`,
/// `{{ self.htmx_url() }}`, etc. — no per-struct fields required.
pub trait StaticUrls {
    /// Fingerprinted URL for the concatenated app stylesheet.
    fn app_css_url(&self) -> &'static str {
        crate::static_assets::app_css_url()
    }

    /// Fingerprinted URL for the vendored htmx script.
    fn htmx_url(&self) -> &'static str {
        crate::static_assets::htmx_url()
    }

    /// Fingerprinted URL for the vendored htmx-ext-sse script.
    fn htmx_sse_url(&self) -> &'static str {
        crate::static_assets::htmx_sse_url()
    }

    /// Fingerprinted URL for the app shell JS.
    fn app_js_url(&self) -> &'static str {
        crate::static_assets::app_js_url()
    }

    /// Fingerprinted URL for chat-specific JS.
    fn chat_js_url(&self) -> &'static str {
        crate::static_assets::chat_js_url()
    }

    /// Fingerprinted URL for the trace detail viewer JS.
    fn trace_detail_js_url(&self) -> &'static str {
        crate::static_assets::trace_detail_js_url()
    }

    /// Fingerprinted URL for the agent form validator JS.
    fn agent_form_js_url(&self) -> &'static str {
        crate::static_assets::agent_form_js_url()
    }
}

/// HTML-escape a string to prevent XSS.
///
/// Used by the chat SSE token stream and webhook test helpers.
/// Askama-based templates get auto-escaping for free.
pub fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Convert any `Display`-able error into an Axum-compatible 500 response pair.
pub fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

/// Percent-encode a string for use in URL query parameters.
pub fn url_encode(input: &str) -> String {
    input
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{:02X}", byte),
        })
        .collect()
}

/// Format a millisecond duration as a human-readable string.
pub fn format_duration(ms: i64) -> String {
    if ms >= 60_000 {
        format!("{:.1} min", ms as f64 / 60_000.0)
    } else if ms >= 1_000 {
        format!("{:.1} s", ms as f64 / 1_000.0)
    } else {
        format!("{ms} ms")
    }
}

/// Render an Askama template into an axum [`Response`].
///
/// On success returns `200 OK` with `text/html`.  On failure logs a
/// warning and returns `500 Internal Server Error`.
pub fn render_template(tmpl: impl Template) -> Response {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            warn!("Template render error: {}", e);
            if cfg!(debug_assertions) {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}
