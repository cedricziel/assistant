mod a2a;
mod analytics;
pub mod auth;
mod chat;
pub mod common;
mod contexts;
mod logs;
mod pwa;
mod skills;
pub(crate) mod static_assets;
mod traces;
mod webhooks;
mod workflows;

use std::collections::HashMap;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::{
    apply_agent_context, default_workspace_dir, set_runtime_agent_root, set_runtime_workspace_dir,
    validate_agent_id, BusKind, Interface, LlmProviderKind, MessageBus,
};
use assistant_llm::LlmProvider;
use assistant_provider_anthropic::AnthropicProvider;
use assistant_provider_moonshot::MoonshotProvider;
use assistant_provider_ollama::OllamaProvider;
use assistant_provider_openai::OpenAIProvider;
use assistant_runtime::bootstrap::AutoDenyConfirmation;
use assistant_runtime::{init_tracing, Orchestrator};
use assistant_skills::SkillSource;
use assistant_storage::registry::SkillRegistry;
use assistant_storage::{default_db_path, StorageLayer};
use assistant_tool_executor::ToolExecutor;
use assistant_workflow::{
    spawn_event_trigger_adapter, spawn_schedule_trigger_adapter, spawn_workflow_runner,
    AssistantTurnActionExecutor, AssistantTurnClient, WorkflowActionExecutor,
};
use assistant_workflow_http::HttpRequestActionExecutor;
use axum::{
    http::StatusCode,
    response::Redirect,
    routing::{get, post},
    Extension, Router,
};
use clap::Parser;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use uuid::Uuid;

use auth::AuthConfig;

use a2a::agent_store::AgentStore;
use a2a::handlers::{build_default_agent_card, A2AState};
use a2a::pages::AgentPagesState;
use a2a::task_store::TaskStore;

#[derive(Parser, Debug)]
#[command(version)]
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

    /// Assistant agent context ID (e.g. "default", "work", "personal").
    #[arg(long, env = "ASSISTANT_AGENT")]
    agent: Option<String>,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) pool: SqlitePool,
    pub(crate) agent_id: Arc<RwLock<String>>,
    pub(crate) registry: Arc<SkillRegistry>,
    pub(crate) trace_limit: i64,
    pub(crate) log_limit: i64,
    pub(crate) bus_kind: BusKind,
    pub(crate) nats_url: Option<String>,
    pub(crate) nats_token: Option<String>,
}

struct OrchestratorTurnClient {
    orchestrator: Arc<Orchestrator>,
}

#[async_trait::async_trait]
impl AssistantTurnClient for OrchestratorTurnClient {
    async fn submit_turn(&self, prompt: &str, conversation_id: Uuid) -> Result<String> {
        let result = self
            .orchestrator
            .submit_turn(prompt, conversation_id, Interface::Scheduler, None)
            .await?;
        Ok(result.answer)
    }
}

