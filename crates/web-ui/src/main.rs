mod a2a;
pub mod api;
pub(crate) mod audio_store;
pub mod auth;
pub(crate) mod backends;
mod flutter_assets;
mod oauth;
mod openapi;
pub(crate) mod push;

use std::collections::HashMap;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::LlmProvider;
use assistant_core::{
    BusKind, Interface, LlmProviderKind, MessageBus, OtelExporter, apply_agent_context,
    default_workspace_dir, set_runtime_agent_root, set_runtime_workspace_dir, validate_agent_id,
};
use assistant_runtime::bootstrap::AutoDenyConfirmation;
use assistant_runtime::{Orchestrator, init_tracing};
use assistant_skills::SkillSource;
use assistant_storage::registry::SkillRegistry;
use assistant_storage::{StorageLayer, default_db_path};
use assistant_tool_executor::ToolExecutor;
use assistant_transcription::{build_provider as build_transcription_provider, build_tts_provider};
use assistant_workflow::{
    AssistantTurnActionExecutor, AssistantTurnClient, WorkflowActionExecutor,
    spawn_event_trigger_adapter, spawn_schedule_trigger_adapter, spawn_workflow_runner,
};
use assistant_workflow_http::HttpRequestActionExecutor;
use axum::{
    Extension, Router,
    http::StatusCode,
    routing::{get, post},
};
use backends::{
    IcebergLogBackend, IcebergMetricsBackend, IcebergTraceBackend, LogBackend, MetricsBackend,
    SqliteLogBackend, SqliteMetricsBackend, SqliteTraceBackend, TraceBackend,
};
use clap::Parser;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use utoipa::OpenApi as _;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use auth::WebAuthConfig;

