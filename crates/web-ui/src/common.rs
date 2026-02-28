//! Shared helpers used across web-ui page modules.

use axum::http::StatusCode;

/// HTML-escape a string to prevent XSS in server-rendered pages.
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

/// Shared CSS used by legacy pages (traces, logs, analytics, agents, webhooks).
pub fn default_css() -> &'static str {
    include_str!("default.css")
}

/// Render the sidebar for agent and webhook management pages.
///
/// Navigation links have been removed — the icon rail handles cross-page
/// navigation.  The sidebar retains the brand heading and section title
/// derived from `active`.
pub fn render_sidebar(active: &str) -> String {
    let heading = match active {
        "agents" => "Agents",
        "webhooks" => "Webhooks",
        _ => "Management",
    };

    format!(
        "<div class=\"sidebar-inner\">\
         <div class=\"brand\"><p>assistant</p><h2>{heading}</h2></div>\
         </div>",
        heading = heading,
    )
}
