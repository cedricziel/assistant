mod a2a;
pub mod api;
pub(crate) mod audio_store;
pub mod auth;
pub mod catalog_sync;
mod cors;
pub(crate) mod errors;
mod flutter_assets;
pub mod install;
mod mcp;
mod oauth;
mod openapi;
pub(crate) mod push;
pub(crate) mod rate_limit;
pub mod space;
mod space_tasks;
pub mod sw_version;

use std::collections::HashMap;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::types::llm::LlmProviderKind;
use assistant_core::types::storage::BusKind;
use assistant_core::{
    MessageBus, agent_base_dir, apply_agent_context, default_workspace_dir, load_server_config,
    server_config_path, set_runtime_agent_root, set_runtime_install_root,
    set_runtime_workspace_dir, validate_agent_id,
};
use assistant_mcp_server::{ListChangeListeners, TaskStore};
use assistant_runtime::{OrgContexts, SpaceRuntimes, SpaceSupervisor, init_tracing};
use assistant_transcription::{build_provider as build_transcription_provider, build_tts_provider};
use axum::{Extension, Router, http::StatusCode, routing::get};
use clap::Parser;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use utoipa::OpenApi as _;
use utoipa_swagger_ui::SwaggerUi;

use auth::WebAuthConfig;

use a2a::agent_store::AgentStore;
use a2a::handlers::{A2AState, build_default_agent_card};
use api::push::{PushApiState, push_api_router};
use push::PushDispatcher;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Address to listen on (e.g. 127.0.0.1:8080)
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,

    /// Public URL the server is reachable at, used as the OAuth issuer and
    /// in the A2A agent card. Behind a reverse proxy (e.g. Pangolin), set
    /// this to the proxy's external URL so OAuth metadata advertises
    /// reachable endpoints. Falls back to `http://{listen}` when unset.
    #[arg(long, env = "ASSISTANT_PUBLIC_URL")]
    public_url: Option<String>,

    /// Optional legacy authentication token.  Falls back to ASSISTANT_WEB_TOKEN env var.
    /// When empty or absent the server starts without legacy-token auth.
    #[arg(long, env = "ASSISTANT_WEB_TOKEN")]
    auth_token: Option<String>,

    /// Disable the `Secure` attribute on session cookies.
    /// Useful when running behind a VPN or firewall over plain HTTP on a
    /// non-loopback address.  Without this flag, binding to a non-loopback
    /// address automatically sets `Secure`, which requires HTTPS.
    #[arg(long)]
    no_secure_cookie: bool,

    /// LLM provider to use for chat responses (ollama, anthropic, or openai).
    /// Overrides the provider set in each org's org.toml when specified.
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

    /// Allowed CORS origins (e.g. "http://localhost:3000"); repeat the flag or
    /// comma-separate `ASSISTANT_WEB_CORS_ORIGIN`. When unset, no CORS headers
    /// are sent and browsers only reach the API same-origin. An invalid origin
    /// aborts startup.
    #[arg(long, env = "ASSISTANT_WEB_CORS_ORIGIN", value_delimiter = ',')]
    cors_origin: Vec<String>,

    /// Print the OpenAPI specification as JSON to stdout and exit.
    /// Use this to regenerate the committed `openapi.json` file:
    ///   `cargo run -p assistant-cli -- webui serve --print-openapi > openapi.json`
    #[arg(long)]
    print_openapi: bool,

    #[command(flatten)]
    rate_limit: rate_limit::RateLimitArgs,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) pool: SqlitePool,
    pub(crate) agent_id: Arc<RwLock<String>>,
    pub(crate) bus_kind: BusKind,
    pub(crate) nats_url: Option<String>,
    pub(crate) nats_token: Option<String>,
    pub(crate) push_dispatcher: Option<Arc<PushDispatcher>>,
}

/// How often the server looks for spaces that were created or deleted.
const SPACE_RECONCILE_INTERVAL: Duration = Duration::from_secs(30);

