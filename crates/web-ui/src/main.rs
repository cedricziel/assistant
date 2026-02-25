use std::collections::{HashMap, HashSet};
use std::{net::SocketAddr, path::PathBuf};

use anyhow::Result;
use assistant_storage::{
    default_db_path, RecordedSpan, StorageLayer, TraceStats, TraceStore, TraceSummary,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    routing::get,
    Router,
};
use chrono::{DateTime, Local, Utc};
use clap::Parser;
use serde::Deserialize;
use serde_json::to_string_pretty;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
struct Args {
    /// Address to listen on (e.g. 127.0.0.1:8080)
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,

    /// Path to the SQLite database (defaults to ~/.assistant/assistant.db)
    #[arg(long)]
    db_path: Option<PathBuf>,

    /// Maximum number of traces to show on the traces page
    #[arg(long, default_value_t = 200)]
    trace_limit: i64,
}

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    trace_limit: i64,
}

#[derive(Debug, Default, Deserialize)]
struct TraceQuery {
    skill: Option<String>,
    status: Option<String>,
    conversation: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::default().add_directive(Level::INFO.into())),
        )
        .init();

    let db_path = match args.db_path.or_else(default_db_path) {
        Some(p) => p,
        None => anyhow::bail!("Cannot determine default DB path. Specify --db-path."),
    };

    let storage = StorageLayer::new(&db_path).await?;
    let state = AppState {
        pool: storage.pool.clone(),
        trace_limit: args.trace_limit,
    };

    let router = Router::new()
        .route("/", get(show_dashboard))
        .route("/traces", get(show_dashboard))
        .route("/trace/:trace_id", get(show_trace_detail))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = args.listen.parse()?;
    info!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router.into_make_service()).await?;
    Ok(())
}

