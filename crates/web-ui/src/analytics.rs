//! Analytics dashboard page.

use assistant_storage::MetricsStore;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Html,
};
use serde::Deserialize;

use crate::common::{default_css, html_escape, internal_error};
use crate::legacy;
use crate::AppState;

#[derive(Debug, Default, Deserialize)]
struct AnalyticsQuery {
    /// Time window in hours (defaults to 24).
    window: Option<i64>,
}

/// Returns the sub-router for analytics-related routes.
pub(crate) fn analytics_router() -> axum::Router<AppState> {
    axum::Router::new().route("/analytics", axum::routing::get(show_analytics))
}

async fn show_analytics(
    State(state): State<AppState>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    let window_hours = query.window.unwrap_or(24);
    let store = MetricsStore::new(state.pool.clone());

    let summary = store.summary(window_hours).await.map_err(internal_error)?;
    let models = store
        .model_comparison(window_hours)
        .await
        .map_err(internal_error)?;
    let tools = store
        .tool_usage(window_hours)
        .await
        .map_err(internal_error)?;
    let token_series = store
        .token_usage_over_time(window_hours, 15)
        .await
        .map_err(internal_error)?;
    let request_series = store
        .request_rate(window_hours, 15)
        .await
        .map_err(internal_error)?;

    // -- Summary cards --------------------------------------------------------
    let cards = format!(
        "<div class=\"analytics-cards\">\
         <div class=\"a-card\"><h4>Total Tokens (In)</h4><p class=\"a-big\">{}</p></div>\
         <div class=\"a-card\"><h4>Total Tokens (Out)</h4><p class=\"a-big\">{}</p></div>\
         <div class=\"a-card\"><h4>Requests</h4><p class=\"a-big\">{}</p></div>\
         <div class=\"a-card\"><h4>Tool Calls</h4><p class=\"a-big\">{}</p></div>\
         <div class=\"a-card\"><h4>Avg Duration</h4><p class=\"a-big\">{:.2}s</p></div>\
         <div class=\"a-card\"><h4>Errors</h4><p class=\"a-big a-err\">{}</p></div>\
         </div>",
        format_number(summary.total_tokens_in),
        format_number(summary.total_tokens_out),
        format_number(summary.total_requests),
        format_number(summary.total_tool_invocations),
        summary.avg_duration_s,
        summary.error_count,
    );

    // -- Token usage sparkline ------------------------------------------------
    let token_chart = render_bar_chart(
        "Token Usage Over Time",
        &token_series
            .iter()
            .map(|p| (p.bucket.as_str(), p.value))
            .collect::<Vec<_>>(),
        "#60a5fa",
    );

    // -- Request rate sparkline -----------------------------------------------
    let request_chart = render_bar_chart(
        "Requests Over Time",
        &request_series
            .iter()
            .map(|p| (p.bucket.as_str(), p.value))
            .collect::<Vec<_>>(),
        "#34d399",
    );

    // -- Model comparison table -----------------------------------------------
    let model_rows: String = models
        .iter()
        .map(|m| {
            format!(
                "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td>\
                 <td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
                html_escape(&m.model),
                format_number(m.input_tokens),
                format_number(m.output_tokens),
                format_number(m.input_tokens + m.output_tokens),
                m.request_count,
            )
        })
        .collect();
    let model_table = format!(
        "<div class=\"panel\"><div class=\"panel-head\"><h3>Model Comparison</h3></div>\
         <table class=\"trace-table\"><thead><tr>\
         <th>Model</th><th>Input Tokens</th><th>Output Tokens</th>\
         <th>Total Tokens</th><th>Requests</th>\
         </tr></thead><tbody>{model_rows}</tbody></table></div>"
    );

    // -- Tool usage table -----------------------------------------------------
    let tool_rows: String = tools
        .iter()
        .map(|t| {
            format!(
                "<tr><td>{}</td><td class=\"num\">{}</td></tr>",
                html_escape(&t.tool_name),
                t.invocations,
            )
        })
        .collect();
    let tool_table = format!(
        "<div class=\"panel\"><div class=\"panel-head\"><h3>Tool Usage</h3></div>\
         <table class=\"trace-table\"><thead><tr>\
         <th>Tool</th><th>Invocations</th>\
         </tr></thead><tbody>{tool_rows}</tbody></table></div>"
    );

    // -- Window selector sidebar ----------------------------------------------
    let window_options: &[(i64, &str)] =
        &[(1, "1h"), (6, "6h"), (24, "24h"), (72, "3d"), (168, "7d")];
    let window_links: String = window_options
        .iter()
        .map(|(h, label)| {
            let active = if *h == window_hours { " active" } else { "" };
            format!(
                "<li><a class=\"facet-link{active}\" href=\"/analytics?window={h}\">\
                 <span>{label}</span></a></li>"
            )
        })
        .collect();

    let sidebar = format!(
        "<div class=\"sidebar-inner\">\
         <div class=\"brand\"><p>assistant</p><h2>Analytics</h2></div>\
         <div class=\"facet-group\">\
         <h3>Time Window</h3>\
         <ul>{window_links}</ul></div>\
         </div>"
    );

    let sections = format!(
        "{cards}\
         <div class=\"analytics-charts\">{token_chart}{request_chart}</div>\
         {model_table}{tool_table}"
    );

    let content_html = format!(
        "<div class=\"layout\">\
         <aside class=\"sidebar\">{sidebar}</aside>\
         <main class=\"main\">{sections}</main>\
         </div>",
    );

    let page_css = format!("{}\n{}", default_css(), analytics_css());
    let body = legacy::render_page(
        "analytics",
        "Analytics",
        "Observability",
        "Analytics",
        &page_css,
        &content_html,
        "",
    );

    Ok(Html(body))
}

