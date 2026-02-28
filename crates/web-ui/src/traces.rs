//! Trace-related handlers and rendering functions.

use std::collections::{HashMap, HashSet};

use assistant_storage::{RecordedSpan, TraceStore, TraceSummary};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::common::{default_css, format_duration, html_escape, internal_error, url_encode};
use crate::legacy;
use crate::AppState;

#[derive(Debug, Default, Deserialize)]
struct TraceQuery {
    skill: Option<String>,
    status: Option<String>,
    conversation: Option<String>,
    min_duration_ms: Option<i64>,
}

/// Returns the sub-router for trace-related routes.
pub(crate) fn traces_router() -> Router<AppState> {
    Router::new()
        .route("/traces", get(show_dashboard))
        .route("/trace/{trace_id}", get(show_trace_detail))
}

async fn show_dashboard(
    State(state): State<AppState>,
    Query(query): Query<TraceQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
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
    let skill_facets = build_skill_facets(&all_traces);
    let status_facets = build_status_facets(&all_traces);

    let sidebar = render_sidebar(
        &skill_facets,
        skill_filter,
        &status_facets,
        status_value.as_deref(),
        conversation_value.as_deref(),
        min_duration_ms,
        filtered_count,
        total_count,
    );
    let trace_panel = render_trace_list(&traces, total_count);

    let content_html = format!(
        "<div class=\"layout\">\
         <aside class=\"sidebar\">{sidebar}</aside>\
         <main class=\"main\">{sections}</main>\
         </div>",
        sidebar = sidebar,
        sections = trace_panel,
    );

    let body = legacy::render_page(
        "traces",
        "Traces",
        "Observability",
        "Traces",
        default_css(),
        &content_html,
        "",
    );

    Ok(Html(body))
}

async fn show_trace_detail(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let store = TraceStore::new(state.pool.clone());
    let spans = store.get_trace(&trace_id).await.map_err(internal_error)?;
    if spans.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Trace {} not found", trace_id),
        ));
    }

    let detail_html = render_trace_detail(&trace_id, &spans);
    let content_html = format!("<div class=\"page\">{detail}</div>", detail = detail_html);

    let short_id = if trace_id.len() >= 8 {
        &trace_id[..8]
    } else {
        &trace_id
    };
    let body = legacy::render_page(
        "traces",
        &format!("Trace {}", html_escape(&trace_id)),
        "Observability",
        &format!("Trace {short_id}..."),
        default_css(),
        &content_html,
        &format!("<script>{}</script>", detail_js()),
    );

    Ok(Html(body))
}

