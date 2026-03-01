//! Legacy page wrapper that embeds pre-rendered HTML inside the Askama base template.
//!
//! This allows existing pages (traces, logs, analytics, agents, webhooks) to use
//! the new icon-rail navigation shell without rewriting their rendering logic.

use askama::Template;

/// Wrapper template that extends `base.html`.
///
/// Fields like `page_title` and breadcrumb labels are auto-escaped by Askama.
/// Pre-rendered HTML fields (`content_html`, `page_css`, `page_js`) use the
/// `|safe` filter in the template to bypass escaping.
#[derive(Template)]
#[template(path = "wrapper.html")]
pub struct WrapperTemplate {
    pub active_page: String,
    pub page_title: String,
    pub breadcrumb_section: String,
    pub breadcrumb_page: String,
    pub page_css: String,
    pub content_html: String,
    pub page_js: String,
}

/// Render a legacy page inside the new app shell.
///
/// # Parameters
///
/// * `active_page` — Icon-rail highlight: `"traces"`, `"logs"`, `"analytics"`,
///   `"agents"`, `"webhooks"`.
/// * `page_title` — Browser tab title (without the ` - Assistant` suffix).
/// * `breadcrumb_section` — Top-level section name (e.g. `"Observability"`).
/// * `breadcrumb_page` — Current page name (e.g. `"Traces"`).
/// * `page_css` — Combined CSS string (typically `default_css()` + page-specific).
/// * `content_html` — Pre-rendered HTML for the content area.
/// * `page_js` — Optional JavaScript to inject (include `<script>` tags).
#[allow(dead_code)]
pub fn render_page(
    active_page: &str,
    page_title: &str,
    breadcrumb_section: &str,
    breadcrumb_page: &str,
    page_css: &str,
    content_html: &str,
    page_js: &str,
) -> String {
    let template = WrapperTemplate {
        active_page: active_page.to_string(),
        page_title: page_title.to_string(),
        breadcrumb_section: breadcrumb_section.to_string(),
        breadcrumb_page: breadcrumb_page.to_string(),
        page_css: page_css.to_string(),
        content_html: content_html.to_string(),
        page_js: page_js.to_string(),
    };
    template
        .render()
        .unwrap_or_else(|e| format!("Template render error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_page_contains_content() {
        let html = render_page(
            "traces",
            "Test Page",
            "Testing",
            "Unit Test",
            ".custom { color: red; }",
            "<p>Hello world</p>",
            "",
        );
        assert!(html.contains("<p>Hello world</p>"), "content_html present");
        assert!(html.contains("Test Page"), "page_title present");
        assert!(html.contains("Testing"), "breadcrumb_section present");
        assert!(html.contains("Unit Test"), "breadcrumb_page present");
        assert!(html.contains(".custom { color: red; }"), "page_css present");
        // Icon rail should mark traces as active
        assert!(
            html.contains("href=\"/traces\" class=\"rail-item active\""),
            "traces rail item active"
        );
    }

    #[test]
    fn render_page_includes_legacy_overrides() {
        let html = render_page(
            "logs",
            "Logs",
            "Observability",
            "Logs",
            "",
            "<div></div>",
            "",
        );
        assert!(
            html.contains(".content-area .layout"),
            "legacy override CSS present"
        );
    }

    #[test]
    fn render_page_includes_js() {
        let html = render_page(
            "agents",
            "Test",
            "Mgmt",
            "Test",
            "",
            "<p></p>",
            "<script>alert(1)</script>",
        );
        assert!(
            html.contains("<script>alert(1)</script>"),
            "page_js injected"
        );
    }
}
