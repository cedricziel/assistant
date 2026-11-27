//! Common bootstrap — build the shared `StorageLayer` / `SkillRegistry` /
//! `ToolExecutor` / `Orchestrator` tuple used by every CLI subcommand.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;

use assistant_core::MessageBus;
use assistant_core::types::agent::{OrgConfig, ServerConfig};
use assistant_core::types::storage::BusKind;
use assistant_runtime::Orchestrator;
use assistant_runtime::{OrgContexts, SpaceRuntimes, orchestrator::ConfirmationCallback};
use assistant_storage::{DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG};
use assistant_storage::{OrgPoolFactory, StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;

/// This process's runtimes: one per space, built on demand, plus the default
/// space's stack for the REPL, one-shot turns and the MCP server.
pub struct Bootstrap {
    /// The settings of the default org, which the REPL, one-shot turns and the
    /// messengers run under.
    pub config: Arc<OrgConfig>,
    pub runtimes: Arc<SpaceRuntimes>,
    pub storage: Arc<StorageLayer>,
    pub registry: Arc<SkillRegistry>,
    pub executor: Arc<ToolExecutor>,
    pub orchestrator: Arc<Orchestrator>,
    pub user_skills_dir: PathBuf,
}

/// The broker shared by every space, when `[bus] kind = "nats"`; otherwise each
/// space uses the SQLite bus in its own database.
async fn shared_nats_bus(config: &ServerConfig) -> Result<Option<Arc<dyn MessageBus>>> {
    if config.bus.kind != BusKind::Nats {
        return Ok(None);
    }
    #[cfg(feature = "nats")]
    {
        info!("Using NATS message bus");
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

pub async fn bootstrap(
    install_root: &Path,
    confirmation_cb: Option<Arc<dyn ConfirmationCallback>>,
    pools: Arc<OrgPoolFactory>,
    server: ServerConfig,
    orgs: Arc<OrgContexts>,
) -> Result<Bootstrap> {
    let user_skills_dir = install_root.join("skills");

    // With the `nats` feature and `[bus] kind = "nats"`, turns travel over NATS
    // JetStream (namespaced per space) to avoid SQLite write-lock contention
    // between the bus and the rest of the storage layer.
    // Integration tools are assembled per turn from org.db, through one
    // connection pool for the whole process; they are never registered as
    // ambient tools.
    let org_storage = pools
        .org_storage()
        .await
        .context("Failed to open org.db for integrations")?;
    let secrets = Arc::new(
        assistant_core::integrations::SecretBox::load_from_env(pools.base_dir())
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
        secrets,
        Arc::new(assistant_core::clock::SystemClock),
    ));

    // Interactions are on in the process that runs turns (ADR-0017, D9): an
    // undecided call on an interactive turn asks the turn's owner. A REPL
    // keeps answering its own `Interface::Cli` turns through `confirmation_cb`.
    let mut runtimes = SpaceRuntimes::new(pools.clone(), orgs.clone(), confirmation_cb)
        .with_interactions()
        .with_audio_store(Arc::new(assistant_transcription::AudioStore::new()))
        .with_turn_tool_source(integration_tools);
    if let Some(bus) = shared_nats_bus(&server).await? {
        runtimes = runtimes.with_shared_bus(bus);
    }
    let runtimes = Arc::new(runtimes);

    // Connect to configured external MCP servers and register their tools in
    // every space.
    if !server.mcp.servers.is_empty() {
        let mcp_manager =
            Arc::new(assistant_mcp_client::McpClientManager::start(&server.mcp.servers).await?);
        let mcp_tools = mcp_manager.tool_handlers().await;
        for handler in &mcp_tools {
            runtimes.register_ambient_tool(handler.clone());
        }
        info!(
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
            assistant_mcp_client::McpSkillSource::new(pools.base_dir()),
            Arc::new(move |server, published| {
                let skills = skills.clone();
                Box::pin(async move { skills.set_ambient_skills(&server, published).await })
            }),
        );
    }

    let default_space = runtimes
        .get(DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG)
        .await
        .context("Failed to open the default space runtime")?;

    Ok(Bootstrap {
        config: default_space.config.clone(),
        runtimes,
        storage: default_space.storage.clone(),
        registry: default_space.registry.clone(),
        executor: default_space.executor.clone(),
        orchestrator: default_space.orchestrator.clone(),
        user_skills_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_llm_provider::scripted::ScriptedLlmProvider;
    use assistant_runtime::OrgContexts;

    #[tokio::test]
    async fn the_process_that_runs_turns_can_ask_the_owner() {
        let root = tempfile::tempdir().unwrap();
        let mut config = OrgConfig::default();
        config.memory.enabled = false;
        let orgs = Arc::new(OrgContexts::fixed(
            config,
            Arc::new(ScriptedLlmProvider::new()),
        ));
        let bs = bootstrap(
            root.path(),
            None,
            Arc::new(OrgPoolFactory::new(root.path())),
            ServerConfig::default(),
            orgs,
        )
        .await
        .unwrap();

        let runtime = bs
            .runtimes
            .get(DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG)
            .await
            .unwrap();
        assert!(
            runtime.interactions.is_some(),
            "an undecided call goes to the turn's owner"
        );
    }
}