#[allow(clippy::too_many_arguments)]
fn render_sidebar(
    skill_facets: &[(String, usize)],
    selected_skill: Option<&str>,
    _status_facets: &[(String, usize)],
    selected_status: Option<&str>,
    conversation: Option<&str>,
    min_duration_ms: Option<i64>,
    filtered_count: usize,
    total_count: usize,
) -> String {
    let mut skill_items = String::new();
    for (skill, count) in skill_facets.iter().take(12) {
        let active = selected_skill
            .map(|s| s.eq_ignore_ascii_case(skill))
            .unwrap_or(false);
        let url = build_query_url(
            Some(skill.as_str()),
            selected_status,
            conversation,
            min_duration_ms,
        );
        skill_items.push_str(&format!(
            "<li><a class=\"facet-link{active}\" href=\"{url}\">\
             <span>{label}</span><em>{count}</em></a></li>",
            active = if active { " active" } else { "" },
            url = url,
            label = html_escape(skill),
            count = count,
        ));
    }
    if skill_items.is_empty() {
        skill_items.push_str("<li class=\"muted\">No skill activity yet</li>");
    }

    // Hidden inputs to preserve other filter params in the status form
    let skill_hidden = selected_skill
        .map(|s| {
            format!(
                "<input type=\"hidden\" name=\"skill\" value=\"{}\">",
                html_escape(s)
            )
        })
        .unwrap_or_default();
    let conv_hidden = conversation
        .map(|c| {
            format!(
                "<input type=\"hidden\" name=\"conversation\" value=\"{}\">",
                html_escape(c)
            )
        })
        .unwrap_or_default();
    let min_dur_hidden = min_duration_ms
        .map(|d| format!("<input type=\"hidden\" name=\"min_duration_ms\" value=\"{d}\">"))
        .unwrap_or_default();

    // Status radio group (auto-submits on change)
    let status_options: &[(&str, &str)] = &[("", "All"), ("ok", "Success"), ("error", "Error")];
    let mut status_radios = String::new();
    for (value, label) in status_options {
        let checked = if value.is_empty() {
            selected_status.is_none()
        } else {
            selected_status
                .map(|s| s.eq_ignore_ascii_case(value))
                .unwrap_or(false)
        };
        status_radios.push_str(&format!(
            "<label class=\"radio-label\"><input type=\"radio\" name=\"status\" value=\"{value}\"\
             onchange=\"this.form.submit()\"{checked}> {label}</label>",
            value = value,
            checked = if checked { " checked" } else { "" },
            label = label,
        ));
    }
    let status_form = format!(
        "<form method=\"get\">{skill}{conv}{min_dur}{radios}</form>",
        skill = skill_hidden,
        conv = conv_hidden,
        min_dur = min_dur_hidden,
        radios = status_radios,
    );

    // Min Duration sub-form — hidden inputs preserve skill, conversation, status
    let skill_hidden2 = selected_skill
        .map(|s| {
            format!(
                "<input type=\"hidden\" name=\"skill\" value=\"{}\">",
                html_escape(s)
            )
        })
        .unwrap_or_default();
    let conv_hidden2 = conversation
        .map(|c| {
            format!(
                "<input type=\"hidden\" name=\"conversation\" value=\"{}\">",
                html_escape(c)
            )
        })
        .unwrap_or_default();
    let status_hidden = selected_status
        .map(|s| {
            format!(
                "<input type=\"hidden\" name=\"status\" value=\"{}\">",
                html_escape(s)
            )
        })
        .unwrap_or_default();
    let min_dur_val = min_duration_ms.map(|d| d.to_string()).unwrap_or_default();
    let min_dur_form = format!(
        "<form method=\"get\" class=\"min-dur-row\">{skill}{conv}{status}\
         <input type=\"number\" name=\"min_duration_ms\" value=\"{val}\" placeholder=\"e.g. 500\" min=\"0\">\
         <button type=\"submit\">Go</button></form>",
        skill = skill_hidden2,
        conv = conv_hidden2,
        status = status_hidden,
        val = html_escape(&min_dur_val),
    );

    format!(
        "<div class=\"sidebar-inner\">\
         <div class=\"brand\"><p>assistant</p><h2>Telemetry</h2></div>\
         <div class=\"facet-group\"><h3>Service</h3><ul>{skills}</ul></div>\
         <div class=\"facet-group\"><h3>Status</h3>{status_form}</div>\
         <div class=\"facet-group\"><h3>Min Duration</h3>{min_dur_form}</div>\
         <div class=\"facet-footer\">\
           <span class=\"trace-count\">Showing {filtered} of {total} traces</span>\
           <a href=\"/traces\">Reset filters</a>\
         </div>\
         </div>",
        skills = skill_items,
        status_form = status_form,
        min_dur_form = min_dur_form,
        filtered = filtered_count,
        total = total_count,
    )
}

fn render_trace_list(traces: &[TraceSummary], total_count: usize) -> String {
    if traces.is_empty() {
        return "<div class=\"panel trace-panel\"><div class=\"panel-head\"><div>\
                <h2>Traces</h2><p>No traces match this query yet. Trigger a skill or soften the filters.</p>\
                </div></div><p class=\"empty\">No rows to display.</p></div>"
            .to_string();
    }

    let mut rows = String::new();
    for trace in traces {
        let elapsed_ms = (trace.end_time - trace.start_time).num_milliseconds();
        let short_id = if trace.trace_id.len() >= 8 {
            &trace.trace_id[..8]
        } else {
            trace.trace_id.as_str()
        };
        let service = trace
            .tool_names
            .iter()
            .find(|t| !t.is_empty())
            .map(|t| html_escape(t))
            .unwrap_or_else(|| "<span class=\"muted\">&mdash;</span>".to_string());
        let duration = format_duration(elapsed_ms);
        let (status_icon, status_class) = if trace.error_count > 0 {
            ("&#x2717;", "status-err")
        } else {
            ("&#x2713;", "status-ok")
        };
        let trace_url = html_escape(&trace.trace_id);
        rows.push_str(&format!(
            "<tr onclick=\"window.location='/trace/{trace_url}'\">\
             <td><span class=\"trace-id\">{short_id}&hellip;</span></td>\
             <td>{service}</td>\
             <td>{duration}</td>\
             <td>{spans}</td>\
             <td><span class=\"{status_class}\">{status_icon}</span></td>\
             </tr>",
            trace_url = trace_url,
            short_id = html_escape(short_id),
            service = service,
            duration = duration,
            spans = trace.span_count,
            status_class = status_class,
            status_icon = status_icon,
        ));
    }

    format!(
        "<div class=\"panel trace-panel\">\
         <div class=\"panel-head\"><div><h2>Traces</h2></div>\
         <span class=\"pill\">{filtered} of {total}</span></div>\
         <table class=\"trace-table\">\
            <thead><tr>\
              <th>Trace ID</th>\
              <th>Service</th>\
              <th>Duration</th>\
              <th>Spans</th>\
              <th>Status</th>\
            </tr></thead>\
            <tbody>{rows}</tbody>\
         </table>\
         </div>",
        filtered = traces.len(),
        total = total_count,
        rows = rows,
    )
}

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

