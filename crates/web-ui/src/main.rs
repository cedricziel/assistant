use std::{net::SocketAddr, path::PathBuf};

use anyhow::Result;
use assistant_storage::{default_db_path, RecordedSpan, StorageLayer, TraceStore};
use axum::{extract::State, response::Html, routing::get, Router};
use chrono::{DateTime, Local};
use clap::Parser;
use serde::Deserialize;
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
        .route("/", get(show_skills))
        .route("/traces", get(show_traces))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = args.listen.parse()?;
    info!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router.into_make_service()).await?;
    Ok(())
}

async fn show_skills(
    State(state): State<AppState>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let store = TraceStore::new(state.pool.clone());
    let skills = store.list_skills().await.map_err(internal_error)?;

    let mut rows = String::new();
    for skill in skills {
        let stats = store
            .stats_for_skill(&skill, 100)
            .await
            .map_err(internal_error)?;
        rows.push_str(&format!(
            "<tr><td><a href=\"/traces?skill={name}\">{name}</a></td><td>{total}</td><td>{success}</td><td>{errors}</td><td>{avg:.1}</td></tr>",
            name = html_escape(&skill),
            total = stats.total,
            success = stats.success_count,
            errors = stats.error_count,
            avg = stats.avg_duration_ms,
        ));
    }

    let body = format!(
        "<html><head><title>Trace Analysis</title><style>{css}</style></head>\
         <body><h1>Skill Stats</h1>\
         <p><a href=\"/traces\">View recent traces</a></p>\
         <table><thead><tr><th>Skill</th><th>Total</th><th>Success</th><th>Error</th><th>Avg ms</th></tr></thead>\
         <tbody>{rows}</tbody></table></body></html>",
        css = default_css(),
        rows = rows,
    );

    Ok(Html(body))
}

#[derive(Deserialize)]
struct TraceQuery {
    skill: Option<String>,
}

async fn show_traces(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<TraceQuery>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let store = TraceStore::new(state.pool.clone());
    let traces = if let Some(skill) = &query.skill {
        store
            .get_recent_for_skill(skill, state.trace_limit)
            .await
            .map_err(internal_error)?
    } else {
        store
            .list_recent(state.trace_limit)
            .await
            .map_err(internal_error)?
    };

    let mut rows = String::new();
    for trace in traces {
        rows.push_str(&render_trace_row(&trace));
    }

    let header = if let Some(skill) = &query.skill {
        format!("Recent traces for {skill}")
    } else {
        "Recent traces".to_string()
    };

    let body = format!(
        "<html><head><title>Traces</title><style>{css}</style></head>\
         <body><h1>{header}</h1>\
         <p><a href=\"/\">Back to skill stats</a></p>\
         <table><thead><tr><th>Timestamp</th><th>Skill</th><th>Conversation</th><th>Duration (ms)</th><th>Observation</th><th>Error</th></tr></thead>\
         <tbody>{rows}</tbody></table></body></html>",
        css = default_css(),
        header = header,
        rows = rows,
    );
    Ok(Html(body))
}

fn render_trace_row(trace: &RecordedSpan) -> String {
    let ts: DateTime<Local> = DateTime::from(trace.start_time);
    let observation = trace
        .observation
        .as_deref()
        .map(|s| html_escape(s.trim()))
        .unwrap_or_else(|| "(no observation)".to_string());
    let error = trace
        .error
        .as_ref()
        .map(|e| html_escape(e))
        .unwrap_or_else(|| "&mdash;".to_string());
    let tool_name = trace
        .tool_name
        .as_deref()
        .map(html_escape)
        .unwrap_or_else(|| "(unknown)".to_string());
    let conversation = trace
        .conversation_id
        .map(|id| html_escape(&id.to_string()))
        .unwrap_or_else(|| "&mdash;".to_string());
    format!(
        "<tr><td>{ts}</td><td>{skill}</td><td>{conv}</td><td>{duration}</td><td class=\"obs\">{obs}</td><td class=\"err\">{err}</td></tr>",
        ts = ts.format("%Y-%m-%d %H:%M:%S"),
        skill = tool_name,
        conv = conversation,
        duration = trace.duration_ms,
        obs = observation,
        err = error,
    )
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
    "body { font-family: system-ui, sans-serif; margin: 2rem; }\n\
     table { border-collapse: collapse; width: 100%; }\n\
     th, td { border: 1px solid #ccc; padding: 0.4rem; text-align: left; }\n\
     th { background: #f2f2f2; }\n\
     tr:nth-child(even) { background: #fafafa; }\n\
     td.obs { max-width: 480px; word-wrap: break-word; }\n\
     td.err { color: #b00020; }"
}

fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        err.to_string(),
    )
}
