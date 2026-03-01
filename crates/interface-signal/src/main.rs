//! `assistant-signal` — Signal messenger interface binary.
//!
//! # Subcommands
//!
//! - `link`  — Link this machine as a Signal secondary device (scan QR in app).
//! - `run`   — Start the Signal listener loop.
//!
//! # Building
//!
//! Without the Signal feature (stub only):
//! ```sh
//! cargo build -p assistant-interface-signal
//! ```
//!
//! With the full Signal integration (presage):
//! ```sh
//! cargo build -p assistant-interface-signal --features signal
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_core::{AssistantConfig, MessageBus};
use assistant_provider_ollama::OllamaProvider;
use assistant_runtime::{orchestrator::ConfirmationCallback, Orchestrator};
use assistant_skills::SkillSource;
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use assistant_tool_executor::ToolExecutor;
use clap::Parser;
use tracing::{info, warn};

use assistant_interface_signal::{
    config::{SignalConfig, SignalConfigExt},
    link_device, SignalInterface,
};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "assistant-signal",
    about = "Signal messenger interface for the AI assistant",
    version
)]
enum Cmd {
    /// Link this machine as a Signal secondary device.
    ///
    /// Prints a QR code — scan it in Signal → Settings → Linked Devices.
    Link {
        /// Name shown for this device in the Signal app.
        #[arg(long, default_value = "Assistant")]
        device_name: String,
    },

    /// Start the Signal listener loop.
    ///
    /// Requires the device to be linked first (`assistant-signal link`).
    Run,
}

// ── Confirmation callback ────────────────────────────────────────────────────

/// A confirmation callback that always denies, suitable for an automated
/// interface where interactive prompts are not possible.
///
/// Signal runs without interactive prompts, so this callback auto-denies
/// any tool that explicitly requires confirmation.
struct AutoDenyConfirmation;

impl ConfirmationCallback for AutoDenyConfirmation {
    fn confirm(&self, skill_name: &str, _params: &serde_json::Value) -> bool {
        warn!(
            skill = skill_name,
            "Signal interface: auto-denying confirmation-required skill"
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

// ── Stack bootstrap ───────────────────────────────────────────────────────────

/// Bootstrap the common stack shared by both subcommands.
///
/// Returns `(orchestrator, signal_config, store_path)`.
async fn bootstrap() -> Result<(Arc<Orchestrator>, SignalConfig, PathBuf)> {
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
    let registry = SkillRegistry::new(storage.pool.clone())
        .await
        .context("Failed to create skill registry")?;

    let project_root = std::env::current_dir().ok();
    let dirs_to_scan = assistant_runtime::bootstrap::skill_dirs(&config, project_root.as_deref());
    let dirs_ref: Vec<(&Path, SkillSource)> = dirs_to_scan
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

    // Build LLM client.
    let llm = Arc::new(
        OllamaProvider::from_llm_config(&config.llm).context("Failed to create LLM client")?,
    );

    // Build tool executor.
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    // Build message bus and orchestrator with auto-deny confirmation.
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let confirmation_cb: Arc<dyn ConfirmationCallback> = Arc::new(AutoDenyConfirmation);
    let orchestrator = Arc::new(
        Orchestrator::new(
            llm,
            storage,
            executor.clone(),
            registry.clone(),
            bus,
            &config,
        )
        .with_confirmation_callback(confirmation_cb),
    );

    // Wire up subagent support (breaks the init-time circular dep).
    executor.set_subagent_runner(orchestrator.clone());

    // Extract the [signal] section from config (or use defaults).
    let signal_config: SignalConfig = config.signal.clone().unwrap_or_default();
    let store_path = signal_config.resolved_store_path();

    Ok((orchestrator, signal_config, store_path))
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
        Cmd::Link { device_name } => {
            // Only the store path is needed for linking; bootstrap the full
            // stack anyway so the config is validated early.
            let (_orchestrator, _signal_config, store_path) = bootstrap().await?;
            link_device(&store_path, &device_name).await?;
        }

        Cmd::Run => {
            let (orchestrator, signal_config, _store_path) = bootstrap().await?;

            // Spawn the turn worker so bus-submitted turns get processed.
            let worker_orch = orchestrator.clone();
            let _worker = tokio::spawn(async move {
                worker_orch.run_worker("signal-worker").await;
            });

            let interface = SignalInterface::new(signal_config, orchestrator);
            info!("Starting Signal interface");
            interface.run().await?;
        }
    }

    Ok(())
}
