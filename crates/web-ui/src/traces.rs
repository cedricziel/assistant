//! Trace-related handlers and rendering functions.
//!
//! All HTML is rendered via Askama templates under `templates/traces/`.

use std::collections::{HashMap, HashSet};

use askama::Template;
use assistant_storage::{RecordedSpan, TraceStore, TraceSummary};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::common::{format_duration, internal_error, render_template, url_encode};
use crate::AppState;

// -- Query -------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct TraceQuery {
    skill: Option<String>,
    status: Option<String>,
    conversation: Option<String>,
    min_duration_ms: Option<i64>,
}

// -- View models -------------------------------------------------------------

/// A skill/service facet in the sidebar.
struct SkillFacetView {
    name: String,
    count: usize,
    url: String,
    active: bool,
}

/// A status radio option in the sidebar.
struct StatusOptionView {
    value: &'static str,
    label: &'static str,
    checked: bool,
}

/// A row in the trace list table.
struct TraceRowView {
    trace_url: String,
    short_id: String,
    service: Option<String>,
    duration: String,
    span_count: i64,
    is_error: bool,
}

/// A span row in the waterfall chart.
struct WaterfallRowView {
    span_id: String,
    attrs_json: String,
    padding_left: usize,
    label: String,
    duration: String,
    has_error: bool,
    bar_class: &'static str,
    offset_pct: String,
    width_pct: String,
}

// -- Templates ---------------------------------------------------------------

/// Full trace-list page (extends base.html).
#[derive(Template)]
#[template(path = "traces/page.html")]
struct TracesPageTemplate {
    active_page: &'static str,
    // Sidebar
    skill_facets: Vec<SkillFacetView>,
    status_options: Vec<StatusOptionView>,
    selected_skill: Option<String>,
    selected_status: Option<String>,
    selected_conversation: Option<String>,
    min_duration_ms: Option<i64>,
    min_duration_str: String,
    // Content
    traces: Vec<TraceRowView>,
    filtered_count: usize,
    total_count: usize,
}

/// Trace detail page with waterfall chart (extends base.html).
#[derive(Template)]
#[template(path = "traces/detail.html")]
struct TraceDetailTemplate {
    active_page: &'static str,
    short_id: String,
    first_service: Option<String>,
    last_service: Option<String>,
    duration: String,
    has_errors: bool,
    waterfall_rows: Vec<WaterfallRowView>,
    time_ticks: Vec<String>,
}

// -- Router ------------------------------------------------------------------

/// Returns the sub-router for trace-related routes.
pub(crate) fn traces_router() -> Router<AppState> {
    Router::new()
        .route("/traces", get(show_dashboard))
        .route("/trace/{trace_id}", get(show_trace_detail))
}

// -- Handlers ----------------------------------------------------------------

async fn show_dashboard(
    State(state): State<AppState>,
    Query(query): Query<TraceQuery>,
) -> Result<Response, (StatusCode, String)> {
    let store = TraceStore::new(state.pool.clone());

    let skill_filter = query
        .skill
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let status_value = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let status_filter = status_value.as_ref().map(|s| s.to_lowercase());

    let conversation_value = query
        .conversation
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let min_duration_ms = query.min_duration_ms;

    let all_traces = store
        .list_recent_traces(state.trace_limit, None)
        .await
        .map_err(internal_error)?;
    let total_count = all_traces.len();
    let mut traces = all_traces.clone();

    if let Some(filter) = skill_filter {
        traces.retain(|trace| {
            trace
                .tool_names
                .iter()
                .any(|tool| tool.eq_ignore_ascii_case(filter))
        });
    }

    if let Some(filter) = status_filter.as_deref() {
        traces.retain(|trace| match filter {
            "ok" | "success" => trace.error_count == 0,
            "error" | "fail" => trace.error_count > 0,
            _ => true,
        });
    }

    if let Some(filter) = conversation_value.as_ref() {
        traces.retain(|trace| {
            trace
                .conversation_id
                .as_ref()
                .map(|id| id.to_string() == *filter)
                .unwrap_or(false)
        });
    }

    if let Some(min_ms) = min_duration_ms {
        traces.retain(|trace| {
            let elapsed = (trace.end_time - trace.start_time).num_milliseconds();
            elapsed >= min_ms
        });
    }

    let filtered_count = traces.len();

    // -- Build view models --

    let skill_facets = build_skill_facet_views(
        &all_traces,
        skill_filter,
        status_value.as_deref(),
        conversation_value.as_deref(),
        min_duration_ms,
    );

    let status_options = build_status_options(status_value.as_deref());

    let trace_rows: Vec<TraceRowView> = traces.iter().map(trace_to_row_view).collect();

    let tmpl = TracesPageTemplate {
        active_page: "traces",
        skill_facets,
        status_options,
        selected_skill: skill_filter.map(|s| s.to_string()),
        selected_status: status_value,
        selected_conversation: conversation_value,
        min_duration_ms,
        min_duration_str: min_duration_ms.map(|d| d.to_string()).unwrap_or_default(),
        traces: trace_rows,
        filtered_count,
        total_count,
    };

    Ok(render_template(tmpl))
}