async fn show_dashboard(
    State(state): State<AppState>,
    Query(query): Query<TraceQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    let store = TraceStore::new(state.pool.clone());
    let skills = store.list_skills().await.map_err(internal_error)?;

    let mut skill_stats: Vec<TraceStats> = Vec::new();
    for skill in &skills {
        let stats = store
            .stats_for_skill(skill, 100)
            .await
            .map_err(internal_error)?;
        skill_stats.push(stats);
    }

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

    let all_traces = store
        .list_recent_traces(state.trace_limit, None)
        .await
        .map_err(internal_error)?;
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

    let skill_facets = build_skill_facets(&all_traces);
    let status_facets = build_status_facets(&all_traces);

    let query_builder = render_query_builder(
        &skills,
        skill_filter,
        status_value.as_deref(),
        conversation_value.as_deref(),
    );
    let sidebar = render_sidebar(
        &skill_facets,
        skill_filter,
        &status_facets,
        status_value.as_deref(),
        conversation_value.as_deref(),
    );
    let trace_panel = render_trace_list(&traces);
    let skill_panel = format!(
        "<div class=\"panel\"><div class=\"panel-head\"><div><h2>Skill health</h2>\
         <p>Rolling stats over the last 100 executions per skill</p></div></div>{stats}</div>",
        stats = render_skill_stats(&skill_stats),
    );

    let body = format!(
        "<html><head><title>Agent Trace Analytics</title><style>{css}</style></head>\
         <body><div class=\"layout\">\
         <aside class=\"sidebar\">{sidebar}</aside>\
         <main class=\"main\">{query}{traces}{skills}</main>\
         </div></body></html>",
        css = default_css(),
        sidebar = sidebar,
        query = query_builder,
        traces = trace_panel,
        skills = skill_panel,
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
    let body = format!(
        "<html><head><title>Trace {trace_id}</title><style>{css}</style></head>\
         <body><div class=\"page\">{detail}</div></body></html>",
        trace_id = html_escape(&trace_id),
        css = default_css(),
        detail = detail_html,
    );

    Ok(Html(body))
}

fn render_query_builder(
    skills: &[String],
    selected_skill: Option<&str>,
    status_value: Option<&str>,
    conversation_value: Option<&str>,
) -> String {
    let skill_options = render_skill_options(skills, selected_skill);
    let status_val = status_value.unwrap_or("");
    let conversation_val = conversation_value.unwrap_or("");
    format!(
        "<section class=\"panel query-builder\">\
         <div class=\"panel-head\"><div><p class=\"eyebrow\">Query builder</p>\
         <h1>Slice the timeline</h1>\
         <p>Combine filters to inspect specific agent behaviours.</p></div></div>\
         <form method=\"get\">\
            <div class=\"qb-grid\">\
              <label>Skill<select name=\"skill\"><option value=\"\">All skills</option>{skill_options}</select></label>\
              <label>Status<select name=\"status\">\
                  <option value=\"\"{status_all}>All outcomes</option>\
                  <option value=\"ok\"{status_ok}>Successful runs</option>\
                  <option value=\"error\"{status_err}>Runs with errors</option>\
              </select></label>\
              <label>Conversation<input type=\"text\" name=\"conversation\" value=\"{conversation}\" placeholder=\"Conversation UUID\"></label>\
            </div>\
            <div class=\"qb-actions\">\
                <button type=\"submit\">Run query</button>\
                <a href=\"/\">Clear all</a>\
            </div>\
         </form>\
         </section>",
        status_all = if status_val.is_empty() { " selected" } else { "" },
        status_ok = if status_val.eq_ignore_ascii_case("ok") {
            " selected"
        } else {
            ""
        },
        status_err = if status_val.eq_ignore_ascii_case("error") {
            " selected"
        } else {
            ""
        },
        conversation = html_escape(conversation_val),
    )
}

fn render_sidebar(
    skill_facets: &[(String, usize)],
    selected_skill: Option<&str>,
    status_facets: &[(String, usize)],
    selected_status: Option<&str>,
    conversation: Option<&str>,
) -> String {
    let mut skill_items = String::new();
    for (skill, count) in skill_facets.iter().take(12) {
        let active = selected_skill
            .map(|s| s.eq_ignore_ascii_case(skill))
            .unwrap_or(false);
        let url = build_query_url(Some(skill.as_str()), selected_status, conversation);
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

    let mut status_items = String::new();
    for (status, count) in status_facets {
        let label = match status.as_str() {
            "ok" => "Healthy runs",
            "error" => "Runs with errors",
            other => other,
        };
        let active = selected_status
            .map(|s| s.eq_ignore_ascii_case(status))
            .unwrap_or(false);
        let url = build_query_url(selected_skill, Some(status.as_str()), conversation);
        status_items.push_str(&format!(
            "<li><a class=\"facet-link{active}\" href=\"{url}\">\
             <span>{label}</span><em>{count}</em></a></li>",
            active = if active { " active" } else { "" },
            url = url,
            label = html_escape(label),
            count = count,
        ));
    }
    if status_items.is_empty() {
        status_items.push_str("<li class=\"muted\">No traces available</li>");
    }

    let conversation_chip = conversation.map(|conv| {
        format!(
            "<div class=\"facet-note\">Conversation filter active:<br><strong>{}</strong></div>",
            html_escape(conv)
        )
    });

    format!(
        "<div class=\"sidebar-inner\">\
         <div class=\"brand\"><p>assistant</p><h2>Telemetry</h2></div>\
         <div class=\"facet-group\"><h3>Skills</h3><ul>{skills}</ul></div>\
         <div class=\"facet-group\"><h3>Status</h3><ul>{status}</ul></div>\
         {conversation} \
         <div class=\"facet-footer\"><a href=\"/\">Reset filters</a></div>\
         </div>",
        skills = skill_items,
        status = status_items,
        conversation = conversation_chip.unwrap_or_default(),
    )
}

fn render_trace_list(traces: &[TraceSummary]) -> String {
    if traces.is_empty() {
        return "<div class=\"panel trace-panel\"><div class=\"panel-head\"><div><h2>Trace results</h2><p>No traces match this query yet. Trigger a skill or soften the filters.</p></div></div><p class=\"empty\">No rows to display.</p></div>".to_string();
    }

    let mut rows = String::new();
    for trace in traces {
        let ts: DateTime<Local> = DateTime::from(trace.start_time);
        let elapsed_ms = (trace.end_time - trace.start_time).num_milliseconds();
        let conversation = trace
            .conversation_id
            .as_ref()
            .map(|id| {
                let text = html_escape(&id.to_string());
                format!(
                    "<a href=\"/?conversation={id}\">{text}</a>",
                    id = text,
                    text = text
                )
            })
            .unwrap_or_else(|| "&mdash;".to_string());
        let status_class = if trace.error_count > 0 {
            "status-pill error"
        } else {
            "status-pill ok"
        };
        let status_label = if trace.error_count > 0 {
            format!(
                "{} error{}",
                trace.error_count,
                if trace.error_count == 1 { "" } else { "s" }
            )
        } else {
            "Healthy".to_string()
        };
        let mut tool_preview = String::new();
        let mut shown = 0usize;
        for tool in trace.tool_names.iter().filter(|t| !t.is_empty()) {
            if shown >= 3 {
                break;
            }
            if shown > 0 {
                tool_preview.push_str("<span class=\"dot\">&middot;</span>");
            }
            tool_preview.push_str(&html_escape(tool));
            shown += 1;
        }
        if shown == 0 {
            tool_preview.push_str("<span class=\"muted\">No tools</span>");
        } else if trace.tool_names.len() > shown {
            tool_preview.push_str(&format!(
                "<span class=\"muted\">+{} more</span>",
                trace.tool_names.len() - shown
            ));
        }
        let trace_url = html_escape(&trace.trace_id);
        rows.push_str(&format!(
            "<tr>\
             <td><div class=\"primary\">{time}</div><div class=\"subtle\">{duration}</div></td>\
             <td>{conversation}</td>\
             <td>{spans}</td>\
             <td><span class=\"{status_class}\">{status}</span></td>\
             <td>{tools}</td>\
             <td><a href=\"/trace/{trace}\">Open</a></td>\
             </tr>",
            time = ts.format("%b %d, %H:%M"),
            duration = format_duration(elapsed_ms),
            conversation = conversation,
            spans = trace.span_count,
            status_class = status_class,
            status = html_escape(&status_label),
            tools = tool_preview,
            trace = trace_url,
        ));
    }

    format!(
        "<div class=\"panel trace-panel\">\
         <div class=\"panel-head\"><div><h2>Trace results</h2><p>{count} matching runs (newest first)</p></div>\
         <span class=\"pill\">{count}</span></div>\
         <table class=\"trace-table\">\
            <thead><tr><th>Timestamp</th><th>Conversation</th><th>Spans</th><th>Status</th><th>Tools seen</th><th></th></tr></thead>\
            <tbody>{rows}</tbody>\
         </table>\
         </div>",
        count = traces.len(),
        rows = rows,
    )
}

fn render_skill_options(skills: &[String], selected: Option<&str>) -> String {
    let mut options = String::new();
    for skill in skills {
        let name = html_escape(skill);
        let selected_attr = selected
            .map(|s| s.eq_ignore_ascii_case(skill))
            .unwrap_or(false);
        let attr = if selected_attr { " selected" } else { "" };
        options.push_str(&format!(
            "<option value=\"{value}\"{attr}>{label}</option>",
            value = name,
            attr = attr,
            label = name
        ));
    }
    options
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
    if parts.is_empty() {
        "/".to_string()
    } else {
        format!("/?{}", parts.join("&"))
    }
}

fn url_encode(input: &str) -> String {
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

fn render_skill_stats(stats: &[TraceStats]) -> String {
    if stats.is_empty() {
        return "<p class=\"empty\">No skill executions recorded yet.</p>".to_string();
    }
    let mut cards = String::new();
    for stat in stats {
        let success_rate = if stat.total > 0 {
            (stat.success_count as f64 / stat.total as f64) * 100.0
        } else {
            0.0
        };
        let severity = if stat.error_count > 0 {
            "skill-card warn"
        } else {
            "skill-card"
        };
        let errors = if stat.common_errors.is_empty() {
            "<span class=\"muted\">No recurring errors</span>".to_string()
        } else {
            let joined = stat
                .common_errors
                .iter()
                .take(3)
                .map(|e| format!("<li>{}</li>", html_escape(e)))
                .collect::<String>();
            format!("<ul>{}</ul>", joined)
        };
        cards.push_str(&format!(
            "<article class=\"{class}\">\
             <h3>{name}</h3>\
             <div class=\"metrics\">\
               <div><p>Success rate</p><strong>{rate:.0}%</strong></div>\
               <div><p>Avg duration</p><strong>{avg:.0} ms</strong></div>\
               <div><p>Runs</p><strong>{total}</strong></div>\
             </div>\
             <div class=\"errors\"><p>Top errors</p>{errors}</div>\
             </article>",
            class = severity,
            name = html_escape(&stat.skill_name),
            rate = success_rate,
            avg = stat.avg_duration_ms,
            total = stat.total,
            errors = errors,
        ));
    }
    format!("<div class=\"skills-grid\">{}</div>", cards)
}

fn render_trace_detail(trace_id: &str, spans: &[RecordedSpan]) -> String {
    let start = spans.first().map(|s| s.start_time).unwrap_or_else(Utc::now);
    let end = spans
        .last()
        .map(|s| s.end_time)
        .unwrap_or(start + chrono::Duration::milliseconds(1));
    let duration = (end - start).num_milliseconds();
    let conversation = spans
        .iter()
        .find_map(|s| s.conversation_id.as_ref())
        .map(|id| id.to_string());
    let distinct_tools = collect_distinct_tools(spans);
    let error_spans = spans
        .iter()
        .filter(|span| span.error.is_some() || span.tool_status.as_deref() == Some("error"))
        .count();
    let tool_invocations = spans.iter().filter(|span| span.tool_name.is_some()).count();

    let meta = format!(
        "<div class=\"metrics-grid\">\
         <div><p>Conversation</p><strong>{conversation}</strong></div>\
         <div><p>Duration</p><strong>{duration}</strong></div>\
         <div><p>Tool calls</p><strong>{tools}</strong></div>\
         <div><p>Error spans</p><strong>{errors}</strong></div>\
         </div>",
        conversation = conversation
            .map(|id| {
                let safe = html_escape(&id);
                format!(
                    "<a href=\"/?conversation={id}\">{label}</a>",
                    id = safe,
                    label = safe
                )
            })
            .unwrap_or_else(|| "&mdash;".to_string()),
        duration = format_duration(duration),
        tools = tool_invocations,
        errors = error_spans,
    );

    let tool_badges = if distinct_tools.is_empty() {
        "<span class=\"badge muted\">No tool spans captured</span>".to_string()
    } else {
        distinct_tools
            .iter()
            .map(|tool| format!("<span class=\"badge\">{}</span>", html_escape(tool)))
            .collect()
    };

    let tree = render_span_tree(spans);

    format!(
        "<div class=\"trace-detail\">\
         <a class=\"back-link\" href=\"/\">&larr; Back to all traces</a>\
         <h1>Trace timeline</h1>\
         <p class=\"lede\">Trace ID: {trace}</p>\
         {meta}\
         <div class=\"tool-badges\"><p>Tools invoked</p>{badges}</div>\
         <section><h2>Span hierarchy</h2>{tree}</section>\
         </div>",
        trace = html_escape(trace_id),
        meta = meta,
        badges = tool_badges,
        tree = tree,
    )
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

fn render_span_tree(spans: &[RecordedSpan]) -> String {
    if spans.is_empty() {
        return "<p class=\"empty\">No spans recorded for this trace.</p>".to_string();
    }

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

    render_span_nodes(&roots, spans, &children)
}

fn render_span_nodes(
    indexes: &[usize],
    spans: &[RecordedSpan],
    children: &HashMap<String, Vec<usize>>,
) -> String {
    if indexes.is_empty() {
        return String::new();
    }
    let mut html = String::new();
    html.push_str("<ul class=\"span-tree\">");
    for idx in indexes {
        html.push_str("<li>");
        html.push_str(&render_span_node(*idx, spans, children));
        html.push_str("</li>");
    }
    html.push_str("</ul>");
    html
}

fn render_span_node(
    index: usize,
    spans: &[RecordedSpan],
    children: &HashMap<String, Vec<usize>>,
) -> String {
    let span = &spans[index];
    let ts: DateTime<Local> = DateTime::from(span.start_time);
    let status_class = match span.tool_status.as_deref() {
        Some("error") => "span-card status-error",
        Some("ok") => "span-card status-ok",
        _ => "span-card",
    };
    let title = span
        .tool_name
        .as_deref()
        .map(html_escape)
        .unwrap_or_else(|| html_escape(&span.name));
    let duration = format_duration(span.duration_ms);
    let observation = span
        .observation
        .as_deref()
        .map(|text| {
            format!(
                "<div class=\"span-observation\"><strong>Observation</strong><pre>{}</pre></div>",
                html_escape(text.trim())
            )
        })
        .unwrap_or_default();
    let error = span
        .error
        .as_deref()
        .map(|text| {
            format!(
                "<div class=\"span-error\"><strong>Error</strong><pre>{}</pre></div>",
                html_escape(text.trim())
            )
        })
        .unwrap_or_default();
    let meta = format!(
        "Started {ts} &middot; Span {span_id}{turn}",
        ts = ts.format("%Y-%m-%d %H:%M:%S"),
        span_id = html_escape(&span.span_id),
        turn = span
            .turn
            .map(|t| format!(" &middot; Turn {t}"))
            .unwrap_or_default(),
    );
    let attrs = to_string_pretty(&span.attributes).unwrap_or_else(|_| span.attributes.to_string());

    let mut block = format!(
        "<div class=\"{class}\">\
         <div class=\"span-head\">\
            <div><p class=\"span-name\">{name}</p><h3>{title}</h3></div>\
            <span class=\"duration\">{duration}</span>\
         </div>\
         <p class=\"span-meta\">{meta}</p>\
         {observation}{error}\
         <details><summary>Attributes</summary><pre>{attrs}</pre></details>\
         </div>",
        class = status_class,
        name = html_escape(&span.name),
        title = title,
        duration = duration,
        meta = meta,
        observation = observation,
        error = error,
        attrs = html_escape(&attrs),
    );

    if let Some(child_indexes) = children.get(&span.span_id) {
        block.push_str(&render_span_nodes(child_indexes, spans, children));
    }

    block
}

fn format_duration(ms: i64) -> String {
    if ms >= 60_000 {
        format!("{:.1} min", ms as f64 / 60_000.0)
    } else if ms >= 1_000 {
        format!("{:.1} s", ms as f64 / 1_000.0)
    } else {
        format!("{ms} ms")
    }
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn default_css() -> &'static str {
    r#"
    :root {
        color-scheme: dark;
    }
    * {
        box-sizing: border-box;
    }
    body {
        font-family: "Inter", system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
        background: #030712;
        color: #e5e9f0;
        margin: 0;
    }
    a {
        color: #6ec6ff;
    }
    .layout {
        display: grid;
        grid-template-columns: 260px minmax(0, 1fr);
        min-height: 100vh;
    }
    .sidebar {
        background: #050f21;
        border-right: 1px solid #0b1b32;
        padding: 2.5rem 1.5rem;
    }
    .sidebar-inner {
        display: flex;
        flex-direction: column;
        gap: 2rem;
        height: 100%;
    }
    .brand p {
        text-transform: uppercase;
        letter-spacing: 0.2em;
        color: #7aa2ff;
        margin: 0 0 0.2rem;
        font-size: 0.75rem;
    }
    .brand h2 {
        margin: 0;
    }
    .facet-group h3 {
        margin: 0 0 0.8rem;
        color: #8aa5d8;
        font-size: 0.9rem;
        letter-spacing: 0.08em;
        text-transform: uppercase;
    }
    .facet-group ul {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 0.3rem;
    }
    .facet-link {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.4rem 0.2rem;
        color: inherit;
        text-decoration: none;
        border-radius: 8px;
    }
    .facet-link.active {
        background: rgba(110, 198, 255, 0.1);
    }
    .facet-link span {
        font-size: 0.95rem;
    }
    .facet-link em {
        font-style: normal;
        color: #8ba2c6;
    }
    .facet-footer {
        margin-top: auto;
    }
    .facet-note {
        font-size: 0.8rem;
        color: #9fb4d6;
        background: rgba(111, 163, 255, 0.08);
        padding: 0.8rem;
        border-radius: 12px;
    }
    .main {
        padding: 2.5rem 2.5rem 3rem;
        display: flex;
        flex-direction: column;
        gap: 1.75rem;
        background: #030712;
    }
    .panel {
        background: #050d1c;
        border: 1px solid #0f1f36;
        border-radius: 18px;
        padding: 1.5rem;
    }
    .panel-head {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 1rem;
        margin-bottom: 1rem;
    }
    .panel-head p {
        margin: 0.3rem 0 0;
        color: #8ba2c6;
    }
    .pill {
        background: rgba(110, 198, 255, 0.15);
        color: #d4ecff;
        border-radius: 999px;
        padding: 0.2rem 0.8rem;
        font-size: 0.9rem;
    }
    .query-builder form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    .qb-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 1rem;
    }
    .query-builder label {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
        font-size: 0.85rem;
        color: #9fb4d6;
    }
    .query-builder select,
    .query-builder input {
        background: #020511;
        border: 1px solid #15243b;
        border-radius: 12px;
        color: #e5e9f0;
        padding: 0.5rem 0.6rem;
    }
    .qb-actions {
        display: flex;
        gap: 1rem;
        align-items: center;
    }
    .qb-actions button {
        border: none;
        border-radius: 999px;
        background: linear-gradient(135deg, #64cafe, #8b5dff);
        color: #050b16;
        padding: 0.6rem 1.6rem;
        font-weight: 600;
        cursor: pointer;
    }
    .qb-actions a {
        color: #8ba2c6;
        text-decoration: none;
        font-size: 0.9rem;
    }
    .trace-table {
        width: 100%;
        border-collapse: collapse;
    }
    .trace-table th,
    .trace-table td {
        padding: 0.8rem;
        border-bottom: 1px solid #0f1f36;
        text-align: left;
        vertical-align: top;
    }
    .trace-table th {
        font-size: 0.8rem;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: #7da0d4;
    }
    .trace-table tr:last-child td {
        border-bottom: none;
    }
    .primary {
        font-weight: 600;
    }
    .subtle {
        color: #8aa2c6;
        font-size: 0.85rem;
    }
    .status-pill {
        padding: 0.2rem 0.8rem;
        border-radius: 999px;
        font-size: 0.8rem;
    }
    .status-pill.ok {
        background: rgba(99, 230, 190, 0.2);
        color: #9ef7d6;
    }
    .status-pill.error {
        background: rgba(248, 113, 113, 0.2);
        color: #ffb4b4;
    }
    .dot {
        margin: 0 0.3rem;
        color: #31425f;
    }
    .empty {
        color: #8ba2c6;
        margin: 0;
    }
    .skills-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 1rem;
    }
    .skill-card {
        background: #071222;
        border: 1px solid #14233a;
        padding: 1rem;
        border-radius: 18px;
    }
    .skill-card.warn {
        border-color: rgba(248, 113, 113, 0.4);
    }
    .skill-card h3 {
        margin-top: 0;
    }
    .skill-card .metrics {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 0.6rem;
    }
    .skill-card .metrics p {
        margin: 0;
        color: #8898b5;
        font-size: 0.75rem;
    }
    .skill-card .metrics strong {
        font-size: 1.1rem;
    }
    .skill-card .errors ul {
        margin: 0.3rem 0 0;
        padding-left: 1rem;
        color: #fca5a5;
    }
    .badge {
        background: rgba(94, 195, 255, 0.15);
        color: #9ccfff;
        border-radius: 999px;
        padding: 0.1rem 0.65rem;
        font-size: 0.8rem;
    }
    .badge.muted {
        background: rgba(255, 255, 255, 0.08);
        color: #a7b6d8;
    }
    .page {
        max-width: 1100px;
        margin: 0 auto;
        padding: 2.5rem 1.5rem 4rem;
    }
    .trace-detail {
        background: #050d1c;
        border: 1px solid #0f1f36;
        border-radius: 24px;
        padding: 2rem;
    }
    .back-link {
        text-decoration: none;
        color: #9ccfff;
        font-size: 0.9rem;
    }
    .metrics-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 1rem;
        margin: 1rem 0 1.5rem;
    }
    .metrics-grid p {
        margin: 0;
        color: #97a4c0;
        font-size: 0.85rem;
    }
    .metrics-grid strong {
        font-size: 1.4rem;
    }
    .tool-badges {
        display: flex;
        align-items: center;
        gap: 0.6rem;
        flex-wrap: wrap;
        margin-bottom: 1.5rem;
    }
    .tool-badges p {
        margin: 0;
        font-weight: 500;
    }
    .span-tree {
        list-style: none;
        padding-left: 0;
    }
    .span-tree li {
        margin-left: 1rem;
        padding-left: 1rem;
        border-left: 2px solid rgba(255, 255, 255, 0.05);
    }
    .span-card {
        background: #060b18;
        border: 1px solid #1f2937;
        border-radius: 16px;
        padding: 1rem;
        margin-bottom: 1rem;
    }
    .span-card.status-ok {
        border-color: rgba(94, 195, 255, 0.3);
    }
    .span-card.status-error {
        border-color: rgba(248, 113, 113, 0.6);
    }
    .span-head {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 1rem;
    }
    .span-name {
        margin: 0;
        color: #97a4c0;
        font-size: 0.85rem;
    }
    .span-meta {
        color: #97a4c0;
        font-size: 0.85rem;
    }
    .span-observation,
    .span-error {
        background: rgba(94, 195, 255, 0.08);
        border-radius: 12px;
        padding: 0.6rem;
        font-family: ui-monospace, monospace;
        white-space: pre-wrap;
    }
    .span-error {
        background: rgba(248, 113, 113, 0.12);
        color: #fecaca;
    }
    details {
        margin-top: 0.5rem;
    }
    details pre {
        white-space: pre-wrap;
        background: #030712;
        border-radius: 10px;
        padding: 0.5rem;
        font-size: 0.8rem;
    }
    @media (max-width: 900px) {
        .layout {
            grid-template-columns: 1fr;
        }
        .sidebar {
            border-right: none;
            border-bottom: 1px solid #0b1b32;
        }
    }
    @media (max-width: 640px) {
        .qb-actions {
            flex-direction: column;
            align-items: flex-start;
        }
        .span-tree li {
            margin-left: 0.4rem;
            padding-left: 0.8rem;
        }
    }
    "#
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