use a2a::agent_store::AgentStore;
use a2a::handlers::{A2AState, build_default_agent_card};
use a2a::task_store::TaskStore;
use api::push::{PushApiState, push_api_router};
use push::PushDispatcher;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Address to listen on (e.g. 127.0.0.1:8080)
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,

    /// Path to the SQLite database (defaults to ~/.assistant/assistant.db)
    #[arg(long)]
    db_path: Option<PathBuf>,

    /// Optional legacy authentication token.  Falls back to ASSISTANT_WEB_TOKEN env var.
    /// When empty or absent the server starts without legacy-token auth.
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

    /// Allowed CORS origin for API routes (e.g. "http://localhost:3000").
    /// Defaults to wildcard (`*`) when not set.
    /// Use `ASSISTANT_WEB_CORS_ORIGIN` env var as an alternative.
    #[arg(long, env = "ASSISTANT_WEB_CORS_ORIGIN")]
    cors_origin: Option<String>,

    /// Print the OpenAPI specification as JSON to stdout and exit.
    /// Use this to regenerate the committed `openapi.json` file:
    ///   `cargo run -p assistant-cli -- webui serve --print-openapi > openapi.json`
    #[arg(long)]
    print_openapi: bool,

    /// OIDC issuer URL (e.g. "https://auth.example.com/realms/main").
    /// When set, the server uses OIDC for authentication instead of password login.
    #[arg(long, env = "OIDC_ISSUER_URL")]
    oidc_issuer_url: Option<String>,

    /// OAuth2 client_id registered with the OIDC provider.
    #[arg(long, env = "OIDC_CLIENT_ID")]
    oidc_client_id: Option<String>,

    /// OAuth2 client_secret for the OIDC provider (confidential clients).
    #[arg(long, env = "OIDC_CLIENT_SECRET")]
    oidc_client_secret: Option<String>,
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
    pub(crate) trace_backend: Arc<dyn TraceBackend>,
    pub(crate) log_backend: Arc<dyn LogBackend>,
    pub(crate) metrics_backend: Arc<dyn MetricsBackend>,
    pub(crate) push_dispatcher: Option<Arc<PushDispatcher>>,
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
    // -- OpenAPI spec dump (no server required) -------------------------------
    if args.print_openapi {
        let spec = openapi::ApiDoc::openapi().to_pretty_json()?;
        println!("{spec}");
        return Ok(());
    }

    // -- Auth token (legacy, optional) -----------------------------------------
    let legacy_token = args
        .auth_token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());

    // Parse listen address early so we can determine cookie security.
    let addr: SocketAddr = args.listen.parse()?;
    let secure_cookie = !addr.ip().is_loopback() && !args.no_secure_cookie;

    let db_path = match args.db_path.or_else(default_db_path) {
        Some(p) => p,
        None => anyhow::bail!("Cannot determine default DB path. Specify --db-path."),
    };

    // Before opening databases, check for a legacy single-user layout and
    // auto-migrate to the new org/space directory structure.  The migration
    // must run before StorageLayer opens `assistant.db` so the backup gets a
    // clean WAL checkpoint without contention.
    let base_path = db_path.parent().unwrap_or(Path::new("."));
    if assistant_storage::migration::is_legacy_layout(base_path) {
        info!("detected legacy single-user layout — running auto-migration");
        let backup = assistant_storage::migration::run_migration(base_path)
            .await
            .context("legacy → multi-org auto-migration failed")?;
        info!("migration complete — backup at {}", backup.display());
    }

    let storage = Arc::new(StorageLayer::new(&db_path).await?);

    // -- Org-level storage (users, spaces, auth state) --------------------------
    let pool_factory = assistant_storage::OrgPoolFactory::new(base_path);
    let org_storage = Arc::new(
        pool_factory
            .org_storage()
            .await
            .context("Failed to open org.db")?,
    );

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
            "openrouter" => LlmProviderKind::OpenRouter,
            other => anyhow::bail!(
                "Unsupported --llm-provider value: {other}. \
                 Expected one of: ollama, anthropic, openai, moonshot, openrouter."
            ),
        };
    }
    if let Some(model) = args.llm_model {
        config.llm.model = model;
    }
    if let Some(base_url) = args.llm_base_url {
        config.llm.base_url = base_url;
    }

    let _otel_guard = init_tracing(storage.pool.clone(), &config.observability).await?;

    // -- VAPID key provisioning (once, at startup) ---------------------------
    //
    // If `[notifications]` is absent or incomplete, generate a new P-256 key
    // pair and write it back to the config file.  This is done before anything
    // else so all subsequent handlers can read stable VAPID keys.
    let config_path = assistant_core::default_config_path();
    let (vapid_private_key, vapid_public_key) = if let Some(ref path) = config_path {
        push::ensure_vapid_keys(
            path,
            config.notifications.vapid_private_key.as_deref(),
            config.notifications.vapid_public_key.as_deref(),
        )
        .await
        .unwrap_or_else(|e| {
            warn!("VAPID key provisioning failed: {e}; push notifications will be unavailable");
            (String::new(), String::new())
        })
    } else {
        warn!("Cannot determine config path; VAPID key provisioning skipped");
        (String::new(), String::new())
    };

    let cli_agent_override = args.agent.clone();
    let selected_agent = cli_agent_override
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

    let (trace_backend, log_backend, metrics_backend): (
        Arc<dyn TraceBackend>,
        Arc<dyn LogBackend>,
        Arc<dyn MetricsBackend>,
    ) = match config.observability.exporter {
        OtelExporter::Iceberg => {
            let iceberg_cfg = config.observability.iceberg.clone();
            (
                Arc::new(IcebergTraceBackend::new(iceberg_cfg.clone())),
                Arc::new(IcebergLogBackend::new(iceberg_cfg.clone())),
                Arc::new(IcebergMetricsBackend::new(iceberg_cfg)),
            )
        }
        _ => (
            Arc::new(SqliteTraceBackend::new(storage.pool.clone())),
            Arc::new(SqliteLogBackend::new(storage.pool.clone())),
            Arc::new(SqliteMetricsBackend::new(storage.pool.clone())),
        ),
    };

    // -- Push dispatcher (built here so AppState can hold a reference) -------
    let push_store_for_state = Arc::new(storage.push_subscription_store());
    let push_dispatcher_for_state = if !vapid_private_key.is_empty() {
        Some(Arc::new(PushDispatcher::new(
            vapid_private_key.clone(),
            push_store_for_state.clone(),
        )))
    } else {
        None
    };

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
        trace_backend,
        log_backend,
        metrics_backend,
        push_dispatcher: push_dispatcher_for_state,
    };

    // 2. LLM provider
    let llm: Arc<dyn LlmProvider> = assistant_llm_provider::create_provider(&config.llm)
        .context("Failed to create LLM provider")?;

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
    let persona_timeout = storage
        .persona_store()
        .get(&config.agent.id)
        .await
        .ok()
        .flatten()
        .and_then(|p| p.turn_timeout_secs);

    // Create the audio store early so the orchestrator can reference it.
    let audio_store = Arc::new(audio_store::AudioStore::new());

    let orchestrator = Arc::new({
        let mut o = Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry,
            bus.clone(),
            &config,
        )
        .with_confirmation_callback(Arc::new(AutoDenyConfirmation {
            interface_name: "Web",
        }))
        .with_audio_store(audio_store.clone());
        if let Some(secs) = persona_timeout {
            o = o.with_submit_timeout(secs);
        }
        o
    });

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
    // Clone for the agents REST API — AgentStore is Clone (wraps a PathBuf).
    let agents_api_store = agent_store.clone();

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

    let workflows_api_state = api::workflows::WorkflowsApiState {
        pool: storage.pool.clone(),
        agent_id: state.agent_id.clone(),
    };

    // -- Build transcription provider (for voice message STT) ----------------
    let transcription_provider = config
        .transcription
        .as_ref()
        .map(|tc| {
            let provider = build_transcription_provider(tc)?;
            info!(
                provider = provider.name(),
                "Audio transcription enabled (web UI)"
            );
            Ok::<_, anyhow::Error>(provider)
        })
        .transpose()?;

    // -- Build TTS provider (for voice message playback) ---------------------
    let tts_provider = config
        .tts
        .as_ref()
        .map(|tc| {
            let provider = build_tts_provider(tc)?;
            info!(provider = provider.name(), "TTS enabled (web UI)");
            Ok::<_, anyhow::Error>(provider)
        })
        .transpose()?;

    // -- Spawn TTL sweep task ------------------------------------------------
    {
        let store = audio_store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(600));
            loop {
                interval.tick().await;
                store.sweep().await;
            }
        });
    }

    // Register the voice-response tool when TTS is configured.
    if let Some(ref tts) = tts_provider {
        executor.register_voice_response(tts.clone(), audio_store.clone());
        tracing::info!("voice-response tool registered");
    }

    let api_state = {
        let orchestrator_ref = orchestrator.clone();
        let mut s = api::ApiState::new(
            storage.pool.clone(),
            orchestrator,
            state.agent_id.clone(),
            orchestrator_ref,
        );
        if let Some(ref d) = state.push_dispatcher {
            s = s.with_push_dispatcher(d.clone());
        }
        if let Some(ref tp) = transcription_provider {
            s = s.with_transcription_provider(tp.clone());
        }
        if let Some(ref tts) = tts_provider {
            s = s.with_tts_provider(tts.clone());
        }
        s.audio_store = audio_store.clone();
        s
    };

    let persona_api_state = api::personas::PersonaApiState {
        pool: storage.pool.clone(),
        agent_id: state.agent_id.clone(),
    };

    let traces_api_state = api::traces::TracesApiState {
        trace_backend: state.trace_backend.clone(),
    };

    let logs_api_state = api::logs::LogsApiState {
        log_backend: state.log_backend.clone(),
    };

    let skills_api_state = api::skills::SkillsApiState {
        pool: storage.pool.clone(),
        registry: state.registry.clone(),
    };

    let webhooks_api_state = api::webhooks::WebhooksApiState {
        pool: storage.pool.clone(),
        agent_id: state.agent_id.clone(),
    };

    let agents_api_state = api::agents::AgentsApiState {
        agent_store: agents_api_store,
    };

    let analytics_api_state = api::analytics::AnalyticsApiState {
        metrics_backend: state.metrics_backend.clone(),
        agent_id: state.agent_id.clone(),
    };

    // Only mount the push API when a real VAPID keypair was provisioned.
    // An empty key means key generation failed; exposing the endpoints in that
    // state would let clients subscribe but never receive any delivery.
    let push_api_state = (!vapid_public_key.is_empty()).then(|| PushApiState {
        store: push_store_for_state,
        vapid_public_key: Arc::new(vapid_public_key),
    });

    // -- JWT + OAuth2 state ---------------------------------------------------
    let jwt_manager = {
        use assistant_auth::jwt::{JwtKeyPair, JwtManager};
        let jwt_secret_path = db_path.with_file_name("jwt_secret");
        let jwt_key_pair = JwtKeyPair::load_or_generate(&jwt_secret_path)
            .context("Failed to load or generate JWT signing key")?;
        Arc::new(JwtManager::new(
            jwt_key_pair,
            base_url.clone(),
            base_url.clone(),
        ))
    };

    let oauth_state = {
        use assistant_auth::oauth2::clients::ClientRegistrar;
        use assistant_auth::oauth2::device::DeviceCodeManager;
        use assistant_auth::oauth2::server::OAuth2Server;

        let oauth2_server = Arc::new(OAuth2Server::new(
            Arc::new(org_storage.auth_code_store()),
            Arc::new(org_storage.refresh_token_store()),
        ));

        let client_registrar = Arc::new(ClientRegistrar::new(Arc::new(org_storage.client_store())));

        let device_manager = Arc::new(DeviceCodeManager::new(
            Arc::new(org_storage.device_code_store()),
            format!("{base_url}/oauth/device/verify"),
        ));

        // Optionally discover an OIDC provider.
        let (oidc_provider, oidc_sessions) = if let Some(ref issuer_url) = args.oidc_issuer_url {
            let oidc_client_id = args
                .oidc_client_id
                .clone()
                .unwrap_or_else(|| "assistant".to_string());
            let config = assistant_auth::oidc::OidcConfig {
                issuer_url: issuer_url.clone(),
                client_id: oidc_client_id,
                client_secret: args.oidc_client_secret.clone(),
                auto_provision: true,
                allowed_email_domains: None,
            };
            let provider = assistant_auth::oidc::OidcProvider::discover(config)
                .await
                .context("Failed to discover OIDC provider")?;
            info!(issuer = %issuer_url, "OIDC provider discovered");
            let sessions = Arc::new(oauth::oidc_sessions::PendingOidcSessionStore::new(
                std::time::Duration::from_secs(300),
            ));
            (Some(Arc::new(provider)), Some(sessions))
        } else {
            (None, None)
        };

        oauth::OAuthState {
            oauth2_server,
            jwt_manager: jwt_manager.clone(),
            client_registrar,
            device_manager,
            org_storage: org_storage.clone(),
            issuer: base_url.clone(),
            secure_cookie,
            oidc_provider,
            oidc_sessions,
        }
    };

    // -- Auth config (JWT + API key + legacy token) --------------------------
    let legacy_context = legacy_token.as_ref().map(|_| {
        use assistant_core::auth::AuthContext;
        use assistant_core::identity::{Action, OrgId, ResourceKind, Role, Scope, SpaceId, UserId};
        let mut space_roles = std::collections::HashMap::new();
        space_roles.insert(SpaceId::from("default"), Role::OrgAdmin);
        AuthContext {
            user_id: UserId::from("admin"),
            org_id: OrgId::from("default"),
            email: String::new(),
            space_roles,
            scopes: vec![Scope::new(ResourceKind::Org, Action::Manage)],
            client_id: "legacy".into(),
        }
    });
    let api_key_store: Arc<dyn assistant_auth::api_keys::ApiKeyStore> =
        Arc::new(org_storage.api_key_store());
    let auth_config = WebAuthConfig::new(
        jwt_manager,
        api_key_store.clone(),
        legacy_token,
        legacy_context,
        secure_cookie,
    );

    // -- Management API states (orgs, users, spaces, members, API keys) ------
    let orgs_api_state = api::orgs::OrgsApiState {
        org_storage: org_storage.clone(),
    };
    let users_api_state = api::users::UsersApiState {
        org_storage: org_storage.clone(),
    };
    let spaces_api_state = api::spaces::SpacesApiState {
        org_storage: org_storage.clone(),
    };
    let members_api_state = api::members::MembersApiState {
        org_storage: org_storage.clone(),
    };
    let api_keys_api_state = api::api_keys::ApiKeysApiState { api_key_store };
    let catalog_api_state = api::catalog::CatalogApiState {
        org_storage: org_storage.clone(),
    };
    let interfaces_api_state = api::interfaces::InterfacesApiState {
        org_storage: org_storage.clone(),
    };
    let bindings_api_state = api::bindings::BindingsApiState {
        org_storage: org_storage.clone(),
    };
    let templates_api_state = api::templates::TemplatesApiState {
        org_storage: org_storage.clone(),
        pool: storage.pool.clone(),
    };

    // -- Router: public routes (no auth required) --------------------------
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/login", get(auth::login_page).post(auth::login_submit))
        .route("/logout", post(auth::logout))
        .with_state(auth_config.clone())
        // OpenAPI spec + Swagger UI (public — clients need the spec to discover auth).
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", openapi::ApiDoc::openapi()))
        // Public workflow webhook trigger ingress.
        .merge(api::workflows::workflow_public_router().with_state(workflows_api_state.clone()))
        // A2A agent card is public per spec — callers need it to discover auth.
        .merge(a2a::public_router().with_state(a2a_state.clone()))
        // OAuth2 endpoints (public — clients need these for auth flows).
        .merge(oauth::oauth_router().with_state(oauth_state));

    // -- Router: protected routes (auth required) --------------------------
    let protected_routes = Router::new()
        .with_state(state)
        // A2A protocol routes (auth-protected endpoints only).
        .merge(a2a::protected_router().with_state(a2a_state))
        // REST API — all page routes are now handled by the Flutter SPA fallback.
        .nest(
            "/api",
            api::api_router()
                .with_state(api_state)
                .merge(api::personas::personas_router().with_state(persona_api_state))
                .merge(api::traces::traces_router().with_state(traces_api_state))
                .merge(api::logs::logs_router().with_state(logs_api_state))
                .merge(api::skills::skills_router().with_state(skills_api_state))
                .merge(api::webhooks::webhooks_api_router().with_state(webhooks_api_state))
                .merge(api::agents::agents_api_router().with_state(agents_api_state))
                .merge(api::analytics::analytics_api_router().with_state(analytics_api_state))
                .merge(api::workflows::workflows_api_router().with_state(workflows_api_state))
                .merge(api::orgs::orgs_api_router().with_state(orgs_api_state))
                .merge(api::users::users_api_router().with_state(users_api_state))
                .merge(api::spaces::spaces_api_router().with_state(spaces_api_state))
                .merge(api::members::members_api_router().with_state(members_api_state))
                .merge(api::api_keys::api_keys_router().with_state(api_keys_api_state))
                .merge(api::catalog::catalog_api_router().with_state(catalog_api_state))
                .merge(api::interfaces::interfaces_api_router().with_state(interfaces_api_state))
                .merge(api::bindings::bindings_api_router().with_state(bindings_api_state))
                .merge(api::templates::templates_api_router().with_state(templates_api_state))
                .merge(
                    push_api_state
                        .map(|s| push_api_router().with_state(s))
                        .unwrap_or_else(Router::new),
                ),
        )
        .route_layer(axum::middleware::from_fn(
            auth::require_same_origin_mutation,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            auth_config,
            auth::require_auth,
        ));

    // -- CORS layer for /api/* routes ----------------------------------------
    //
    // The Flutter web build runs at the same origin as the server (embedded),
    // but the macOS desktop app and external clients make cross-origin requests.
    // We emit permissive CORS headers on all /api/* routes.
    let cors = if let Some(ref origin) = args.cors_origin {
        let origin_val = match origin.parse::<axum::http::HeaderValue>() {
            Ok(v) => v,
            Err(e) => {
                warn!("Invalid --cors-origin value {origin:?}: {e}; falling back to wildcard");
                axum::http::HeaderValue::from_static("*")
            }
        };
        CorsLayer::new()
            .allow_origin(origin_val)
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
                axum::http::header::ACCEPT,
            ])
            .allow_methods(Any)
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
                axum::http::header::ACCEPT,
            ])
            .allow_methods(Any)
    };

    let router = public_routes
        .merge(protected_routes)
        // Flutter SPA fallback: serves embedded web assets for all unmatched
        // paths (no auth required — FR-013).
        .fallback(flutter_assets::flutter_handler)
        .layer(cors)
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
    info!("Authentication enabled — enter token at http://{}/", addr);
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
