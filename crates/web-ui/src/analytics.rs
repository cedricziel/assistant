//! Analytics dashboard page.
//!
//! All HTML is rendered via the Askama template at `templates/analytics/page.html`.

use askama::Template;
use assistant_storage::MetricsStore;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Response,
};
use serde::Deserialize;

use crate::common::{internal_error, render_template, StaticUrls};
use crate::AppState;

// -- Query -------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct AnalyticsQuery {
    /// Time window in hours (defaults to 24).
    window: Option<i64>,
}

// -- View models -------------------------------------------------------------

/// A time-window option in the sidebar.
#[derive(Debug)]
struct WindowOptionView {
    hours: i64,
    label: &'static str,
    active: bool,
}

/// A single bar in an SVG bar chart.
#[derive(Debug)]
struct BarView {
    x: usize,
    y: String,
    w: usize,
    h: String,
    label: String,
    value: String,
}

/// An SVG bar chart panel.
#[derive(Debug)]
struct ChartView {
    title: &'static str,
    color: &'static str,
    chart_w: usize,
    chart_h: usize,
    bars: Vec<BarView>,
}

/// A row in the model comparison table.
#[derive(Debug)]
struct ModelRowView {
    model: String,
    input_tokens: String,
    output_tokens: String,
    total_tokens: String,
    request_count: i64,
}

/// A row in the tool usage table.
#[derive(Debug)]
struct ToolRowView {
    tool_name: String,
    invocations: i64,
}

// -- Templates ---------------------------------------------------------------

/// Full analytics dashboard page (extends base.html).
#[derive(Template)]
#[template(path = "analytics/page.html")]
struct AnalyticsPageTemplate {
    active_page: &'static str,
    // Sidebar
    window_options: Vec<WindowOptionView>,
    // Summary cards
    tokens_in: String,
    tokens_out: String,
    requests: String,
    tool_calls: String,
    avg_duration: String,
    errors: String,
    // Charts
    charts: Vec<ChartView>,
    // Tables
    models: Vec<ModelRowView>,
    tools: Vec<ToolRowView>,
}

impl StaticUrls for AnalyticsPageTemplate {}

// -- Router ------------------------------------------------------------------

/// Returns the sub-router for analytics-related routes.
pub(crate) fn analytics_router() -> axum::Router<AppState> {
    axum::Router::new().route("/analytics", axum::routing::get(show_analytics))
}

// -- Handler -----------------------------------------------------------------

async fn show_analytics(
    State(state): State<AppState>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Response, (StatusCode, String)> {
    let window_hours = query.window.unwrap_or(24);
    let store = MetricsStore::new(state.pool.clone());

    let summary = store.summary(window_hours).await.map_err(internal_error)?;
    let model_data = store
        .model_comparison(window_hours)
        .await
        .map_err(internal_error)?;
    let tool_data = store
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

    // -- Sidebar --
    let window_opts: &[(i64, &str)] = &[(1, "1h"), (6, "6h"), (24, "24h"), (72, "3d"), (168, "7d")];
    let window_options: Vec<WindowOptionView> = window_opts
        .iter()
        .map(|(h, label)| WindowOptionView {
            hours: *h,
            label,
            active: *h == window_hours,
        })
        .collect();

    // -- Charts --
    let token_bars: Vec<(&str, f64)> = token_series
        .iter()
        .map(|p| (p.bucket.as_str(), p.value))
        .collect();
    let request_bars: Vec<(&str, f64)> = request_series
        .iter()
        .map(|p| (p.bucket.as_str(), p.value))
        .collect();
    let charts = vec![
        build_chart("Token Usage Over Time", &token_bars, "#60a5fa"),
        build_chart("Requests Over Time", &request_bars, "#34d399"),
    ];

    // -- Tables --
    let models: Vec<ModelRowView> = model_data
        .iter()
        .map(|m| ModelRowView {
            model: m.model.clone(),
            input_tokens: format_number(m.input_tokens),
            output_tokens: format_number(m.output_tokens),
            total_tokens: format_number(m.input_tokens + m.output_tokens),
            request_count: m.request_count,
        })
        .collect();

    let tools: Vec<ToolRowView> = tool_data
        .iter()
        .map(|t| ToolRowView {
            tool_name: t.tool_name.clone(),
            invocations: t.invocations,
        })
        .collect();

    let tmpl = AnalyticsPageTemplate {
        active_page: "analytics",
        window_options,
        tokens_in: format_number(summary.total_tokens_in),
        tokens_out: format_number(summary.total_tokens_out),
        requests: format_number(summary.total_requests),
        tool_calls: format_number(summary.total_tool_invocations),
        avg_duration: format!("{:.2}s", summary.avg_duration_s),
        errors: summary.error_count.to_string(),
        charts,
        models,
        tools,
    };

    Ok(render_template(tmpl))
}

// -- Chart builder -----------------------------------------------------------

fn build_chart(title: &'static str, data: &[(&str, f64)], color: &'static str) -> ChartView {
    let chart_w = 600;
    let chart_h = 120;
    let bar_gap = 2;
    let n = data.len();

    if n == 0 {
        return ChartView {
            title,
            color,
            chart_w,
            chart_h,
            bars: Vec::new(),
        };
    }

    let max_val = data.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max);
    let bar_w = ((chart_w as f64 - (n as f64 * bar_gap as f64)) / n as f64).max(2.0) as usize;

    let bars = data
        .iter()
        .enumerate()
        .map(|(i, (label, val))| {
            let h = if max_val > 0.0 {
                (val / max_val * chart_h as f64).max(1.0)
            } else {
                1.0
            };
            let x = i * (bar_w + bar_gap);
            let y = chart_h as f64 - h;
            // Labels are ISO timestamps like "2024-01-15 10:30:00";
            // extract "10:30" (chars 11..16) for compact x-axis display.
            let short_label = if label.len() >= 16 {
                label.get(11..16).unwrap_or(label)
            } else {
                label
            };
            BarView {
                x,
                y: format!("{y:.0}"),
                w: bar_w,
                h: format!("{h:.0}"),
                label: short_label.to_string(),
                value: format!("{val:.0}"),
            }
        })
        .collect();

    ChartView {
        title,
        color,
        chart_w,
        chart_h,
        bars,
    }
}

// -- Helpers -----------------------------------------------------------------

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