async fn run_with_args(args: Args) -> Result<()> {
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

    let storage = Arc::new(StorageLayer::new(&db_path).await?);

    // -- Load assistant config from ~/.assistant/config.toml --------------------
    let mut config = match assistant_core::default_config_path() {
        Some(p) => assistant_core::load_config(&p),
        None => {
            warn!("Cannot determine home directory; using default LLM config");
            assistant_core::AssistantConfig::default()
        }
    };

    // CLI args override config file values when explicitly set.
    if let Some(provider) = args.llm_provider {
        config.llm.provider = match provider.to_lowercase().as_str() {
            "ollama" => LlmProviderKind::Ollama,
            "anthropic" => LlmProviderKind::Anthropic,
            "openai" => LlmProviderKind::OpenAI,
            "moonshot" => LlmProviderKind::Moonshot,
            other => anyhow::bail!(
                "Unsupported --llm-provider value: {other}. \
                 Expected one of: ollama, anthropic, openai, moonshot."
            ),
        };
    }
    if let Some(model) = args.llm_model {
        config.llm.model = model;
    }
    if let Some(base_url) = args.llm_base_url {
        config.llm.base_url = base_url;
    }

    let _otel_guard = init_tracing(storage.pool.clone(), config.mirror.trace_enabled)?;

    let cli_agent_override = args.agent.clone();
    let mut selected_agent = cli_agent_override
        .clone()
        .unwrap_or_else(|| config.agent.id.clone());
    if !validate_agent_id(&selected_agent) {
        anyhow::bail!(
            "Invalid agent ID '{}'. Use only letters, numbers, '-' and '_'.",
            selected_agent
        );
    }
    apply_agent_context(&mut config, &selected_agent);
    if let Some(home) = dirs::home_dir() {
        let agent_root = home.join(".assistant").join("agents").join(&selected_agent);
        set_runtime_agent_root(agent_root);
    }
    let workspace_dir = default_workspace_dir(&selected_agent);
    set_runtime_workspace_dir(workspace_dir.clone());
    tokio::fs::create_dir_all(&workspace_dir)
        .await
        .with_context(|| format!("Failed to create workspace at {}", workspace_dir.display()))?;

    let personas = storage.persona_store();
    personas.ensure_default().await?;
    if cli_agent_override.is_none() {
        let default_id = personas.default_id().await?;
        selected_agent = default_id;
        apply_agent_context(&mut config, &selected_agent);
        if let Some(home) = dirs::home_dir() {
            let agent_root = home.join(".assistant").join("agents").join(&selected_agent);
            set_runtime_agent_root(agent_root);
        }
        let workspace_dir = default_workspace_dir(&selected_agent);
        set_runtime_workspace_dir(workspace_dir.clone());
        tokio::fs::create_dir_all(&workspace_dir)
            .await
            .with_context(|| {
                format!("Failed to create workspace at {}", workspace_dir.display())
            })?;
    }
    personas.ensure_exists(&selected_agent).await?;

    // -- Build the full orchestrator chain -----------------------------------
    //
    // The web UI MUST route chat messages through the Orchestrator so the
    // assistant gets the same system prompt, tools, skills, memory, and ReAct
    // loop as every other interface (CLI, Slack, etc.).
    //
    // See skills/interface-implementation/SKILL.md for the canonical checklist.

    // 1. Skill registry
    let registry = SkillRegistry::new(storage.pool.clone())
        .await
        .context("Failed to create skill registry")?;

    let project_root = std::env::current_dir().ok();
    let dirs_to_scan = assistant_runtime::bootstrap::skill_dirs(&config, project_root.as_deref());
    let dirs_ref: Vec<(&std::path::Path, SkillSource)> = dirs_to_scan
        .iter()
        .map(|(p, s)| (p.as_path(), s.clone()))
        .collect();

    registry
        .load_embedded()
        .await
        .context("Failed to load embedded builtin skills")?;

    if let Some(home) = dirs::home_dir() {
        let builtin_target = home.join(".assistant").join("skills");
        match registry.sync_builtins_to_disk(&builtin_target) {
            Ok(updated) if !updated.is_empty() => {
                tracing::info!(
                    "Synced {} built-in skill(s) to disk: {}",
                    updated.len(),
                    updated.join(", ")
                );
            }
            Err(e) => {
                tracing::warn!("Failed to sync built-in skills to disk: {e}");
            }
            _ => {}
        }
    }

    registry
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills from directories")?;

    let registry = Arc::new(registry);

    let state = AppState {
        pool: storage.pool.clone(),
        agent_id: Arc::new(RwLock::new(selected_agent.clone())),
        registry: registry.clone(),
        trace_limit: args.trace_limit,
        log_limit: args.log_limit,
        bus_kind: config.bus.kind.clone(),
        nats_url: config
            .bus
            .nats_url
            .clone()
            .or_else(|| std::env::var("NATS_URL").ok()),
        nats_token: config
            .bus
            .token
            .clone()
            .or_else(|| std::env::var("NATS_TOKEN").ok()),
    };

    // 2. LLM provider
    let llm: Arc<dyn LlmProvider> = match config.llm.provider {
        LlmProviderKind::Ollama => Arc::new(
            OllamaProvider::from_llm_config(&config.llm)
                .context("Failed to create Ollama LLM provider")?,
        ),
        LlmProviderKind::Anthropic => Arc::new(
            AnthropicProvider::from_llm_config(&config.llm)
                .context("Failed to create Anthropic LLM provider")?,
        ),
        LlmProviderKind::OpenAI => Arc::new(
            OpenAIProvider::from_llm_config(&config.llm)
                .context("Failed to create OpenAI LLM provider")?,
        ),
        LlmProviderKind::Moonshot => Arc::new(
            MoonshotProvider::from_llm_config(&config.llm)
                .context("Failed to create Moonshot LLM provider")?,
        ),
    };

    info!(
        "Chat LLM: provider={}, model={}",
        llm.provider_name(),
        llm.model_name()
    );

    // 3. Tool executor
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    // 4. Message bus + Orchestrator
    let bus: Arc<dyn MessageBus> = {
        #[cfg(feature = "nats")]
        {
            use assistant_core::BusKind;
            if config.bus.kind == BusKind::Nats {
                tracing::info!("Using NATS message bus");
                Arc::new(
                    assistant_bus_nats::NatsMessageBus::connect(&config.bus)
                        .await
                        .context("failed to connect to NATS")?,
                )
            } else {
                Arc::new(storage.message_bus())
            }
        }
        #[cfg(not(feature = "nats"))]
        {
            use assistant_core::BusKind;
            if config.bus.kind == BusKind::Nats {
                anyhow::bail!(
                    "[bus] kind = \"nats\" configured but this binary was built without the `nats` feature"
                );
            }
            Arc::new(storage.message_bus())
        }
    };
    let orchestrator = Arc::new(
        Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry,
            bus.clone(),
            &config,
        )
        .with_confirmation_callback(Arc::new(AutoDenyConfirmation {
            interface_name: "Web",
        })),
    );

    // Wire up subagent support (breaks the init-time circular dep).
    executor.set_subagent_runner(orchestrator.clone());

    // Connect to configured external MCP servers and register their tools.
    if !config.mcp.servers.is_empty() {
        let mcp_manager =
            Arc::new(assistant_mcp_client::McpClientManager::start(&config.mcp.servers).await?);
        let mcp_tools = mcp_manager.tool_handlers().await;
        for handler in &mcp_tools {
            executor.register_ambient_tool(handler.clone());
        }
        tracing::info!(
            servers = mcp_manager.server_count().await,
            tools = mcp_tools.len(),
            "registered MCP client tools"
        );

        // Spawn background health-check loop for reconnection.
        let exec_register = executor.clone();
        let exec_unregister = executor.clone();
        mcp_manager.spawn_health_loop(
            Arc::new(move |h| exec_register.register_ambient_tool(h)),
            Arc::new(move |prefix| exec_unregister.unregister_tools_by_prefix(prefix)),
        );
    }

    // 5. Spawn the turn-processing worker (scoped to Web interface only,
    //    so it doesn't steal turns from Slack/Mattermost workers sharing
    //    the same SQLite database).
    let worker_orch = orchestrator.clone();
    tokio::spawn(async move {
        worker_orch
            .run_worker_filtered("web-worker", Some("Web"))
            .await;
    });

    // 6. Spawn workflow run processor (loop guardrails + action executors).
    let turn_client = Arc::new(OrchestratorTurnClient {
        orchestrator: orchestrator.clone(),
    });
    let action_executors: Vec<Arc<dyn WorkflowActionExecutor>> = vec![
        Arc::new(AssistantTurnActionExecutor::new(turn_client)),
        Arc::new(HttpRequestActionExecutor::default()),
    ];
    let _workflow_runner =
        spawn_workflow_runner(storage.clone(), Duration::from_secs(2), action_executors);
    let _workflow_schedule_adapter =
        spawn_schedule_trigger_adapter(storage.clone(), Duration::from_secs(2));
    let _workflow_event_adapter =
        spawn_event_trigger_adapter(storage.clone(), bus.clone(), Duration::from_secs(2));

    // -- Persona-scoped A2A profile store (filesystem-backed) --
    let agent_store = AgentStore::for_persona(&selected_agent)?;

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
        agent_id: state.agent_id.clone(),
    };

    let workflow_pages_state = workflows::pages::WorkflowPagesState {
        pool: storage.pool.clone(),
        agent_id: state.agent_id.clone(),
    };

    let skills_pages_state = skills::pages::SkillsPagesState {
        registry: state.registry.clone(),
        orchestrator: orchestrator.clone(),
    };

    let chat_state =
        chat::ChatState::new(storage.pool.clone(), orchestrator, selected_agent.clone());

    // -- Router: public routes (no auth required) --------------------------
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/login", get(auth::login_page).post(auth::login_submit))
        .route("/logout", post(auth::logout))
        // Public workflow webhook trigger ingress.
        .merge(workflows::workflow_public_router().with_state(workflow_pages_state.clone()))
        // A2A agent card is public per spec — callers need it to discover auth.
        .merge(a2a::public_router().with_state(a2a_state.clone()))
        // PWA assets must be public so the browser can fetch them before auth.
        .merge(pwa::pwa_router())
        // Fingerprinted static assets (CSS).
        .merge(static_assets::static_router());

    // -- Router: protected routes (auth required) --------------------------
    let protected_routes = Router::new()
        // Trace / log UI routes.
        .route("/", get(|| async { Redirect::to("/chat") }))
        .merge(traces::traces_router())
        .merge(logs::logs_router())
        .merge(analytics::analytics_router())
        .merge(contexts::contexts_router())
        .with_state(state)
        // A2A protocol routes (auth-protected endpoints only).
        .merge(a2a::protected_router().with_state(a2a_state))
        // Agent management UI pages.
        .merge(a2a::agent_pages_router().with_state(agent_pages_state))
        // Webhook management UI pages.
        .merge(webhooks::webhook_pages_router().with_state(webhook_pages_state))
        // Workflow graph management pages + JSON API.
        .merge(workflows::workflow_pages_router().with_state(workflow_pages_state))
        // Skill management pages.
        .merge(skills::skills_router().with_state(skills_pages_state))
        // Chat interface.
        .merge(chat::chat_router().with_state(chat_state))
        .route_layer(axum::middleware::from_fn(
            auth::require_same_origin_mutation,
        ))
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

