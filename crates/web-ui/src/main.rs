mod a2a;
pub mod auth;
mod chat;
pub mod common;
mod legacy;
mod webhooks;

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use assistant_storage::{
    default_db_path, LogStats, LogStore, MetricsStore, RecordedLog, RecordedSpan, StorageLayer,
    TraceStore, TraceSummary,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
    routing::{get, post},
    Extension, Router,
};
use chrono::{DateTime, Utc};
use clap::Parser;
use serde::Deserialize;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Level};
use tracing_subscriber::EnvFilter;

use auth::AuthConfig;

use a2a::agent_store::AgentStore;
use a2a::handlers::{build_default_agent_card, A2AState};
use a2a::pages::AgentPagesState;
use a2a::task_store::TaskStore;

#[derive(Parser, Debug)]
struct Args {
    /// Address to listen on (e.g. 127.0.0.1:8080)
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,

    /// Path to the SQLite database (defaults to ~/.assistant/assistant.db)
    #[arg(long)]
    db_path: Option<PathBuf>,

    /// Authentication token.  Falls back to ASSISTANT_WEB_TOKEN env var.
    /// The server will refuse to start without a token.
    #[arg(long, env = "ASSISTANT_WEB_TOKEN")]
    auth_token: Option<String>,

    /// Maximum number of traces to show on the traces page
    #[arg(long, default_value_t = 200)]
    trace_limit: i64,

    /// Maximum number of logs to show on the logs page
    #[arg(long, default_value_t = 500)]
    log_limit: i64,

    /// Disable the `Secure` attribute on session cookies.
    /// Useful when running behind a VPN or firewall over plain HTTP on a
    /// non-loopback address.  Without this flag, binding to a non-loopback
    /// address automatically sets `Secure`, which requires HTTPS.
    #[arg(long)]
    no_secure_cookie: bool,
}

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    trace_limit: i64,
    log_limit: i64,
}

