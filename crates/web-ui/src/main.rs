mod a2a;
pub mod auth;
mod chat;
pub mod common;
mod legacy;
mod traces;
mod webhooks;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use assistant_storage::{
    default_db_path, LogStats, LogStore, MetricsStore, RecordedLog, StorageLayer,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
    routing::{get, post},
    Extension, Router,
};
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
pub(crate) struct AppState {
    pub(crate) pool: SqlitePool,
    pub(crate) trace_limit: i64,
    pub(crate) log_limit: i64,
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
        .merge(traces::traces_router())
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

fn html_escape(input: &str) -> String {
    common::html_escape(input)
}

fn url_encode(input: &str) -> String {
    common::url_encode(input)
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

fn default_css() -> &'static str {
    common::default_css()
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    common::internal_error(err)
}
