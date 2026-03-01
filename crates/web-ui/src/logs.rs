//! Log viewer: list + detail pages.
//!
//! All HTML is rendered via Askama templates under `templates/logs/`.

use askama::Template;
use assistant_storage::{LogStats, LogStore, RecordedLog};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
};
use serde::Deserialize;

use crate::common::{internal_error, render_template, url_encode};
use crate::AppState;

// -- Query -------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct LogQuery {
    severity: Option<String>,
    target: Option<String>,
    search: Option<String>,
    trace_id: Option<String>,
}

// -- View models -------------------------------------------------------------

/// A severity radio option in the sidebar.
struct SeverityOptionView {
    value: &'static str,
    label: &'static str,
    count: i64,
    checked: bool,
}

/// A target facet link in the sidebar.
struct TargetFacetView {
    name: String,
    url: String,
    active: bool,
}

/// A trace ID link (short + full).
struct TraceLinkView {
    short_id: String,
    full_id: String,
}

/// A row in the log list table.
struct LogRowView {
    id: String,
    severity_class: &'static str,
    severity_text: String,
    timestamp: String,
    target: Option<String>,
    body_preview: String,
    trace_link: Option<TraceLinkView>,
}

// -- Templates ---------------------------------------------------------------

/// Full log-list page (extends base.html).
#[derive(Template)]
#[template(path = "logs/page.html")]
struct LogsPageTemplate {
    app_css_url: &'static str,
    active_page: &'static str,
    // Sidebar
    severity_options: Vec<SeverityOptionView>,
    target_facets: Vec<TargetFacetView>,
    selected_severity: Option<String>,
    selected_target: Option<String>,
    search_query: Option<String>,
    search_value: String,
    trace_id_filter: Option<String>,
    shown_count: usize,
    // Content
    logs: Vec<LogRowView>,
}

/// Log detail page (extends base.html).
#[derive(Template)]
#[template(path = "logs/detail.html")]
struct LogDetailTemplate {
    app_css_url: &'static str,
    active_page: &'static str,
    short_id: String,
    log_id: String,
    severity_class: &'static str,
    severity_text: String,
    severity_number: i32,
    timestamp: String,
    target: String,
    trace_link: Option<TraceLinkView>,
    span_id: Option<String>,
    body: String,
    attrs_json: String,
}

// -- Router ------------------------------------------------------------------

/// Returns the sub-router for log-related routes.
pub(crate) fn logs_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/logs", axum::routing::get(show_logs))
        .route("/log/{log_id}", axum::routing::get(show_log_detail))
}

// -- Handlers ----------------------------------------------------------------

async fn show_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> Result<Response, (StatusCode, String)> {
    let store = LogStore::new(state.pool.clone());

    let severity_label = query
        .severity
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let min_severity = severity_label.and_then(severity_label_to_min);

    let target_filter = query
        .target
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let trace_id = query
        .trace_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let logs = store
        .list_recent(
            state.log_limit,
            min_severity,
            target_filter,
            search,
            trace_id,
        )
        .await
        .map_err(internal_error)?;

    let stats = store.stats().await.map_err(internal_error)?;
    let targets = store.list_targets().await.map_err(internal_error)?;

    let severity_options = build_severity_options(&stats, severity_label);
    let target_facets =
        build_target_facets(&targets, target_filter, severity_label, search, trace_id);
    let log_rows: Vec<LogRowView> = logs.iter().map(log_to_row_view).collect();
    let shown_count = log_rows.len();

    let tmpl = LogsPageTemplate {
        app_css_url: crate::static_assets::app_css_url(),
        active_page: "logs",
        severity_options,
        target_facets,
        selected_severity: severity_label.map(|s| s.to_string()),
        selected_target: target_filter.map(|s| s.to_string()),
        search_query: search.map(|s| s.to_string()),
        search_value: search.unwrap_or("").to_string(),
        trace_id_filter: trace_id.map(|s| s.to_string()),
        shown_count,
        logs: log_rows,
    };

    Ok(render_template(tmpl))
}