#[derive(Debug, Default, Deserialize)]
struct TraceQuery {
    skill: Option<String>,
    status: Option<String>,
    conversation: Option<String>,
    min_duration_ms: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
struct LogQuery {
    severity: Option<String>,
    target: Option<String>,
    search: Option<String>,
    trace_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AnalyticsQuery {
    /// Time window in hours (defaults to 24).
    window: Option<i64>,
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

    // -- Auth token (required) -----------------------------------------------
    let auth_token = match args.auth_token.map(|t| t.trim().to_string()) {
        Some(t) if !t.is_empty() => t,
        _ => {
            anyhow::bail!(
                "No authentication token configured.\n\
                 Set --auth-token <TOKEN> or the ASSISTANT_WEB_TOKEN environment variable.\n\
                 The web UI refuses to start without authentication."
            );
        }
    };

    // Parse listen address early so we can pass `is_loopback` to AuthConfig.
    let addr: SocketAddr = args.listen.parse()?;
    let secure_cookie = !addr.ip().is_loopback() && !args.no_secure_cookie;
    let auth_config = AuthConfig::new(auth_token, secure_cookie);

    let db_path = match args.db_path.or_else(default_db_path) {
        Some(p) => p,
        None => anyhow::bail!("Cannot determine default DB path. Specify --db-path."),
    };

    let storage = StorageLayer::new(&db_path).await?;
    let state = AppState {
        pool: storage.pool.clone(),
        trace_limit: args.trace_limit,
        log_limit: args.log_limit,
    };

    // -- Agent store (filesystem-backed) --
    let agent_store = AgentStore::default_dir()?;

    // -- A2A protocol state --
    let base_url = format!("http://{}", args.listen);

    // Resolve the agent card from the store, falling back to a built-in default.
    let mut agent_card = match agent_store.get_default().await {
        Some(agent) => agent.card,
        None => build_default_agent_card(&base_url),
    };

    // Auto-harden: inject Bearer auth into the agent card so A2A callers
    // know they need to present a token.
    harden_agent_card(&mut agent_card);

    let a2a_state = A2AState {
        task_store: TaskStore::new(),
        agent_card,
    };

    let agent_pages_state = AgentPagesState {
        agent_store,
        base_url: base_url.clone(),
    };

    let webhook_pages_state = webhooks::pages::WebhookPagesState {
        pool: storage.pool.clone(),
    };

    let chat_state = chat::ChatState {
        pool: storage.pool.clone(),
    };

    // -- Router: public routes (no auth required) --------------------------
    let public_routes = Router::new()
        .route("/login", get(auth::login_page).post(auth::login_submit))
        .route("/logout", post(auth::logout))
        // A2A agent card is public per spec — callers need it to discover auth.
        .merge(a2a::public_router().with_state(a2a_state.clone()));

    // -- Router: protected routes (auth required) --------------------------
    let protected_routes = Router::new()
        // Trace / log UI routes.
        .route("/", get(|| async { Redirect::to("/chat") }))
        .route("/traces", get(show_dashboard))
        .route("/trace/{trace_id}", get(show_trace_detail))
        .route("/logs", get(show_logs))
        .route("/log/{log_id}", get(show_log_detail))
        .route("/analytics", get(show_analytics))
        .with_state(state)
        // A2A protocol routes (auth-protected endpoints only).
        .merge(a2a::protected_router().with_state(a2a_state))
        // Agent management UI pages.
        .merge(a2a::agent_pages_router().with_state(agent_pages_state))
        // Webhook management UI pages.
        .merge(webhooks::webhook_pages_router().with_state(webhook_pages_state))
        // Chat interface.
        .merge(chat::chat_router().with_state(chat_state))
        .route_layer(axum::middleware::from_fn(auth::require_auth));

    let router = public_routes
        .merge(protected_routes)
        .layer(Extension(auth_config))
        .layer(TraceLayer::new_for_http());

    // Warn when binding to a non-loopback address.
    if !addr.ip().is_loopback() {
        warn!(
            "Listening on non-loopback address {}. Ensure network access is intentional.",
            addr
        );
    }

    info!("Listening on http://{}", addr);
    info!("A2A agent card: http://{}/.well-known/agent.json", addr);
    info!("Authentication enabled — login at http://{}/login", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router.into_make_service()).await?;
    Ok(())
}

// -- Auto-hardening ---------------------------------------------------------

/// Inject Bearer authentication metadata into an [`AgentCard`] so that A2A
/// callers discover the auth requirement via the public card endpoint.
fn harden_agent_card(card: &mut assistant_a2a_json_schema::agent_card::AgentCard) {
    use assistant_a2a_json_schema::security::{
        HttpAuthSecurityScheme, SecurityRequirement, SecurityScheme,
    };
    use assistant_a2a_json_schema::types::StringList;

    let scheme_name = "bearer_token".to_string();

    // Ensure the security scheme exists.
    if !card.security_schemes.contains_key(&scheme_name) {
        card.security_schemes.insert(
            scheme_name.clone(),
            SecurityScheme {
                http_auth_security_scheme: Some(HttpAuthSecurityScheme {
                    description: Some(
                        "Bearer token authentication. Pass the token via \
                         Authorization: Bearer <token>."
                            .to_string(),
                    ),
                    scheme: "Bearer".to_string(),
                    bearer_format: None,
                }),
                api_key_security_scheme: None,
                oauth2_security_scheme: None,
                open_id_connect_security_scheme: None,
                mtls_security_scheme: None,
            },
        );
    }

    // Ensure a matching security requirement exists (checked independently
    // so that a card with the scheme but a missing requirement still gets
    // hardened).
    let has_requirement = card
        .security_requirements
        .iter()
        .any(|req| req.schemes.contains_key(&scheme_name));

    if !has_requirement {
        card.security_requirements.push(SecurityRequirement {
            schemes: HashMap::from([(
                scheme_name,
                StringList {
                    list: vec![], // no scopes required
                },
            )]),
        });
    }

    info!("Auto-hardened agent card with Bearer auth security scheme");
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

// -- Logs handlers --

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
    // Navigation
    let nav = "<div class=\"facet-group\">\
        <h3>Navigation</h3>\
        <ul>\
        <li><a class=\"facet-link\" href=\"/traces\"><span>Traces</span></a></li>\
        <li><a class=\"facet-link active\" href=\"/logs\"><span>Logs</span></a></li>\
        <li><a class=\"facet-link\" href=\"/agents\"><span>Agents</span></a></li>\
        <li><a class=\"facet-link\" href=\"/analytics\"><span>Analytics</span></a></li>\
        </ul></div>";

    // Severity facets
    let severity_options: &[(&str, &str, i64)] = &[
        ("", "All", stats.total),
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
         {nav}\
         <div class=\"facet-group\"><h3>Severity</h3>{severity_form}</div>\
         <div class=\"facet-group\"><h3>Target</h3><ul>{targets}</ul></div>\
         <div class=\"facet-group\"><h3>Search</h3>{search_form}</div>\
         <div class=\"facet-footer\">\
           <span class=\"trace-count\">Showing {shown} log records</span>\
           <a href=\"/logs\">Reset filters</a>\
         </div>\
         </div>",
        nav = nav,
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
    let target = log.target.as_deref().unwrap_or("&mdash;");

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

    let nav = "<div class=\"facet-group\">\
        <h3>Navigation</h3>\
        <ul>\
        <li><a class=\"facet-link active\" href=\"/traces\"><span>Traces</span></a></li>\
        <li><a class=\"facet-link\" href=\"/logs\"><span>Logs</span></a></li>\
        <li><a class=\"facet-link\" href=\"/agents\"><span>Agents</span></a></li>\
        <li><a class=\"facet-link\" href=\"/analytics\"><span>Analytics</span></a></li>\
        </ul></div>";

    format!(
        "<div class=\"sidebar-inner\">\
         <div class=\"brand\"><p>assistant</p><h2>Telemetry</h2></div>\
         {nav}\
         <div class=\"facet-group\"><h3>Service</h3><ul>{skills}</ul></div>\
         <div class=\"facet-group\"><h3>Status</h3>{status_form}</div>\
         <div class=\"facet-group\"><h3>Min Duration</h3>{min_dur_form}</div>\
         <div class=\"facet-footer\">\
           <span class=\"trace-count\">Showing {filtered} of {total} traces</span>\
           <a href=\"/traces\">Reset filters</a>\
         </div>\
         </div>",
        nav = nav,
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

    // Service chain: first → last distinct tool
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
    common::html_escape(input)
}

// -- Analytics dashboard -------------------------------------------------------

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

    let nav = format!(
        "<div class=\"facet-group\">\
         <h3>Navigation</h3>\
         <ul>\
         <li><a class=\"facet-link\" href=\"/traces\"><span>Traces</span></a></li>\
         <li><a class=\"facet-link\" href=\"/logs\"><span>Logs</span></a></li>\
         <li><a class=\"facet-link\" href=\"/agents\"><span>Agents</span></a></li>\
         <li><a class=\"facet-link active\" href=\"/analytics\"><span>Analytics</span></a></li>\
         </ul></div>\
         <div class=\"facet-group\">\
         <h3>Time Window</h3>\
         <ul>{window_links}</ul></div>"
    );

    let sidebar = format!(
        "<div class=\"sidebar-inner\">\
         <div class=\"brand\"><p>assistant</p><h2>Analytics</h2></div>\
         {nav}</div>"
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
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .logout-btn {
        background: transparent;
        border: 1px solid #1a2744;
        border-radius: 8px;
        color: #8aa5d8;
        padding: 0.45rem 0.8rem;
        font-size: 0.85rem;
        cursor: pointer;
        width: 100%;
        transition: background 0.15s, color 0.15s;
    }
    .logout-btn:hover {
        background: rgba(239, 68, 68, 0.12);
        color: #fca5a5;
        border-color: rgba(239, 68, 68, 0.3);
    }
    .trace-count {
        font-size: 0.8rem;
        color: #8aa5d8;
    }
    .radio-label {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 0.9rem;
        color: #c2d6f0;
        cursor: pointer;
        padding: 0.25rem 0;
    }
    .radio-label input[type=radio] {
        accent-color: #6ec6ff;
    }
    .min-dur-row {
        display: flex;
        gap: 0.5rem;
        align-items: center;
    }
    .min-dur-row input[type=number] {
        background: #020511;
        border: 1px solid #15243b;
        border-radius: 8px;
        color: #e5e9f0;
        padding: 0.35rem 0.6rem;
        font-size: 0.85rem;
        width: 80px;
    }
    .min-dur-row button {
        background: linear-gradient(135deg, #64cafe, #8b5dff);
        border: none;
        border-radius: 8px;
        color: #050b16;
        padding: 0.35rem 0.8rem;
        font-weight: 600;
        cursor: pointer;
        font-size: 0.85rem;
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
        white-space: nowrap;
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
        vertical-align: middle;
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
    .trace-table tbody tr {
        cursor: pointer;
        transition: background 0.1s;
    }
    .trace-table tbody tr:hover {
        background: rgba(110, 198, 255, 0.06);
    }
    .trace-id {
        font-family: ui-monospace, monospace;
        font-size: 0.85rem;
        color: #a0bfe0;
    }
    .status-ok {
        color: #63e6be;
        font-weight: 700;
    }
    .status-err {
        color: #f87171;
        font-weight: 700;
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
    .muted {
        color: #8ba2c6;
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
    /* Trace detail page */
    .page {
        max-width: 1400px;
        margin: 0 auto;
        padding: 2.5rem 1.5rem 4rem;
    }
    .trace-detail {
        background: #050d1c;
        border: 1px solid #0f1f36;
        border-radius: 24px;
        padding: 2rem;
    }
    /* Header bar */
    .trace-header-bar {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        flex-wrap: wrap;
        margin-bottom: 1.5rem;
        padding-bottom: 1rem;
        border-bottom: 1px solid #0f1f36;
    }
    .hdr-back {
        text-decoration: none;
        color: #9ccfff;
        font-size: 0.9rem;
    }
    .hdr-sep {
        color: #2a3d5a;
    }
    .hdr-trace-id {
        font-family: ui-monospace, monospace;
        font-size: 0.9rem;
        color: #c8def5;
    }
    .hdr-svc {
        font-weight: 600;
    }
    .hdr-dur {
        color: #a0bfe0;
    }
    .hdr-badge {
        border-radius: 999px;
        padding: 0.2rem 0.8rem;
        font-size: 0.8rem;
        font-weight: 600;
    }
    .hdr-badge.ok {
        background: rgba(99, 230, 190, 0.2);
        color: #9ef7d6;
    }
    .hdr-badge.error {
        background: rgba(248, 113, 113, 0.2);
        color: #ffb4b4;
    }
    /* Waterfall */
    .wf-container {
        display: flex;
        flex-direction: column;
    }
    .wf-time-axis {
        display: flex;
        justify-content: space-between;
        margin-left: 250px;
        font-size: 0.75rem;
        color: #5a7396;
        padding-bottom: 0.5rem;
        border-bottom: 1px solid #0d1a2e;
        margin-bottom: 0.5rem;
    }
    .wf-row {
        display: flex;
        align-items: center;
        height: 30px;
        border-radius: 6px;
        cursor: pointer;
        transition: background 0.1s;
    }
    .wf-row:hover {
        background: rgba(110, 198, 255, 0.07);
    }
    .wf-row.selected {
        background: rgba(110, 198, 255, 0.13);
    }
    .wf-label-col {
        width: 250px;
        min-width: 250px;
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding-right: 0.75rem;
        overflow: hidden;
    }
    .wf-label {
        font-size: 0.82rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        color: #c0d4ee;
    }
    .wf-dur {
        font-size: 0.75rem;
        color: #5a7396;
        white-space: nowrap;
        margin-left: 0.5rem;
    }
    .wf-err-icon {
        color: #f87171;
        margin-right: 0.3rem;
        font-size: 0.8rem;
    }
    .wf-track-axis {
        flex: 1;
        position: relative;
        height: 18px;
        border-radius: 999px;
        background: rgba(148, 163, 184, 0.1);
    }
    .wf-bar {
        position: absolute;
        top: 0;
        bottom: 0;
        border-radius: 999px;
        background: rgba(94, 195, 255, 0.7);
        min-width: 3px;
    }
    .wf-bar.ok {
        background: rgba(99, 230, 190, 0.75);
    }
    .wf-bar.error {
        background: rgba(248, 113, 113, 0.85);
    }
    .wf-section {
        margin: 0.5rem 0 1.5rem;
    }
    /* Attributes panel */
    .attrs-panel {
        background: #030a15;
        border: 1px solid #0f1f36;
        border-radius: 14px;
        padding: 1rem;
        min-height: 80px;
        font-size: 0.85rem;
        color: #9fb4d6;
    }
    .attr-table {
        width: 100%;
        border-collapse: collapse;
    }
    .attr-table td {
        padding: 0.3rem 0.5rem;
        vertical-align: top;
        border-bottom: 1px solid #0d1a2e;
    }
    .attr-k {
        font-family: ui-monospace, monospace;
        color: #6ec6ff;
        width: 35%;
        white-space: nowrap;
    }
    .attr-v {
        font-family: ui-monospace, monospace;
        color: #c0d4ee;
        word-break: break-all;
    }
    /* Log severity badges */
    .sev-trace { color: #8ba2c6; }
    .sev-debug { color: #a78bfa; }
    .sev-info { color: #6ec6ff; }
    .sev-warn { color: #fbbf24; font-weight: 600; }
    .sev-error { color: #f87171; font-weight: 600; }
    .sev-fatal { color: #ff4040; font-weight: 700; text-transform: uppercase; }
    .sev-unknown { color: #8ba2c6; }
    /* Log table tweaks */
    .log-table .log-ts {
        font-family: ui-monospace, monospace;
        font-size: 0.82rem;
        color: #8ba2c6;
        white-space: nowrap;
    }
    .log-table .log-body {
        max-width: 400px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 0.85rem;
    }
    /* Log detail page */
    .log-detail-grid {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
        margin-top: 1rem;
    }
    .log-detail-grid h3 {
        margin: 0 0 0.5rem;
        color: #8aa5d8;
        font-size: 0.9rem;
        text-transform: uppercase;
        letter-spacing: 0.08em;
    }
    .log-body-pre {
        background: #020511;
        border: 1px solid #15243b;
        border-radius: 10px;
        padding: 1rem;
        color: #c0d4ee;
        font-family: ui-monospace, monospace;
        font-size: 0.85rem;
        white-space: pre-wrap;
        word-break: break-all;
        overflow-x: auto;
        max-height: 400px;
    }
    /* Search input in sidebar */
    .min-dur-row input[type=text] {
        background: #020511;
        border: 1px solid #15243b;
        border-radius: 8px;
        color: #e5e9f0;
        padding: 0.35rem 0.6rem;
        font-size: 0.85rem;
        flex: 1;
    }
    @media (max-width: 900px) {
        .layout {
            grid-template-columns: 1fr;
        }
        .sidebar {
            border-right: none;
            border-bottom: 1px solid #0b1b32;
        }
        .wf-label-col {
            width: 150px;
            min-width: 150px;
        }
        .wf-time-axis {
            margin-left: 150px;
        }
    }
    @media (max-width: 640px) {
        .wf-label-col {
            width: 100px;
            min-width: 100px;
        }
        .wf-time-axis {
            margin-left: 100px;
        }
    }
    "#
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    common::internal_error(err)
}
