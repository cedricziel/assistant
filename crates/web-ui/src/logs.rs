//! Log viewer: list + detail pages.

use assistant_storage::{LogStats, LogStore, RecordedLog};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
};
use serde::Deserialize;

use crate::common::{default_css, html_escape, internal_error, url_encode};
use crate::legacy;
use crate::AppState;

#[derive(Debug, Default, Deserialize)]
struct LogQuery {
    severity: Option<String>,
    target: Option<String>,
    search: Option<String>,
    trace_id: Option<String>,
}

/// Returns the sub-router for log-related routes.
pub(crate) fn logs_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/logs", axum::routing::get(show_logs))
        .route("/log/{log_id}", axum::routing::get(show_log_detail))
}

async fn show_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
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

    let sidebar = render_logs_sidebar(
        &stats,
        &targets,
        severity_label,
        target_filter,
        search,
        trace_id,
        logs.len(),
    );
    let log_panel = render_log_list(&logs);

    let content_html = format!(
        "<div class=\"layout\">\
         <aside class=\"sidebar\">{sidebar}</aside>\
         <main class=\"main\">{panel}</main>\
         </div>",
        sidebar = sidebar,
        panel = log_panel,
    );

    let body = legacy::render_page(
        "logs",
        "Logs",
        "Observability",
        "Logs",
        default_css(),
        &content_html,
        "",
    );

    Ok(Html(body))
}