/// The broker shared by every space, when `[bus] kind = "nats"`; otherwise each
/// space uses the SQLite bus in its own database.
async fn shared_nats_bus(
    config: &assistant_core::types::agent::ServerConfig,
) -> Result<Option<Arc<dyn MessageBus>>> {
    if config.bus.kind != BusKind::Nats {
        return Ok(None);
    }
    #[cfg(feature = "nats")]
    {
        tracing::info!("Using NATS message bus");
        let bus = assistant_bus_nats::NatsMessageBus::connect(&config.bus)
            .await
            .context("failed to connect to NATS")?;
        Ok(Some(Arc::new(bus)))
    }
    #[cfg(not(feature = "nats"))]
    {
        anyhow::bail!(
            "[bus] kind = \"nats\" configured but this binary was built without the `nats` feature"
        )
    }
}

/// This process's space runtimes. The web-ui process runs no turns in the
/// packaged install, but its respond route and replay lookups need each
/// space's interaction store and the in-process fast path, so interactions
/// are on here too (ADR-0017, D2, D9).
fn web_runtimes(
    pool_factory: Arc<assistant_storage::OrgPoolFactory>,
    orgs: Arc<OrgContexts>,
    audio_store: Arc<audio_store::AudioStore>,
) -> SpaceRuntimes {
    SpaceRuntimes::new(pool_factory, orgs, None)
        .with_interactions()
        .with_audio_store(audio_store)
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
    let rate_limits = Arc::new(rate_limit::RateLimits::new(
        Arc::new(assistant_core::clock::SystemClock),
        &args.rate_limit,
    ));

    // The install root is always ~/.assistant; org.db lives there and every
    // space database under orgs/{org}/spaces/{space}/.
    let base_path: PathBuf = dirs::home_dir()
        .map(|h| h.join(".assistant"))
        .context("Cannot determine the home directory")?;

    // -- Org-level storage (users, spaces, auth state) --------------------------
    let pool_factory = Arc::new(assistant_storage::OrgPoolFactory::new(base_path.clone()));

    let org_storage = Arc::new(
        pool_factory
            .org_storage()
            .await
            .context("Failed to open org.db")?,
    );
    let default_space = install::ensure_default_space(&org_storage)
        .await
        .context("Failed to seed the default org and space")?;
    install::ensure_initial_admin(&org_storage, &default_space, &base_path)
        .await
        .context("Failed to create the initial admin user")?;
    let space_resolver = space::SpaceResolver::new(org_storage.clone(), pool_factory.clone());

    // -- Settings --------------------------------------------------------------
    //
    // `server.toml` holds the process-wide settings. Each org's `org.toml` is
    // loaded the first time one of its spaces is used; the CLI flags override
    // the LLM settings of every org.
    let server_config = load_server_config(&base_path)?;
    let llm_provider_override = args
        .llm_provider
        .as_deref()
        .map(|provider| match provider.to_lowercase().as_str() {
            "ollama" => Ok(LlmProviderKind::Ollama),
            "anthropic" => Ok(LlmProviderKind::Anthropic),
            "openai" => Ok(LlmProviderKind::OpenAI),
            "moonshot" => Ok(LlmProviderKind::Moonshot),
            "openrouter" => Ok(LlmProviderKind::OpenRouter),
            other => anyhow::bail!(
                "Unsupported --llm-provider value: {other}. \
                 Expected one of: ollama, anthropic, openai, moonshot, openrouter."
            ),
        })
        .transpose()?;
    let (llm_model_override, llm_base_url_override) =
        (args.llm_model.clone(), args.llm_base_url.clone());
    let cli_agent_override = args.agent.clone();
    let agent_install_root = base_path.clone();
    let orgs = Arc::new(OrgContexts::from_files(&base_path).with_adjust(Arc::new(
        move |org_config| {
            if let Some(provider) = &llm_provider_override {
                org_config.llm.provider = provider.clone();
            }
            if let Some(model) = &llm_model_override {
                org_config.llm.model = model.clone();
            }
            if let Some(base_url) = &llm_base_url_override {
                org_config.llm.base_url = base_url.clone();
            }
            let agent = cli_agent_override
                .clone()
                .unwrap_or_else(|| org_config.agent.id.clone());
            if !validate_agent_id(&agent) {
                anyhow::bail!(
                    "Invalid agent ID '{agent}'. Use only letters, numbers, '-' and '_'."
                );
            }
            apply_agent_context(org_config, &agent_install_root, &agent);
            Ok(())
        },
    )));

    let _otel_guard = init_tracing()?;

    // -- VAPID key provisioning (once, at startup) ---------------------------
    //
    // If `[notifications]` is absent or incomplete, generate a new P-256 key
    // pair and write it back to `server.toml`.  This is done before anything
    // else so all subsequent handlers can read stable VAPID keys.
    let (vapid_private_key, vapid_public_key) = push::ensure_vapid_keys(
        &server_config_path(&base_path),
        server_config.notifications.vapid_private_key.as_deref(),
        server_config.notifications.vapid_public_key.as_deref(),
    )
    .await
    .unwrap_or_else(|e| {
        warn!("VAPID key provisioning failed: {e}; push notifications will be unavailable");
        (String::new(), String::new())
    });

    // The default org's settings decide the persona of the process-wide A2A
    // agent card and the voice providers.
    let default_org = orgs
        .get(assistant_storage::DEFAULT_ORG_SLUG)
        .context("Failed to load the default org's settings")?;
    let config = default_org.config.clone();
    let selected_agent = config.agent.id.clone();
    set_runtime_install_root(base_path.clone());
    set_runtime_agent_root(agent_base_dir(&base_path, &selected_agent));
    let workspace_dir = default_workspace_dir(&base_path, &selected_agent);
    set_runtime_workspace_dir(workspace_dir.clone());
    tokio::fs::create_dir_all(&workspace_dir)
        .await
        .with_context(|| format!("Failed to create workspace at {}", workspace_dir.display()))?;

    // -- Per-space runtimes --------------------------------------------------
    //
    // The web UI MUST route chat messages through the Orchestrator so the
    // assistant gets the same system prompt, tools, skills, memory, and ReAct
    // loop as every other interface (CLI, Slack, etc.). Each space has its own
    // orchestrator over its own database; `SpaceRuntimes` builds them on first
    // use.
    //
    // See skills/interface-implementation/SKILL.md for the canonical checklist.
    let llm = default_org.llm.clone();
    info!(
        "Chat LLM: provider={}, model={}",
        llm.provider_name(),
        llm.model_name()
    );

    // Create the audio store early so every orchestrator can reference it.
    let audio_store = Arc::new(audio_store::AudioStore::new());

    // -- Build TTS provider (for voice message playback) ---------------------
    let tts_provider = match config.tts.as_ref() {
        Some(tc) => {
            let provider = build_tts_provider(tc, &config.llm).await?;
            info!(provider = provider.name(), "TTS enabled (web UI)");
            Some(provider)
        }
        None => None,
    };

    // -- Integration tools: assembled per turn, never registered ambiently ----
    let secrets = Arc::new(
        assistant_core::integrations::SecretBox::load_from_env(&base_path)
            .context("Failed to load the integration secrets key")?,
    );
    let integration_tools = Arc::new(assistant_mcp_client::IntegrationToolSource::from_stores(
        assistant_mcp_client::IntegrationStores {
            integrations: Arc::new(org_storage.integration_store()),
            credentials: Arc::new(org_storage.integration_credential_store()),
            health: Arc::new(org_storage.integration_health_store()),
            orgs: Arc::new(org_storage.org_store()),
            spaces: Arc::new(org_storage.space_store()),
        },
        secrets.clone(),
        Arc::new(assistant_core::clock::SystemClock),
    ));

    let mut mcp_app_servers: Vec<Arc<dyn assistant_core::mcp_app::McpAppServers>> =
        vec![integration_tools.clone()];
    let mut runtimes = web_runtimes(pool_factory.clone(), orgs.clone(), audio_store.clone())
        .with_turn_tool_source(integration_tools);
    if let Some(bus) = shared_nats_bus(&server_config).await? {
        runtimes = runtimes.with_shared_bus(bus);
    }
    if let Some(ref tts) = tts_provider {
        runtimes = runtimes.with_voice_response(tts.clone());
    }
    let runtimes = Arc::new(runtimes);

    // Connect to configured external MCP servers and register their tools in
    // every space.
    if !server_config.mcp.servers.is_empty() {
        let mcp_manager = Arc::new(
            assistant_mcp_client::McpClientManager::start(&server_config.mcp.servers).await?,
        );
        mcp_app_servers.insert(0, mcp_manager.clone());
        let mcp_tools = mcp_manager.tool_handlers().await;
        for handler in &mcp_tools {
            runtimes.register_ambient_tool(handler.clone());
        }
        tracing::info!(
            servers = mcp_manager.server_count().await,
            tools = mcp_tools.len(),
            "registered MCP client tools"
        );

        // Spawn background health-check loop for reconnection.
        let register = runtimes.clone();
        let unregister = runtimes.clone();
        mcp_manager.spawn_health_loop(
            Arc::new(move |h| register.register_ambient_tool(h)),
            Arc::new(move |prefix| unregister.unregister_tools_by_prefix(prefix)),
        );

        // Offer the skills servers publish, in every space.
        let skills = runtimes.clone();
        mcp_manager.spawn_skill_sync(
            assistant_mcp_client::McpSkillSource::new(&base_path),
            Arc::new(move |server, published| {
                let skills = skills.clone();
                Box::pin(async move { skills.set_ambient_skills(&server, published).await })
            }),
        );
    }

    // -- Integrations catalog: disk cache now, refreshed in the background ------
    let catalog_sync = Arc::new(
        catalog_sync::CatalogSync::new(server_config.integrations.catalog.clone(), &base_path)
            .context("Failed to set up the integrations catalog sync")?,
    );
    catalog_sync.start().await;

    // -- Push dispatcher (built here so AppState can hold a reference) -------
    let push_dispatcher_for_state = if !vapid_private_key.is_empty() {
        Some(Arc::new(PushDispatcher::new(vapid_private_key.clone())))
    } else {
        None
    };

    let state = AppState {
        pool: org_storage.pool().clone(),
        agent_id: Arc::new(RwLock::new(selected_agent.clone())),
        bus_kind: server_config.bus.kind.clone(),
        nats_url: server_config
            .bus
            .nats_url
            .clone()
            .or_else(|| std::env::var("NATS_URL").ok()),
        nats_token: server_config
            .bus
            .token
            .as_ref()
            .map(|token| token.expose().to_owned())
            .or_else(|| std::env::var("NATS_TOKEN").ok()),
        push_dispatcher: push_dispatcher_for_state,
    };

    // The web-ui binary intentionally does NOT spawn its own turn-processing
    // worker pool or title-generator. Both are owned by the orchestrator
    // service (`assistant orchestrator run --no-repl`), which has a
    // `Web`-filtered worker that consumes the turns this process publishes.
    // Running them in both processes would cause two consumers to race for
    // every claim — one wins, the other floods the journal with
    // `claim failed` warnings every second.
    //
    // Operators MUST run an orchestrator service alongside the web-ui.
    // See docs/web-ui.md.
    //
    // The plan is computed (and asserted empty) so the binary is wired to
    // the same source of truth as the orchestrator's spawn logic. If a
    // future change adds a worker to `BinaryRole::WebUi` in
    // `assistant_runtime::worker_plan`, the assertion fires loudly here
    // instead of silently re-introducing the duplicate-consumer race.
    let webui_worker_plan =
        assistant_runtime::core_worker_plan(assistant_runtime::BinaryRole::WebUi);
    assert!(
        webui_worker_plan.is_empty(),
        "BinaryRole::WebUi must not request any infrastructure workers; got {:?}",
        webui_worker_plan
    );
    let _ = webui_worker_plan;

    // Workflow runner and trigger adapters, one set per space.
    let supervisor = Arc::new(SpaceSupervisor::new(
        org_storage.clone(),
        runtimes.clone(),
        space_tasks::workflow_spawner(Duration::from_secs(2)),
    ));
    tokio::spawn(supervisor.run(SPACE_RECONCILE_INTERVAL));

    // -- Persona-scoped A2A profile store (filesystem-backed) --
    let agent_store = AgentStore::for_persona(&base_path, &selected_agent)?;
    // Clone for the agents REST API — AgentStore is Clone (wraps a PathBuf).
    let agents_api_store = agent_store.clone();

    // -- A2A protocol state --
    //
    // `base_url` is the public-facing origin used in OAuth metadata
    // (`/.well-known/oauth-authorization-server`) and in the A2A agent
    // card. When deployed behind a reverse proxy (Pangolin etc.), the
    // bind address (`args.listen`) is not reachable from clients —
    // `--public-url` / `ASSISTANT_PUBLIC_URL` overrides it.
    let base_url = resolve_base_url(args.public_url.as_deref(), &args.listen);

    // Resolve the agent card from the store, falling back to a built-in default.
    let mut agent_card = match agent_store.get_default().await {
        Some(agent) => agent.card,
        None => build_default_agent_card(&base_url),
    };

    // Auto-harden: inject Bearer auth into the agent card so A2A callers
    // know they need to present a token.
    harden_agent_card(&mut agent_card);

    let a2a_state = A2AState {
        spaces: space_resolver.clone(),
        backend: Arc::new(a2a::backend::RuntimeA2aBackend::new(
            runtimes.clone(),
            selected_agent.clone(),
        )),
        agent_card,
        agent_id: selected_agent.clone(),
    };

    let workflows_api_state = api::workflows::WorkflowsApiState {
        spaces: space_resolver.clone(),
        agent_id: state.agent_id.clone(),
    };

    // -- Build transcription provider (for voice message STT) ----------------
    let transcription_provider = match config.transcription.as_ref() {
        Some(tc) => {
            let provider = build_transcription_provider(tc, &config.llm).await?;
            info!(
                provider = provider.name(),
                "Audio transcription enabled (web UI)"
            );
            Some(provider)
        }
        None => None,
    };

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

    let api_state = {
        let mut s = api::ApiState::new(
            space_resolver.clone(),
            runtimes.clone(),
            state.agent_id.clone(),
        )
        .with_mcp_apps(Arc::new(assistant_core::mcp_app::McpAppServerChain::new(
            mcp_app_servers,
        )));
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
        spaces: space_resolver.clone(),
        agent_id: state.agent_id.clone(),
        agents_dir: base_path.join("agents"),
    };

    let skills_api_state = api::skills::SkillsApiState {
        spaces: space_resolver.clone(),
        runtimes: runtimes.clone(),
    };

    let webhooks_api_state = api::webhooks::WebhooksApiState {
        spaces: space_resolver.clone(),
        agent_id: state.agent_id.clone(),
    };

    let agents_api_state = api::agents::AgentsApiState {
        agent_store: agents_api_store,
    };

    // Only mount the push API when a real VAPID keypair was provisioned.
    // An empty key means key generation failed; exposing the endpoints in that
    // state would let clients subscribe but never receive any delivery.
    let push_api_state = (!vapid_public_key.is_empty()).then(|| PushApiState {
        spaces: space_resolver.clone(),
        vapid_public_key: Arc::new(vapid_public_key),
    });

    // -- JWT + OAuth2 state ---------------------------------------------------
    let jwt_manager = {
        use assistant_auth::jwt::{JwtKeyPair, JwtManager};
        let jwt_secret_path = base_path.join("jwt_secret");
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

        oauth::OAuthState {
            oauth2_server,
            jwt_manager: jwt_manager.clone(),
            client_registrar,
            device_manager,
            org_storage: org_storage.clone(),
            issuer: base_url.clone(),
            secure_cookie,
            oidc: Arc::new(oauth::oidc_registry::OidcProviderRegistry::new(
                org_storage.clone(),
                secrets.clone(),
            )),
            oidc_sessions: Arc::new(oauth::oidc_sessions::PendingOidcSessionStore::new(
                Duration::from_secs(300),
            )),
            rate_limits: rate_limits.clone(),
        }
    };

    // -- Auth config (JWT + API key + legacy token) --------------------------
    let legacy_context = legacy_token.as_ref().map(|_| {
        use assistant_core::auth::AuthContext;
        use assistant_core::identity::{Action, ResourceKind, Role, Scope, UserId};
        let mut space_roles = std::collections::HashMap::new();
        space_roles.insert(default_space.space_id.clone(), Role::OrgAdmin);
        AuthContext {
            user_id: UserId::from("admin"),
            org_id: default_space.org_id.clone(),
            email: String::new(),
            space_roles,
            scopes: vec![Scope::new(ResourceKind::Org, Action::Manage)],
            client_id: "legacy".into(),
        }
    });
    let api_key_store: Arc<dyn assistant_auth::api_keys::ApiKeyStore> =
        Arc::new(org_storage.api_key_store());
    let auth_config = WebAuthConfig {
        auth_state: assistant_auth::middleware::AuthState {
            jwt_manager,
            api_key_store: api_key_store.clone(),
            users: Arc::new(org_storage.user_store()),
            memberships: Arc::new(org_storage.membership_store()),
            refresh_tokens: Arc::new(org_storage.refresh_token_store()),
            legacy_token,
            legacy_context,
        },
        secure_cookie,
    };

    let mcp_state = mcp::McpState {
        auth: auth_config.auth_state.clone(),
        spaces: space_resolver.clone(),
        runtimes: runtimes.clone(),
        issuer: base_url.clone(),
        listeners: ListChangeListeners::default(),
        tasks: TaskStore::default(),
    };

    // -- Management API states (orgs, users, spaces, members, API keys) ------
    let orgs_api_state = api::orgs::OrgsApiState {
        org_storage: org_storage.clone(),
        secrets: secrets.clone(),
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
    let account_api_state = api::account::AccountApiState {
        org_storage: org_storage.clone(),
        refresh_token_store: Arc::new(org_storage.refresh_token_store()),
    };
    let catalog_api_state = api::catalog::CatalogApiState {
        org_storage: org_storage.clone(),
    };
    let integrations_api_state =
        api::integrations::IntegrationsApiState::new(catalog_sync, &org_storage, secrets);
    let interfaces_api_state = api::interfaces::InterfacesApiState {
        org_storage: org_storage.clone(),
    };
    let bindings_api_state = api::bindings::BindingsApiState {
        org_storage: org_storage.clone(),
    };
    let templates_api_state = api::templates::TemplatesApiState {
        org_storage: org_storage.clone(),
        spaces: space_resolver.clone(),
    };

    // -- Router: public routes (no auth required) --------------------------
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .merge(auth::session_router().with_state(auth_config.clone()))
        // OpenAPI spec + Swagger UI (public — clients need the spec to discover auth).
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", openapi::ApiDoc::openapi()))
        // Public workflow webhook trigger ingress.
        .merge(api::workflows::workflow_public_router().with_state(workflows_api_state.clone()))
        // A2A agent card is public per spec — callers need it to discover auth.
        .merge(a2a::public_router().with_state(a2a_state.clone()))
        // OAuth2 endpoints (public — clients need these for auth flows).
        .merge(oauth::oauth_router().with_state(oauth_state))
        // MCP over Streamable HTTP; authenticates on its own (resource server).
        .merge(mcp::mcp_router(mcp_state));

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
                .merge(api::skills::skills_router().with_state(skills_api_state))
                .merge(api::webhooks::webhooks_api_router().with_state(webhooks_api_state))
                .merge(api::agents::agents_api_router().with_state(agents_api_state))
                .merge(api::workflows::workflows_api_router().with_state(workflows_api_state))
                .merge(api::orgs::orgs_api_router().with_state(orgs_api_state.clone()))
                .merge(api::org_oidc::org_oidc_api_router().with_state(orgs_api_state))
                .merge(api::users::users_api_router().with_state(users_api_state))
                .merge(api::spaces::spaces_api_router().with_state(spaces_api_state))
                .merge(api::members::members_api_router().with_state(members_api_state))
                .merge(api::api_keys::api_keys_router().with_state(api_keys_api_state))
                .merge(api::account::account_router().with_state(account_api_state))
                .merge(api::catalog::catalog_api_router().with_state(catalog_api_state))
                .merge(
                    api::integrations::integrations_api_router().with_state(integrations_api_state),
                )
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

    // -- CORS: same-origin unless origins are configured explicitly ----------
    let cors = cors::build_layer(&args.cors_origin)?;

    let router = public_routes
        .merge(protected_routes)
        // Flutter SPA fallback: serves embedded web assets for all unmatched
        // paths (no auth required — FR-013).
        .fallback(flutter_assets::flutter_handler)
        .layer(tower::util::option_layer(cors))
        .layer(axum::middleware::from_fn_with_state(
            rate_limits,
            rate_limit::enforce_ip_limits,
        ))
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
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
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

/// Resolve the public-facing base URL used in OAuth metadata + the A2A
/// agent card. Returns `--public-url` (with trailing slash stripped) when
/// set, else falls back to `http://{listen}`.
///
/// Behind a reverse proxy (Pangolin, Cloudflare, nginx), the bind address
/// is not reachable from clients — operators MUST set `--public-url` /
/// `ASSISTANT_PUBLIC_URL` so OAuth metadata advertises endpoints clients
/// can actually reach.
fn resolve_base_url(public_url: Option<&str>, listen: &str) -> String {
    public_url
        .map(|s| s.trim_end_matches('/').to_owned())
        .unwrap_or_else(|| format!("http://{}", listen))
}

#[cfg(test)]
mod base_url_tests {
    use super::resolve_base_url;

    #[test]
    fn falls_back_to_listen_when_public_url_unset() {
        assert_eq!(
            resolve_base_url(None, "127.0.0.1:8080"),
            "http://127.0.0.1:8080"
        );
        assert_eq!(
            resolve_base_url(None, "0.0.0.0:8080"),
            "http://0.0.0.0:8080"
        );
    }

    #[test]
    fn uses_public_url_when_set() {
        assert_eq!(
            resolve_base_url(Some("https://assistant.58lab.org"), "0.0.0.0:8080"),
            "https://assistant.58lab.org"
        );
    }

    #[test]
    fn strips_trailing_slash_from_public_url() {
        assert_eq!(
            resolve_base_url(Some("https://example.com/"), "0.0.0.0:8080"),
            "https://example.com"
        );
        // Multiple trailing slashes also stripped.
        assert_eq!(
            resolve_base_url(Some("https://example.com///"), "0.0.0.0:8080"),
            "https://example.com"
        );
    }

    #[test]
    fn preserves_subpath_in_public_url() {
        // Trailing slash on a path segment is stripped, but the path itself stays.
        assert_eq!(
            resolve_base_url(Some("https://example.com/api"), "0.0.0.0:8080"),
            "https://example.com/api"
        );
    }
}

#[cfg(test)]
mod harden_agent_card_tests {
    use super::harden_agent_card;
    use assistant_a2a_json_schema::agent_card::AgentCard;

    fn empty_card() -> AgentCard {
        // Construct via Default since the schema is generated; fall back to
        // serde if not Default.
        serde_json::from_value::<AgentCard>(serde_json::json!({
            "name": "test",
            "description": "test card",
            "url": "https://example.com",
            "version": "0.1.0",
            "protocolVersion": "0.2.0",
            "skills": [],
            "capabilities": {},
            "defaultInputModes": ["text"],
            "defaultOutputModes": ["text"],
            "supportedInterfaces": []
        }))
        .expect("valid empty card")
    }

    #[test]
    fn injects_bearer_scheme_when_missing() {
        let mut card = empty_card();
        assert!(card.security_schemes.is_empty());
        harden_agent_card(&mut card);
        assert!(card.security_schemes.contains_key("bearer_token"));
    }

    #[test]
    fn appends_security_requirement_for_bearer() {
        let mut card = empty_card();
        assert!(card.security_requirements.is_empty());
        harden_agent_card(&mut card);
        assert!(
            card.security_requirements
                .iter()
                .any(|r| r.schemes.contains_key("bearer_token")),
            "expected a SecurityRequirement referencing bearer_token"
        );
    }

    #[test]
    fn is_idempotent() {
        let mut card = empty_card();
        harden_agent_card(&mut card);
        let scheme_count = card.security_schemes.len();
        let req_count = card.security_requirements.len();
        harden_agent_card(&mut card);
        // A second pass must not duplicate the scheme or requirement.
        assert_eq!(card.security_schemes.len(), scheme_count);
        assert_eq!(card.security_requirements.len(), req_count);
    }
}

#[cfg(test)]
mod nats_reachable_tests {
    use super::nats_reachable;

    /// A URL pointing at a closed port should return false quickly (within
    /// the 1-second connect timeout). This exercises the connection-failure
    /// branch without requiring a real NATS server.
    #[tokio::test]
    async fn returns_false_for_unreachable_endpoint() {
        // Port 1 is privileged and almost always unbindable / connection-refused.
        let result = nats_reachable("nats://127.0.0.1:1", None).await;
        assert!(!result, "expected false for unreachable endpoint");
    }

    #[tokio::test]
    async fn returns_false_for_unreachable_with_token() {
        let result = nats_reachable("nats://127.0.0.1:1", Some("secret")).await;
        assert!(
            !result,
            "expected false for unreachable endpoint with token"
        );
    }

    #[tokio::test]
    async fn handles_url_without_nats_prefix() {
        // The function should also accept bare host:port.
        let result = nats_reachable("127.0.0.1:1", None).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn handles_url_with_user_password_auth() {
        // user:pass@host should be parsed without panicking.
        let result = nats_reachable("nats://user:pass@127.0.0.1:1", None).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn handles_url_with_token_only_auth() {
        let result = nats_reachable("nats://token@127.0.0.1:1", None).await;
        assert!(!result);
    }
}

#[cfg(test)]
mod args_tests {
    use clap::Parser as _;

    use super::Args;

    #[test]
    fn db_path_flag_is_rejected() {
        let err = Args::try_parse_from(["webui", "--db-path", "/tmp/x.db"])
            .expect_err("--db-path was removed");
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }
}

#[cfg(test)]
mod runtime_tests {
    use std::sync::Arc;

    use assistant_core::types::agent::OrgConfig;
    use assistant_llm_provider::scripted::ScriptedLlmProvider;
    use assistant_runtime::OrgContexts;
    use assistant_storage::{DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG, OrgPoolFactory};

    use super::*;

    #[tokio::test]
    async fn the_webui_process_builds_an_interaction_broker_in_every_space() {
        let root = tempfile::tempdir().unwrap();
        let mut config = OrgConfig::default();
        config.memory.enabled = false;
        let runtimes = web_runtimes(
            Arc::new(OrgPoolFactory::new(root.path())),
            Arc::new(OrgContexts::fixed(
                config,
                Arc::new(ScriptedLlmProvider::new()),
            )),
            Arc::new(audio_store::AudioStore::new()),
        );

        let runtime = runtimes
            .get(DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG)
            .await
            .unwrap();
        assert!(
            runtime.interactions.is_some(),
            "the respond route needs a broker over the space's own store"
        );
    }
}
