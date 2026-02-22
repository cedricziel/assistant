//! `assistant-slack` — Slack interface binary.
//!
//! # Subcommands
//!
//! - `run` — Start the Slack Socket Mode listener loop.
//!
//! # Building
//!
//! ```sh
//! cargo build -p assistant-interface-slack
//! ```
//!
//! # Configuration
//!
//! Add a `[slack]` section to `~/.assistant/config.toml`:
//!
//! ```toml
//! [slack]
//! bot_token = "xoxb-..."
//! app_token = "xapp-..."
//! allowed_channels = ["C0123456789"]   # optional allowlist
//! allowed_users    = ["U0123456789"]   # optional allowlist
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_core::skill::SkillSource;
use assistant_llm::{LlmClient, LlmClientConfig};
use assistant_runtime::{
    bootstrap::{load_config, skill_dirs, AutoDenyConfirmation},
    orchestrator::ConfirmationCallback,
    Orchestrator,
};
use assistant_skills_executor::SkillExecutor;
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use clap::Parser;
use tracing::info;

use assistant_interface_slack::{config::SlackConfig, SlackInterface};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "assistant-slack",
    about = "Slack interface for the AI assistant",
    version
)]
enum Cmd {
    /// Start the Slack Socket Mode listener loop.
    ///
    /// Requires a Slack app with Socket Mode enabled.  Configure bot_token and
    /// app_token in ~/.assistant/config.toml under [slack].
    Run,
}

// ── Stack bootstrap ───────────────────────────────────────────────────────────

/// Bootstrap the common stack shared by all subcommands.
///
/// Returns `(orchestrator, storage, slack_config)`.
async fn bootstrap() -> Result<(Orchestrator, Arc<StorageLayer>, SlackConfig)> {
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
    // Keep a reference to pass to SlackInterface for thread history seeding.
    let storage_ref = storage.clone();

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
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills from directories")?;

    let registry = Arc::new(registry);

    // Build LLM client.
    let llm_config = LlmClientConfig::from(&config.llm);
    let llm = Arc::new(LlmClient::new(llm_config).context("Failed to create LLM client")?);

    // Build skill executor.
    let executor = Arc::new(SkillExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    // Build orchestrator with auto-deny confirmation.
    let confirmation_cb: Arc<dyn ConfirmationCallback> = Arc::new(AutoDenyConfirmation {
        interface_name: "Slack",
    });
    let orchestrator = Orchestrator::new(llm, storage, registry, executor, &config)
        .with_confirmation_callback(confirmation_cb);

    // Extract the [slack] section from config (or use defaults).
    let slack_config: SlackConfig = config.slack.clone().unwrap_or_default();

    Ok((orchestrator, storage_ref, slack_config))
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
            let (orchestrator, storage, slack_config) = bootstrap().await?;
            let interface = SlackInterface::new(slack_config, Arc::new(orchestrator), storage);
            info!("Starting Slack interface");
            interface.run().await?;
        }
    }

    Ok(())
}
