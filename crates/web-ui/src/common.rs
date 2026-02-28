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
