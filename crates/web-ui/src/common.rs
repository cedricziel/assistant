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

/// Render the sidebar navigation shared by agent and webhook management pages.
///
/// `active` should be the lowercase label of the currently active page
/// (e.g. `"agents"`, `"webhooks"`).
pub fn render_sidebar(active: &str) -> String {
    let items = [
        ("Traces", "/traces"),
        ("Logs", "/logs"),
        ("Agents", "/agents"),
        ("Webhooks", "/webhooks"),
    ];
    let mut links = String::new();
    for (label, href) in &items {
        let class = if label.to_ascii_lowercase() == active {
            "facet-link active"
        } else {
            "facet-link"
        };
        links.push_str(&format!(
            "<li><a class=\"{class}\" href=\"{href}\"><span>{label}</span></a></li>",
            class = class,
            href = href,
            label = label,
        ));
    }

    format!(
        "<div class=\"sidebar-inner\">\
         <div class=\"brand\"><p>assistant</p><h2>Agent Manager</h2></div>\
         <div class=\"facet-group\">\
         <h3>Navigation</h3>\
         <ul>{links}</ul>\
         </div>\
         <div class=\"facet-footer\">\
         <form method=\"POST\" action=\"/logout\" style=\"margin:0\">\
         <button type=\"submit\" class=\"logout-btn\">Sign out</button>\
         </form>\
         </div>\
         </div>",
        links = links,
    )
}
