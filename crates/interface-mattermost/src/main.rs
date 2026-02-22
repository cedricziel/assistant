//! `assistant-mattermost` — Mattermost interface binary.
//!
//! # Subcommands
//!
//! - `run` — Start the Mattermost WebSocket listener loop.
//!
//! # Building
//!
//! ```sh
//! cargo build -p assistant-interface-mattermost
//! ```
//!
//! # Configuration
//!
//! Add a `[mattermost]` section to `~/.assistant/config.toml`:
//!
//! ```toml
//! [mattermost]
//! server_url = "https://mattermost.example.com"
//! token      = "your-personal-access-token"
//! allowed_channels = ["town-square"]   # optional allowlist
//! allowed_users    = ["alice"]         # optional allowlist
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_core::skill::SkillSource;
use assistant_provider_ollama::OllamaProvider;
use assistant_runtime::{
    bootstrap::{load_config, skill_dirs, AutoDenyConfirmation},
    orchestrator::ConfirmationCallback,
    Orchestrator,
};
use assistant_skills_executor::SkillExecutor;
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use clap::Parser;
use tracing::info;

use assistant_interface_mattermost::{config::MattermostConfig, MattermostInterface};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "assistant-mattermost",
    about = "Mattermost interface for the AI assistant",
    version
)]
enum Cmd {
    /// Start the Mattermost WebSocket listener loop.
    ///
    /// Requires a Mattermost personal access token and server URL configured
    /// in ~/.assistant/config.toml under [mattermost].
    Run,
}

// ── Stack bootstrap ───────────────────────────────────────────────────────────

/// Bootstrap the common stack shared by all subcommands.
///
/// Returns `(orchestrator, mattermost_config)`.
async fn bootstrap() -> Result<(Orchestrator, MattermostConfig)> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let assistant_dir = home.join(".assistant");
    let config_path = assistant_dir.join("config.toml");
    let config = load_config(&config_path).await?;

    // Resolve database path.
    let db_path: PathBuf = config
        .storage
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| assistant_dir.join("assistant.db"));

    // Open storage layer.
    let storage = Arc::new(
        StorageLayer::new(&db_path)
            .await
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?,
    );

    // Create skill registry.
    let mut registry = SkillRegistry::new(storage.pool.clone())
        .await
        .context("Failed to create skill registry")?;

    let dirs_to_scan = skill_dirs();
    let dirs_ref: Vec<(&Path, SkillSource)> = dirs_to_scan
        .iter()
        .map(|(p, s)| (p.as_path(), s.clone()))
        .collect();

    registry
        .load_embedded()
        .await
        .context("Failed to load embedded builtin skills")?;

    registry
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills from directories")?;

    let registry = Arc::new(registry);

    // Build LLM client.
    let llm = Arc::new(
        OllamaProvider::from_llm_config(&config.llm).context("Failed to create LLM client")?,
    );

    // Build skill executor.
    let executor = Arc::new(SkillExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    // Build orchestrator with auto-deny confirmation.
    let confirmation_cb: Arc<dyn ConfirmationCallback> = Arc::new(AutoDenyConfirmation {
        interface_name: "Mattermost",
    });
    let orchestrator = Orchestrator::new(llm, storage, registry, executor, &config)
        .with_confirmation_callback(confirmation_cb);

    // Extract the [mattermost] section from config (or use defaults).
    let mattermost_config: MattermostConfig = config.mattermost.clone().unwrap_or_default();

    Ok((orchestrator, mattermost_config))
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cmd = Cmd::parse();

    match cmd {
        Cmd::Run => {
            let (orchestrator, mattermost_config) = bootstrap().await?;
            let interface = MattermostInterface::new(mattermost_config, Arc::new(orchestrator));
            info!("Starting Mattermost interface");
            interface.run().await?;
        }
    }

    Ok(())
}
