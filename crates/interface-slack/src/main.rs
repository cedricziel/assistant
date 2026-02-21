//! `assistant-slack` — Slack interface binary.
//!
//! # Subcommands
//!
//! - `run` — Start the Slack Socket Mode listener loop.
//!
//! # Building
//!
//! Without the Slack feature (stub only):
//! ```sh
//! cargo build -p assistant-interface-slack
//! ```
//!
//! With the full Slack integration (slack-morphism):
//! ```sh
//! cargo build -p assistant-interface-slack --features slack
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
use assistant_core::{skill::SkillSource, AssistantConfig};
use assistant_llm::{LlmClient, LlmClientConfig};
use assistant_runtime::{orchestrator::ConfirmationCallback, Orchestrator};
use assistant_skills_executor::SkillExecutor;
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use clap::Parser;
use tracing::{info, warn};

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

// ── Confirmation callback ────────────────────────────────────────────────────

/// A confirmation callback that always denies, suitable for an automated
/// interface where interactive prompts are not possible.
///
/// `SafetyGate` already blocks `shell-exec` on Slack; this callback provides
/// a second layer for skills marked `confirmation_required`.
struct AutoDenyConfirmation;

impl ConfirmationCallback for AutoDenyConfirmation {
    fn confirm(&self, skill_name: &str, _params: &serde_json::Value) -> bool {
        warn!(
            skill = skill_name,
            "Slack interface: auto-denying confirmation-required skill"
        );
        false
    }
}

// ── Config loading ────────────────────────────────────────────────────────────

fn load_config(config_path: &Path) -> Result<AssistantConfig> {
    if !config_path.exists() {
        return Ok(AssistantConfig::default());
    }

    let raw = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config at {}", config_path.display()))?;

    let cfg = toml::from_str::<AssistantConfig>(&raw)
        .with_context(|| format!("Failed to parse config at {}", config_path.display()))?;

    info!("Loaded config from {}", config_path.display());
    Ok(cfg)
}

// ── Skill directories ─────────────────────────────────────────────────────────

fn skill_dirs() -> Vec<(PathBuf, SkillSource)> {
    let mut dirs: Vec<(PathBuf, SkillSource)> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            dirs.push((exe_dir.join("skills"), SkillSource::Builtin));
        }
    }

    if let Some(home) = dirs::home_dir() {
        dirs.push((home.join(".assistant").join("skills"), SkillSource::User));
    }

    dirs
}

// ── Stack bootstrap ───────────────────────────────────────────────────────────

/// Bootstrap the common stack shared by all subcommands.
///
/// Returns `(orchestrator, slack_config)`.
async fn bootstrap() -> Result<(Orchestrator, SlackConfig)> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let assistant_dir = home.join(".assistant");
    let config_path = assistant_dir.join("config.toml");
    let config = load_config(&config_path)?;

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
    ));

    // Build orchestrator with auto-deny confirmation.
    let confirmation_cb: Arc<dyn ConfirmationCallback> = Arc::new(AutoDenyConfirmation);
    let orchestrator = Orchestrator::new(llm, storage, registry, executor, &config)
        .with_confirmation_callback(confirmation_cb);

    // Extract the [slack] section from config (or use defaults).
    let slack_config: SlackConfig = config.slack.clone().unwrap_or_default();

    Ok((orchestrator, slack_config))
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cmd = Cmd::parse();

    match cmd {
        Cmd::Run => {
            let (orchestrator, slack_config) = bootstrap().await?;
            let interface = SlackInterface::new(slack_config, Arc::new(orchestrator));
            info!("Starting Slack interface");
            interface.run().await?;
        }
    }

    Ok(())
}