pub async fn run_from_env() -> Result<()> {
    let args = Args::parse();
    run_with_args(args).await
}

pub async fn run_from_iter<I, T>(iter: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = Args::try_parse_from(iter).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    run_with_args(args).await
}

#[tokio::main]
async fn main() -> Result<()> {
    run_from_env().await
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn ready(Extension(state): Extension<AppState>) -> StatusCode {
    if sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_err()
    {
        return StatusCode::SERVICE_UNAVAILABLE;
    }

    if state.bus_kind == BusKind::Nats {
        let Some(url) = state.nats_url.as_deref() else {
            return StatusCode::SERVICE_UNAVAILABLE;
        };

        if !nats_reachable(url, state.nats_token.as_deref()).await {
            return StatusCode::SERVICE_UNAVAILABLE;
        }
    }

    StatusCode::OK
}

async fn nats_reachable(url: &str, token: Option<&str>) -> bool {
    let endpoint = url.strip_prefix("nats://").unwrap_or(url);
    let (url_auth, endpoint) = match endpoint.rsplit_once('@') {
        Some((auth, host)) => (Some(auth), host),
        None => (None, endpoint),
    };

    let (host, port) = match endpoint.rsplit_once(':') {
        Some((h, p)) => match p.parse::<u16>() {
            Ok(parsed) => (h, parsed),
            Err(_) => (endpoint, 4222),
        },
        None => (endpoint, 4222),
    };

    let mut stream = match tokio::time::timeout(
        Duration::from_secs(1),
        tokio::net::TcpStream::connect((host, port)),
    )
    .await
    {
        Ok(Ok(stream)) => stream,
        _ => return false,
    };

    let mut initial = vec![0_u8; 1024];
    if tokio::time::timeout(Duration::from_secs(1), stream.read(&mut initial))
        .await
        .is_err()
    {
        return false;
    }

    let mut connect = json!({
        "lang": "rust",
        "version": "web-ui-readyz",
        "protocol": 1,
        "verbose": false,
        "pedantic": false,
        "tls_required": false,
    });

    if let Some(token) = token.filter(|t| !t.is_empty()) {
        connect["auth_token"] = json!(token);
    } else if let Some(auth) = url_auth {
        if let Some((user, pass)) = auth.split_once(':') {
            connect["user"] = json!(user);
            connect["pass"] = json!(pass);
        } else if !auth.is_empty() {
            connect["auth_token"] = json!(auth);
        }
    }

    let line = format!("CONNECT {}\r\nPING\r\n", connect);
    if tokio::time::timeout(Duration::from_secs(1), stream.write_all(line.as_bytes()))
        .await
        .is_err()
    {
        return false;
    }

    for _ in 0..3 {
        let mut buf = vec![0_u8; 1024];
        let n = match tokio::time::timeout(Duration::from_secs(1), stream.read(&mut buf)).await {
            Ok(Ok(n)) if n > 0 => n,
            _ => return false,
        };
        let msg = String::from_utf8_lossy(&buf[..n]);
        if msg.contains("PONG") {
            return true;
        }
        if msg.contains("-ERR") {
            return false;
        }
    }

    false
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