/// Render an SVG bar chart (inline, server-rendered).
fn render_bar_chart(title: &str, data: &[(&str, f64)], color: &str) -> String {
    if data.is_empty() {
        return format!(
            "<div class=\"panel\"><div class=\"panel-head\"><h3>{title}</h3></div>\
             <p class=\"a-empty\">No data in this window</p></div>"
        );
    }

    let max_val = data.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max);
    let chart_w = 600;
    let chart_h = 120;
    let bar_gap = 2;
    let n = data.len();
    let bar_w = if n > 0 {
        ((chart_w as f64 - (n as f64 * bar_gap as f64)) / n as f64).max(2.0) as usize
    } else {
        4
    };

    let mut bars = String::new();
    for (i, (label, val)) in data.iter().enumerate() {
        let h = if max_val > 0.0 {
            (val / max_val * chart_h as f64).max(1.0)
        } else {
            1.0
        };
        let x = i * (bar_w + bar_gap);
        let y = chart_h as f64 - h;
        // Shorten the bucket label to just HH:MM for tooltips.
        let short_label = if label.len() >= 14 {
            &label[11..16]
        } else {
            label
        };
        bars.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y:.0}\" width=\"{bar_w}\" height=\"{h:.0}\" \
             fill=\"{color}\" rx=\"1\">\
             <title>{short_label}: {val:.0}</title></rect>"
        ));
    }

    format!(
        "<div class=\"panel\"><div class=\"panel-head\"><h3>{title}</h3></div>\
         <svg class=\"a-chart\" viewBox=\"0 0 {chart_w} {chart_h}\" \
         preserveAspectRatio=\"none\">{bars}</svg></div>"
    )
}

/// Format a number with thousand separators for display.
fn format_number(n: i64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let s = n.abs().to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    if n < 0 {
        result.push('-');
    }
    result.chars().rev().collect()
}

fn analytics_css() -> &'static str {
    r#"
    .analytics-cards {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
        gap: 12px;
        margin-bottom: 20px;
    }
    .a-card {
        background: #111827;
        border: 1px solid #1f2937;
        border-radius: 8px;
        padding: 16px;
    }
    .a-card h4 {
        margin: 0 0 6px 0;
        font-size: 0.75rem;
        color: #9ca3af;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }
    .a-big {
        margin: 0;
        font-size: 1.5rem;
        font-weight: 700;
        color: #e5e9f0;
    }
    .a-err { color: #f87171; }
    .analytics-charts {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 12px;
        margin-bottom: 20px;
    }
    .a-chart {
        width: 100%;
        height: 120px;
        display: block;
        margin-top: 8px;
    }
    .a-chart rect:hover { opacity: 0.8; }
    .a-empty {
        color: #6b7280;
        font-style: italic;
        padding: 20px 0;
        text-align: center;
    }
    .num { text-align: right; font-variant-numeric: tabular-nums; }
    @media (max-width: 900px) {
        .analytics-charts { grid-template-columns: 1fr; }
        .analytics-cards { grid-template-columns: repeat(3, 1fr); }
    }
    @media (max-width: 640px) {
        .analytics-cards { grid-template-columns: repeat(2, 1fr); }
    }
    "#
}