async fn show_log_detail(
    State(state): State<AppState>,
    Path(log_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let store = LogStore::new(state.pool.clone());
    let log = store
        .get_log(&log_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Log {} not found", log_id)))?;

    let detail = render_log_detail_page(&log);
    let content_html = format!("<div class=\"page\">{detail}</div>", detail = detail);

    let short_id = if log.id.len() >= 8 {
        &log.id[..8]
    } else {
        &log.id
    };
    let body = legacy::render_page(
        "logs",
        &format!("Log {}", html_escape(&log.id)),
        "Observability",
        &format!("Log {short_id}..."),
        default_css(),
        &content_html,
        "",
    );

    Ok(Html(body))
}

fn render_logs_sidebar(
    stats: &LogStats,
    targets: &[String],
    selected_severity: Option<&str>,
    selected_target: Option<&str>,
    search: Option<&str>,
    trace_id: Option<&str>,
    shown: usize,
) -> String {
    // Severity facets
    let severity_options: &[(&str, &str, i64)] = &[
        ("", "All", stats.total),
        ("trace", "Trace", stats.trace_count),
        ("debug", "Debug", stats.debug_count),
        ("info", "Info", stats.info_count),
        ("warn", "Warn", stats.warn_count),
        ("error", "Error", stats.error_count),
        ("fatal", "Fatal", stats.fatal_count),
    ];

    let mut hidden_fields = String::new();
    if let Some(t) = selected_target {
        hidden_fields.push_str(&format!(
            "<input type=\"hidden\" name=\"target\" value=\"{}\">",
            html_escape(t)
        ));
    }
    if let Some(s) = search {
        hidden_fields.push_str(&format!(
            "<input type=\"hidden\" name=\"search\" value=\"{}\">",
            html_escape(s)
        ));
    }
    if let Some(tid) = trace_id {
        hidden_fields.push_str(&format!(
            "<input type=\"hidden\" name=\"trace_id\" value=\"{}\">",
            html_escape(tid)
        ));
    }

    let mut severity_radios = String::new();
    for (value, label, count) in severity_options {
        let checked = if value.is_empty() {
            selected_severity.is_none()
        } else {
            selected_severity
                .map(|s| s.eq_ignore_ascii_case(value))
                .unwrap_or(false)
        };
        severity_radios.push_str(&format!(
            "<label class=\"radio-label\"><input type=\"radio\" name=\"severity\" value=\"{value}\"\
             onchange=\"this.form.submit()\"{checked}> {label} <em class=\"muted\">({count})</em></label>",
            value = value,
            checked = if checked { " checked" } else { "" },
            label = label,
            count = count,
        ));
    }
    let severity_form = format!(
        "<form method=\"get\" action=\"/logs\">{hidden}{radios}</form>",
        hidden = hidden_fields,
        radios = severity_radios,
    );

    // Target facets
    let mut target_items = String::new();
    for target in targets.iter().take(15) {
        let active = selected_target
            .map(|t| t.eq_ignore_ascii_case(target))
            .unwrap_or(false);
        let mut params = vec![format!("target={}", url_encode(target))];
        if let Some(sev) = selected_severity.filter(|s| !s.is_empty()) {
            params.push(format!("severity={}", url_encode(sev)));
        }
        if let Some(s) = search {
            params.push(format!("search={}", url_encode(s)));
        }
        if let Some(tid) = trace_id {
            params.push(format!("trace_id={}", url_encode(tid)));
        }
        let url = format!("/logs?{}", params.join("&"));
        target_items.push_str(&format!(
            "<li><a class=\"facet-link{active}\" href=\"{url}\">\
             <span>{label}</span></a></li>",
            active = if active { " active" } else { "" },
            url = url,
            label = html_escape(target),
        ));
    }
    if target_items.is_empty() {
        target_items.push_str("<li class=\"muted\">No targets yet</li>");
    }

    // Search form
    let mut search_hidden = String::new();
    if let Some(sev) = selected_severity.filter(|s| !s.is_empty()) {
        search_hidden.push_str(&format!(
            "<input type=\"hidden\" name=\"severity\" value=\"{}\">",
            html_escape(sev)
        ));
    }
    if let Some(t) = selected_target {
        search_hidden.push_str(&format!(
            "<input type=\"hidden\" name=\"target\" value=\"{}\">",
            html_escape(t)
        ));
    }
    if let Some(tid) = trace_id {
        search_hidden.push_str(&format!(
            "<input type=\"hidden\" name=\"trace_id\" value=\"{}\">",
            html_escape(tid)
        ));
    }
    let search_val = search.unwrap_or("");
    let search_form = format!(
        "<form method=\"get\" action=\"/logs\" class=\"min-dur-row\">{hidden}\
         <input type=\"text\" name=\"search\" value=\"{val}\" placeholder=\"Search body...\">\
         <button type=\"submit\">Go</button></form>",
        hidden = search_hidden,
        val = html_escape(search_val),
    );

    format!(
        "<div class=\"sidebar-inner\">\
         <div class=\"brand\"><p>assistant</p><h2>Logs</h2></div>\
         <div class=\"facet-group\"><h3>Severity</h3>{severity_form}</div>\
         <div class=\"facet-group\"><h3>Target</h3><ul>{targets}</ul></div>\
         <div class=\"facet-group\"><h3>Search</h3>{search_form}</div>\
         <div class=\"facet-footer\">\
           <span class=\"trace-count\">Showing {shown} log records</span>\
           <a href=\"/logs\">Reset filters</a>\
         </div>\
         </div>",
        severity_form = severity_form,
        targets = target_items,
        search_form = search_form,
        shown = shown,
    )
}

fn render_log_list(logs: &[RecordedLog]) -> String {
    if logs.is_empty() {
        return "<div class=\"panel trace-panel\"><div class=\"panel-head\"><div>\
                <h2>Log Records</h2><p>No logs match this query yet.</p>\
                </div></div><p class=\"empty\">No rows to display.</p></div>"
            .to_string();
    }

    let mut rows = String::new();
    for log in logs {
        let severity_class = match log.severity_number.unwrap_or(0) {
            1..=4 => "sev-trace",
            5..=8 => "sev-debug",
            9..=12 => "sev-info",
            13..=16 => "sev-warn",
            17..=20 => "sev-error",
            21.. => "sev-fatal",
            _ => "sev-unknown",
        };
        let severity_text = log.severity_text.as_deref().unwrap_or("???");
        let body_preview = log
            .body
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(120)
            .collect::<String>();
        let target = log
            .target
            .as_deref()
            .map(html_escape)
            .unwrap_or_else(|| "<span class=\"muted\">&mdash;</span>".to_string());
        let timestamp = log.timestamp.format("%H:%M:%S%.3f").to_string();
        let trace_link = log
            .trace_id
            .as_ref()
            .filter(|t| !t.is_empty() && *t != "00000000000000000000000000000000")
            .map(|t| {
                let short = if t.len() >= 8 { &t[..8] } else { t };
                format!(
                    "<a href=\"/trace/{full}\" class=\"trace-id\">{short}&hellip;</a>",
                    full = html_escape(t),
                    short = html_escape(short),
                )
            })
            .unwrap_or_else(|| "<span class=\"muted\">&mdash;</span>".to_string());

        let log_url = html_escape(&log.id);
        rows.push_str(&format!(
            "<tr onclick=\"window.location='/log/{log_url}'\">\
             <td><span class=\"{sev_class}\">{sev}</span></td>\
             <td class=\"log-ts\">{ts}</td>\
             <td>{target}</td>\
             <td class=\"log-body\">{body}</td>\
             <td>{trace}</td>\
             </tr>",
            log_url = log_url,
            sev_class = severity_class,
            sev = html_escape(severity_text),
            ts = html_escape(&timestamp),
            target = target,
            body = html_escape(&body_preview),
            trace = trace_link,
        ));
    }

    format!(
        "<div class=\"panel trace-panel\">\
         <div class=\"panel-head\"><div><h2>Log Records</h2></div>\
         <span class=\"pill\">{count}</span></div>\
         <table class=\"trace-table log-table\">\
            <thead><tr>\
              <th>Severity</th>\
              <th>Time</th>\
              <th>Target</th>\
              <th>Body</th>\
              <th>Trace</th>\
            </tr></thead>\
            <tbody>{rows}</tbody>\
         </table>\
         </div>",
        count = logs.len(),
        rows = rows,
    )
}

fn render_log_detail_page(log: &RecordedLog) -> String {
    let severity_class = match log.severity_number.unwrap_or(0) {
        1..=4 => "sev-trace",
        5..=8 => "sev-debug",
        9..=12 => "sev-info",
        13..=16 => "sev-warn",
        17..=20 => "sev-error",
        21.. => "sev-fatal",
        _ => "sev-unknown",
    };
    let severity_text = log.severity_text.as_deref().unwrap_or("???");
    let timestamp = log
        .timestamp
        .format("%Y-%m-%d %H:%M:%S%.3f UTC")
        .to_string();
    let body = log.body.as_deref().unwrap_or("");
    let target = log.target.as_deref().unwrap_or("\u{2014}");

    let trace_link = log
        .trace_id
        .as_ref()
        .filter(|t| !t.is_empty() && *t != "00000000000000000000000000000000")
        .map(|t| {
            format!(
                "<a href=\"/trace/{full}\">{full}</a>",
                full = html_escape(t),
            )
        })
        .unwrap_or_else(|| "&mdash;".to_string());

    let span_id = log
        .span_id
        .as_deref()
        .filter(|s| !s.is_empty() && *s != "0000000000000000")
        .map(html_escape)
        .unwrap_or_else(|| "&mdash;".to_string());

    let attrs_json =
        serde_json::to_string_pretty(&log.attributes).unwrap_or_else(|_| "{}".to_string());

    format!(
        "<div class=\"trace-detail\">\
         <div class=\"trace-header-bar\">\
           <a class=\"hdr-back\" href=\"/logs\">&larr; Back to Logs</a>\
           <span class=\"hdr-sep\">|</span>\
           <span class=\"{sev_class}\">{sev}</span>\
           <span class=\"hdr-sep\">|</span>\
           <span class=\"hdr-dur\">{ts}</span>\
         </div>\
         <div class=\"log-detail-grid\">\
           <div class=\"log-meta\">\
             <table class=\"attr-table\">\
               <tr><td class=\"attr-k\">ID</td><td class=\"attr-v\">{id}</td></tr>\
               <tr><td class=\"attr-k\">Severity</td><td class=\"attr-v\"><span class=\"{sev_class}\">{sev}</span> ({sev_num})</td></tr>\
               <tr><td class=\"attr-k\">Timestamp</td><td class=\"attr-v\">{ts}</td></tr>\
               <tr><td class=\"attr-k\">Target</td><td class=\"attr-v\">{target}</td></tr>\
               <tr><td class=\"attr-k\">Trace ID</td><td class=\"attr-v\">{trace}</td></tr>\
               <tr><td class=\"attr-k\">Span ID</td><td class=\"attr-v\">{span}</td></tr>\
             </table>\
           </div>\
           <div class=\"log-body-section\">\
             <h3>Body</h3>\
             <pre class=\"log-body-pre\">{body}</pre>\
           </div>\
           <div class=\"log-attrs-section\">\
             <h3>Attributes</h3>\
             <pre class=\"log-body-pre\">{attrs}</pre>\
           </div>\
         </div>\
         </div>",
        sev_class = severity_class,
        sev = html_escape(severity_text),
        ts = html_escape(&timestamp),
        id = html_escape(&log.id),
        sev_num = log.severity_number.unwrap_or(0),
        target = html_escape(target),
        trace = trace_link,
        span = span_id,
        body = html_escape(body),
        attrs = html_escape(&attrs_json),
    )
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