async fn show_log_detail(
    State(state): State<AppState>,
    Path(log_id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let store = LogStore::new(state.pool.clone());
    let log = store
        .get_log(&log_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Log {} not found", log_id)))?;

    let short_id = if log.id.len() >= 8 {
        log.id[..8].to_string()
    } else {
        log.id.clone()
    };

    let severity_class = severity_number_to_class(log.severity_number.unwrap_or(0));
    let severity_text = log.severity_text.as_deref().unwrap_or("???").to_string();
    let timestamp = log
        .timestamp
        .format("%Y-%m-%d %H:%M:%S%.3f UTC")
        .to_string();
    let target = log.target.as_deref().unwrap_or("\u{2014}").to_string();

    let trace_link = trace_id_to_link(log.trace_id.as_ref());

    let span_id = log
        .span_id
        .as_deref()
        .filter(|s| !s.is_empty() && *s != NULL_SPAN_ID)
        .map(|s| s.to_string());

    let body = log.body.as_deref().unwrap_or("").to_string();
    let attrs_json =
        serde_json::to_string_pretty(&log.attributes).unwrap_or_else(|_| "{}".to_string());

    let tmpl = LogDetailTemplate {
        app_css_url: crate::static_assets::app_css_url(),
        active_page: "logs",
        short_id,
        log_id: log.id.clone(),
        severity_class,
        severity_text,
        severity_number: log.severity_number.unwrap_or(0),
        timestamp,
        target,
        trace_link,
        span_id,
        body,
        attrs_json,
    };

    Ok(render_template(tmpl))
}

// -- Helpers -----------------------------------------------------------------

/// Null trace ID (32 zeros) used by OpenTelemetry for unset trace context.
const NULL_TRACE_ID: &str = "00000000000000000000000000000000";
/// Null span ID (16 zeros) used by OpenTelemetry for unset span context.
const NULL_SPAN_ID: &str = "0000000000000000";

/// Build a [`TraceLinkView`] from an optional trace ID, filtering out empty and
/// null OpenTelemetry IDs.
fn trace_id_to_link(trace_id: Option<&String>) -> Option<TraceLinkView> {
    trace_id
        .filter(|t| !t.is_empty() && *t != NULL_TRACE_ID)
        .map(|t| TraceLinkView {
            short_id: if t.len() >= 8 {
                t[..8].to_string()
            } else {
                t.clone()
            },
            full_id: t.clone(),
        })
}

// -- View model builders -----------------------------------------------------

fn log_to_row_view(log: &RecordedLog) -> LogRowView {
    let severity_class = severity_number_to_class(log.severity_number.unwrap_or(0));
    let severity_text = log.severity_text.as_deref().unwrap_or("???").to_string();
    let body_preview = log
        .body
        .as_deref()
        .unwrap_or("")
        .chars()
        .take(120)
        .collect::<String>();
    let target = log.target.clone();
    let timestamp = log.timestamp.format("%H:%M:%S%.3f").to_string();

    let trace_link = trace_id_to_link(log.trace_id.as_ref());

    LogRowView {
        id: log.id.clone(),
        severity_class,
        severity_text,
        timestamp,
        target,
        body_preview,
        trace_link,
    }
}

fn build_severity_options(stats: &LogStats, selected: Option<&str>) -> Vec<SeverityOptionView> {
    let options: &[(&str, &str, i64)] = &[
        ("", "All", stats.total),
        ("trace", "Trace", stats.trace_count),
        ("debug", "Debug", stats.debug_count),
        ("info", "Info", stats.info_count),
        ("warn", "Warn", stats.warn_count),
        ("error", "Error", stats.error_count),
        ("fatal", "Fatal", stats.fatal_count),
    ];

    options
        .iter()
        .map(|(value, label, count)| {
            let checked = if value.is_empty() {
                selected.is_none()
            } else {
                selected
                    .map(|s| s.eq_ignore_ascii_case(value))
                    .unwrap_or(false)
            };
            SeverityOptionView {
                value,
                label,
                count: *count,
                checked,
            }
        })
        .collect()
}

fn build_target_facets(
    targets: &[String],
    selected_target: Option<&str>,
    severity: Option<&str>,
    search: Option<&str>,
    trace_id: Option<&str>,
) -> Vec<TargetFacetView> {
    targets
        .iter()
        .take(15)
        .map(|target| {
            let active = selected_target
                .map(|t| t.eq_ignore_ascii_case(target))
                .unwrap_or(false);
            let mut params = vec![format!("target={}", url_encode(target))];
            if let Some(sev) = severity.filter(|s| !s.is_empty()) {
                params.push(format!("severity={}", url_encode(sev)));
            }
            if let Some(s) = search {
                params.push(format!("search={}", url_encode(s)));
            }
            if let Some(tid) = trace_id {
                params.push(format!("trace_id={}", url_encode(tid)));
            }
            let url = format!("/logs?{}", params.join("&"));
            TargetFacetView {
                name: target.clone(),
                url,
                active,
            }
        })
        .collect()
}

// -- Pure helpers ------------------------------------------------------------

fn severity_number_to_class(num: i32) -> &'static str {
    match num {
        1..=4 => "sev-trace",
        5..=8 => "sev-debug",
        9..=12 => "sev-info",
        13..=16 => "sev-warn",
        17..=20 => "sev-error",
        21.. => "sev-fatal",
        _ => "sev-unknown",
    }
}

fn severity_label_to_min(label: &str) -> Option<i32> {
    match label.to_lowercase().as_str() {
        "trace" => Some(1),
        "debug" => Some(5),
        "info" => Some(9),
        "warn" | "warning" => Some(13),
        "error" => Some(17),
        "fatal" => Some(21),
        _ => None,
    }
}
