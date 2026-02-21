//! `assistant-mattermost` — Mattermost interface binary.
//!
//! # Subcommands
//!
//! - `run` — Start the Mattermost WebSocket listener loop.
//!
//! # Building
//!
//! Without the Mattermost feature (stub only):
//! ```sh
//! cargo build -p assistant-interface-mattermost
//! ```
//!
//! With the full Mattermost integration (mattermost_api):
//! ```sh
//! cargo build -p assistant-interface-mattermost --features mattermost
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
use assistant_core::{skill::SkillSource, AssistantConfig};
use assistant_llm::{LlmClient, LlmClientConfig};
use assistant_runtime::{orchestrator::ConfirmationCallback, Orchestrator};
use assistant_skills_executor::SkillExecutor;
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use clap::Parser;
use tracing::{info, warn};

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

// ── Confirmation callback ────────────────────────────────────────────────────

/// A confirmation callback that always denies, suitable for an automated
/// interface where interactive prompts are not possible.
///
/// `SafetyGate` already blocks `shell-exec` on Mattermost; this callback
/// provides a second layer for skills marked `confirmation_required`.
struct AutoDenyConfirmation;

impl ConfirmationCallback for AutoDenyConfirmation {
    fn confirm(&self, skill_name: &str, _params: &serde_json::Value) -> bool {
        warn!(
            skill = skill_name,
            "Mattermost interface: auto-denying confirmation-required skill"
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
/// Returns `(orchestrator, mattermost_config)`.
async fn bootstrap() -> Result<(Orchestrator, MattermostConfig)> {
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
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
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
