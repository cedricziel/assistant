mod a2a;
mod analytics;
pub mod auth;
mod chat;
pub mod common;
mod legacy;
mod logs;
mod traces;
mod webhooks;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_core::LlmProviderKind;
use assistant_llm::LlmProvider;
use assistant_provider_anthropic::AnthropicProvider;
use assistant_provider_ollama::OllamaProvider;
use assistant_provider_openai::OpenAIProvider;
use assistant_storage::{default_db_path, StorageLayer};
use axum::{
    response::Redirect,
    routing::{get, post},
    Extension, Router,
};
use clap::Parser;
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

    /// LLM provider to use for chat responses (ollama, anthropic, or openai).
    /// Overrides the provider set in ~/.assistant/config.toml when specified.
    #[arg(long, env = "LLM_PROVIDER")]
    llm_provider: Option<String>,

    /// LLM model name (e.g. "qwen2.5:7b" for Ollama, "claude-sonnet-4-20250514" for Anthropic).
    /// Defaults to the provider's built-in default if not set.
    #[arg(long, env = "LLM_MODEL")]
    llm_model: Option<String>,

    /// Base URL for the LLM provider (mainly for Ollama).
    #[arg(long, env = "OLLAMA_BASE_URL")]
    llm_base_url: Option<String>,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) pool: SqlitePool,
    pub(crate) trace_limit: i64,
    pub(crate) log_limit: i64,
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

    // -- Load assistant config from ~/.assistant/config.toml --------------------
    let file_config = match assistant_core::default_config_path() {
        Some(p) => assistant_core::load_config(&p),
        None => {
            warn!("Cannot determine home directory; using default LLM config");
            assistant_core::AssistantConfig::default()
        }
    };
    let mut llm_config = file_config.llm;

    // CLI args override config file values when explicitly set.
    if let Some(provider) = args.llm_provider {
        llm_config.provider = match provider.to_lowercase().as_str() {
            "anthropic" => LlmProviderKind::Anthropic,
            "openai" => LlmProviderKind::OpenAI,
            _ => LlmProviderKind::Ollama,
        };
    }
    if let Some(model) = args.llm_model {
        llm_config.model = model;
    }
    if let Some(base_url) = args.llm_base_url {
        llm_config.base_url = base_url;
    }

    let llm: Arc<dyn LlmProvider> = match llm_config.provider {
        LlmProviderKind::Ollama => Arc::new(
            OllamaProvider::from_llm_config(&llm_config)
                .context("Failed to create Ollama LLM provider")?,
        ),
        LlmProviderKind::Anthropic => Arc::new(
            AnthropicProvider::from_llm_config(&llm_config)
                .context("Failed to create Anthropic LLM provider")?,
        ),
        LlmProviderKind::OpenAI => Arc::new(
            OpenAIProvider::from_llm_config(&llm_config)
                .context("Failed to create OpenAI LLM provider")?,
        ),
    };

    info!(
        "Chat LLM: provider={}, model={}",
        llm.provider_name(),
        llm.model_name()
    );

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
        llm,
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
        .merge(logs::logs_router())
        .merge(analytics::analytics_router())
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