async fn show_trace_detail(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let store = TraceStore::new(state.pool.clone());
    let spans = store.get_trace(&trace_id).await.map_err(internal_error)?;
    if spans.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Trace {} not found", trace_id),
        ));
    }

    let start = spans.first().map(|s| s.start_time).unwrap_or_else(Utc::now);
    let end = spans
        .last()
        .map(|s| s.end_time)
        .unwrap_or(start + chrono::Duration::milliseconds(1));
    let total_duration_ms = (end - start).num_milliseconds();
    let distinct_tools = collect_distinct_tools(&spans);
    let error_spans = spans
        .iter()
        .filter(|span| span.error.is_some() || span.tool_status.as_deref() == Some("error"))
        .count();

    let short_id = if trace_id.len() >= 8 {
        trace_id[..8].to_string()
    } else {
        trace_id.clone()
    };

    let first_service = distinct_tools.first().cloned();
    let last_service = if distinct_tools.len() > 1 {
        distinct_tools.last().cloned()
    } else {
        None
    };

    let time_ticks = build_time_ticks(total_duration_ms);
    let waterfall_rows = build_waterfall_rows(start, end, &spans);

    let tmpl = TraceDetailTemplate {
        active_page: "traces",
        short_id,
        first_service,
        last_service,
        duration: format_duration(total_duration_ms),
        has_errors: error_spans > 0,
        waterfall_rows,
        time_ticks,
    };

    Ok(render_template(tmpl))
}

// -- View model builders -----------------------------------------------------

fn trace_to_row_view(trace: &TraceSummary) -> TraceRowView {
    let elapsed_ms = (trace.end_time - trace.start_time).num_milliseconds();
    let short_id = if trace.trace_id.len() >= 8 {
        trace.trace_id[..8].to_string()
    } else {
        trace.trace_id.clone()
    };
    let service = trace.tool_names.iter().find(|t| !t.is_empty()).cloned();

    TraceRowView {
        trace_url: trace.trace_id.clone(),
        short_id,
        service,
        duration: format_duration(elapsed_ms),
        span_count: trace.span_count,
        is_error: trace.error_count > 0,
    }
}

fn build_skill_facet_views(
    traces: &[TraceSummary],
    selected_skill: Option<&str>,
    selected_status: Option<&str>,
    conversation: Option<&str>,
    min_duration_ms: Option<i64>,
) -> Vec<SkillFacetView> {
    let facets = build_skill_facets(traces);
    facets
        .into_iter()
        .take(12)
        .map(|(name, count)| {
            let active = selected_skill
                .map(|s| s.eq_ignore_ascii_case(&name))
                .unwrap_or(false);
            let url = build_query_url(
                Some(name.as_str()),
                selected_status,
                conversation,
                min_duration_ms,
            );
            SkillFacetView {
                name,
                count,
                url,
                active,
            }
        })
        .collect()
}

fn build_status_options(selected: Option<&str>) -> Vec<StatusOptionView> {
    let options: &[(&str, &str)] = &[("", "All"), ("ok", "Success"), ("error", "Error")];
    options
        .iter()
        .map(|(value, label)| {
            let checked = if value.is_empty() {
                selected.is_none()
            } else {
                selected
                    .map(|s| s.eq_ignore_ascii_case(value))
                    .unwrap_or(false)
            };
            StatusOptionView {
                value,
                label,
                checked,
            }
        })
        .collect()
}

fn build_time_ticks(total_ms: i64) -> Vec<String> {
    (0usize..=4)
        .map(|i| {
            let ms = (total_ms as f64 * i as f64 / 4.0) as i64;
            format_duration(ms)
        })
        .collect()
}

// -- Waterfall ---------------------------------------------------------------