fn build_status_facets(traces: &[TraceSummary]) -> Vec<(String, usize)> {
    let mut ok = 0usize;
    let mut error = 0usize;
    for trace in traces {
        if trace.error_count > 0 {
            error += 1;
        } else {
            ok += 1;
        }
    }
    let mut facets = vec![("error".to_string(), error), ("ok".to_string(), ok)];
    facets.sort_by(|a, b| b.1.cmp(&a.1));
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

fn render_trace_detail(trace_id: &str, spans: &[RecordedSpan]) -> String {
    let start = spans.first().map(|s| s.start_time).unwrap_or_else(Utc::now);
    let end = spans
        .last()
        .map(|s| s.end_time)
        .unwrap_or(start + chrono::Duration::milliseconds(1));
    let duration = (end - start).num_milliseconds();
    let distinct_tools = collect_distinct_tools(spans);
    let error_spans = spans
        .iter()
        .filter(|span| span.error.is_some() || span.tool_status.as_deref() == Some("error"))
        .count();

    // Service chain: first -> last distinct tool
    let svc_chain = if distinct_tools.is_empty() {
        "&mdash;".to_string()
    } else if distinct_tools.len() == 1 {
        html_escape(&distinct_tools[0])
    } else {
        format!(
            "{} &rarr; {}",
            html_escape(&distinct_tools[0]),
            html_escape(&distinct_tools[distinct_tools.len() - 1])
        )
    };

    let status_badge = if error_spans > 0 {
        "<span class=\"hdr-badge error\">ERR</span>"
    } else {
        "<span class=\"hdr-badge ok\">OK</span>"
    };

    let short_id = if trace_id.len() >= 8 {
        &trace_id[..8]
    } else {
        trace_id
    };

    let header_bar = format!(
        "<div class=\"trace-header-bar\">\
         <a class=\"hdr-back\" href=\"/traces\">&larr; Back</a>\
         <span class=\"hdr-sep\">|</span>\
         <span class=\"hdr-trace-id\">Trace {short_id}&hellip;</span>\
         <span class=\"hdr-sep\">|</span>\
         <span class=\"hdr-svc\">{svc}</span>\
         <span class=\"hdr-sep\">|</span>\
         <span class=\"hdr-dur\">{dur}</span>\
         <span class=\"hdr-sep\">|</span>\
         {badge}\
         </div>",
        short_id = html_escape(short_id),
        svc = svc_chain,
        dur = format_duration(duration),
        badge = status_badge,
    );

    let waterfall = render_waterfall_with_hierarchy(start, end, spans);
    let attrs_panel = render_attributes_panel();

    format!(
        "<div class=\"trace-detail\">\
         {header}\
         <section class=\"wf-section\">{waterfall}</section>\
         {attrs}\
         </div>",
        header = header_bar,
        waterfall = waterfall,
        attrs = attrs_panel,
    )
}

/// DFS traversal to produce ordered `(span_index, depth)` pairs.
/// Must be a standalone fn — Rust cannot recurse closures.
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

fn render_waterfall_with_hierarchy(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    spans: &[RecordedSpan],
) -> String {
    if spans.is_empty() {
        return "<p class=\"empty\">No spans recorded for this trace.</p>".to_string();
    }
    let total_ms = (end - start).num_milliseconds().max(1);

    // Build parent/child map
    let mut ids = HashSet::new();
    for span in spans {
        ids.insert(span.span_id.clone());
    }
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

    // Time axis: 5 evenly-spaced tick labels
    let tick_labels: String = (0usize..=4)
        .map(|i| {
            let ms = (total_ms as f64 * i as f64 / 4.0) as i64;
            format!("<span>{}</span>", format_duration(ms))
        })
        .collect();
    let time_axis = format!("<div class=\"wf-time-axis\">{tick_labels}</div>");

    // Waterfall rows
    let mut rows = String::new();
    for (idx, depth) in &ordered {
        let span = &spans[*idx];
        let label = span
            .tool_name
            .as_deref()
            .map(html_escape)
            .unwrap_or_else(|| html_escape(&span.name));
        let offset_ms = (span.start_time - start).num_milliseconds().max(0);
        let offset_pct = ((offset_ms as f64 / total_ms as f64) * 100.0).clamp(0.0, 100.0);
        let width_pct = ((span.duration_ms.max(1) as f64 / total_ms as f64) * 100.0)
            .clamp(0.5, 100.0 - offset_pct);
        let bar_class = match span.tool_status.as_deref() {
            Some("error") => "wf-bar error",
            Some("ok") => "wf-bar ok",
            _ => "wf-bar",
        };
        let err_icon = if span.error.is_some() || span.tool_status.as_deref() == Some("error") {
            "<span class=\"wf-err-icon\">&#x2717;</span>"
        } else {
            ""
        };
        let padding_left = depth * 16;
        let duration = format_duration(span.duration_ms);

        // Build attrs JSON for the click-handler; html_escape so quotes become &quot;
        // The browser decodes HTML entities before getAttribute returns, so JSON.parse works.
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
        let attrs_escaped = html_escape(&attrs_json);

        rows.push_str(&format!(
            "<div class=\"wf-row\" data-span-id=\"{span_id}\" data-attrs=\"{attrs}\">\
               <div class=\"wf-label-col\" style=\"padding-left:{pad}px\">\
                 <span class=\"wf-label\">{err_icon}{label}</span>\
                 <span class=\"wf-dur\">{duration}</span>\
               </div>\
               <div class=\"wf-track-axis\">\
                 <div class=\"{bar_class}\" style=\"margin-left:{offset}%;width:{width}%;\"></div>\
               </div>\
             </div>",
            span_id = html_escape(&span.span_id),
            attrs = attrs_escaped,
            pad = padding_left,
            err_icon = err_icon,
            label = label,
            duration = duration,
            bar_class = bar_class,
            offset = offset_pct,
            width = width_pct,
        ));
    }

    format!(
        "<div class=\"wf-container\">{time_axis}{rows}</div>",
        time_axis = time_axis,
        rows = rows,
    )
}

fn render_attributes_panel() -> String {
    "<div id=\"span-attrs-panel\" class=\"attrs-panel\">\
     <div id=\"span-attrs-content\">Click a span row to view its attributes.</div>\
     </div>"
        .to_string()
}

fn detail_js() -> &'static str {
    r#"
(function(){
  var rows = document.querySelectorAll('.wf-row');
  var panel = document.getElementById('span-attrs-content');
  rows.forEach(function(row){
    row.addEventListener('click', function(){
      rows.forEach(function(r){ r.classList.remove('selected'); });
      row.classList.add('selected');
      var raw = row.getAttribute('data-attrs');
      var data;
      try { data = JSON.parse(raw); } catch(e){ panel.textContent = 'Could not parse attributes.'; return; }
      var html = '<table class="attr-table">';
      function addRow(k, v){
        html += '<tr><td class="attr-k">' + k + '</td><td class="attr-v">' + String(v) + '</td></tr>';
      }
      if(data.span_id) addRow('span_id', data.span_id);
      if(data.name) addRow('name', data.name);
      if(data.tool_name) addRow('tool_name', data.tool_name);
      if(data.status) addRow('status', data.status);
      if(data.duration_ms !== undefined) addRow('duration_ms', data.duration_ms + ' ms');
      if(data.observation) addRow('observation', data.observation);
      if(data.error) addRow('error', data.error);
      if(data.attributes && typeof data.attributes === 'object'){
        Object.keys(data.attributes).forEach(function(k){ addRow(k, data.attributes[k]); });
      }
      html += '</table>';
      panel.innerHTML = html;
    });
  });
})();
"#
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