fn build_waterfall_rows(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    spans: &[RecordedSpan],
) -> Vec<WaterfallRowView> {
    if spans.is_empty() {
        return Vec::new();
    }
    let total_ms = (end - start).num_milliseconds().max(1);

    // Build parent/child map
    let ids: HashSet<String> = spans.iter().map(|s| s.span_id.clone()).collect();
    let mut children: HashMap<String, Vec<usize>> = HashMap::new();
    let mut roots: Vec<usize> = Vec::new();

    for (idx, span) in spans.iter().enumerate() {
        if let Some(parent) = &span.parent_span_id {
            if ids.contains(parent) {
                children.entry(parent.clone()).or_default().push(idx);
            } else {
                roots.push(idx);
            }
        } else {
            roots.push(idx);
        }
    }
    if roots.is_empty() {
        roots.push(0);
    }
    roots.sort_by_key(|idx| spans[*idx].start_time);
    for child in children.values_mut() {
        child.sort_by_key(|idx| spans[*idx].start_time);
    }

    // DFS ordering
    let mut ordered: Vec<(usize, usize)> = Vec::new();
    collect_dfs(&roots, spans, &children, 0, &mut ordered);

    ordered
        .iter()
        .map(|(idx, depth)| {
            let span = &spans[*idx];
            let label = span.tool_name.as_deref().unwrap_or(&span.name).to_string();
            let offset_ms = (span.start_time - start).num_milliseconds().max(0);
            let offset_pct = ((offset_ms as f64 / total_ms as f64) * 100.0).clamp(0.0, 100.0);
            let width_pct = ((span.duration_ms.max(1) as f64 / total_ms as f64) * 100.0)
                .clamp(0.5, 100.0 - offset_pct);
            let bar_class = match span.tool_status.as_deref() {
                Some("error") => "wf-bar error",
                Some("ok") => "wf-bar ok",
                _ => "wf-bar",
            };
            let has_error = span.error.is_some() || span.tool_status.as_deref() == Some("error");

            // JSON blob for the click-handler.  Askama auto-escapes {{ }}
            // expressions, converting " to &quot; etc.  The browser decodes
            // HTML entities before getAttribute returns, so JSON.parse works.
            let mut m = serde_json::json!({
                "span_id": span.span_id,
                "name": span.name,
                "duration_ms": span.duration_ms,
                "status": span.tool_status.as_deref().unwrap_or("unknown"),
                "attributes": span.attributes,
            });
            if let Some(t) = &span.tool_name {
                m["tool_name"] = serde_json::Value::String(t.clone());
            }
            if let Some(obs) = &span.observation {
                m["observation"] = serde_json::Value::String(obs.clone());
            }
            if let Some(err) = &span.error {
                m["error"] = serde_json::Value::String(err.clone());
            }
            let attrs_json = serde_json::to_string(&m).unwrap_or_else(|_| "{}".to_string());

            WaterfallRowView {
                span_id: span.span_id.clone(),
                attrs_json,
                padding_left: depth * 16,
                label,
                duration: format_duration(span.duration_ms),
                has_error,
                bar_class,
                offset_pct: format!("{offset_pct:.2}"),
                width_pct: format!("{width_pct:.2}"),
            }
        })
        .collect()
}

/// DFS traversal to produce ordered `(span_index, depth)` pairs.
fn collect_dfs(
    indexes: &[usize],
    spans: &[RecordedSpan],
    children: &HashMap<String, Vec<usize>>,
    depth: usize,
    out: &mut Vec<(usize, usize)>,
) {
    for &idx in indexes {
        out.push((idx, depth));
        if let Some(child_indexes) = children.get(&spans[idx].span_id) {
            collect_dfs(child_indexes, spans, children, depth + 1, out);
        }
    }
}

// -- Pure helpers (no HTML) --------------------------------------------------

fn build_skill_facets(traces: &[TraceSummary]) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, (String, usize)> = HashMap::new();
    for trace in traces {
        let mut seen: HashSet<String> = HashSet::new();
        for tool in &trace.tool_names {
            let trimmed = tool.trim();
            if trimmed.is_empty() {
                continue;
            }
            let key = trimmed.to_lowercase();
            if !seen.insert(key.clone()) {
                continue;
            }
            counts
                .entry(key)
                .and_modify(|(_, count)| *count += 1)
                .or_insert((trimmed.to_string(), 1));
        }
    }

    let mut facets: Vec<(String, usize)> = counts
        .values()
        .map(|(label, count)| (label.clone(), *count))
        .collect();
    facets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    facets
}

fn build_query_url(
    skill: Option<&str>,
    status: Option<&str>,
    conversation: Option<&str>,
    min_duration_ms: Option<i64>,
) -> String {
    let mut parts = Vec::new();
    if let Some(value) = skill.filter(|s| !s.is_empty()) {
        parts.push(format!("skill={}", url_encode(value)));
    }
    if let Some(value) = status.filter(|s| !s.is_empty()) {
        parts.push(format!("status={}", url_encode(value)));
    }
    if let Some(value) = conversation.filter(|s| !s.is_empty()) {
        parts.push(format!("conversation={}", url_encode(value)));
    }
    if let Some(value) = min_duration_ms {
        parts.push(format!("min_duration_ms={value}"));
    }
    if parts.is_empty() {
        "/traces".to_string()
    } else {
        format!("/traces?{}", parts.join("&"))
    }
}

fn collect_distinct_tools(spans: &[RecordedSpan]) -> Vec<String> {
    let mut tools: Vec<String> = Vec::new();
    for span in spans {
        if let Some(tool) = &span.tool_name {
            if !tools
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(tool))
            {
                tools.push(tool.clone());
            }
        }
    }
    tools
}
