//! Shared bootstrap helpers for interface binaries.
//!
//! These utilities are used by the Slack and Mattermost interface binaries to
//! reduce code duplication for common startup tasks: loading config, finding
//! skill directories, and providing an auto-deny confirmation callback.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::{skill::SkillSource, AssistantConfig};
use assistant_llm::LlmProvider;
use assistant_skills_executor::{spawn_memory_indexer, MemoryIndexer};
use assistant_storage::StorageLayer;
use tracing::info;

use crate::orchestrator::ConfirmationCallback;

// ── Auto-deny confirmation callback ───────────────────────────────────────────

/// A [`ConfirmationCallback`] that always denies, suitable for automated
/// interfaces where interactive prompts are not possible.
///
/// This callback handles skills marked `confirmation_required` in automated
/// interfaces where the user cannot be interactively prompted.
pub struct AutoDenyConfirmation {
    /// Human-readable interface name used in log messages (e.g. `"Slack"`).
    pub interface_name: &'static str,
}

impl ConfirmationCallback for AutoDenyConfirmation {
    fn confirm(&self, skill_name: &str, _params: &serde_json::Value) -> bool {
        tracing::warn!(
            skill = skill_name,
            "{} interface: auto-denying confirmation-required skill",
            self.interface_name,
        );
        false
    }
}

// ── Config loading ─────────────────────────────────────────────────────────────

/// Load [`AssistantConfig`] from a TOML file.
///
/// Returns [`AssistantConfig::default()`] if the file does not exist.
pub async fn load_config(config_path: &Path) -> Result<AssistantConfig> {
    if !config_path.exists() {
        return Ok(AssistantConfig::default());
    }

    let raw = tokio::fs::read_to_string(config_path)
        .await
        .with_context(|| format!("Failed to read config at {}", config_path.display()))?;

    let cfg = toml::from_str::<AssistantConfig>(&raw)
        .with_context(|| format!("Failed to parse config at {}", config_path.display()))?;

    info!("Loaded config from {}", config_path.display());
    Ok(cfg)
}

// ── Skill directories ──────────────────────────────────────────────────────────

/// Construct and spawn a background memory indexer task.
///
/// The indexer scans all memory files every `interval` and updates the
/// `memory_chunks` SQLite table.  Embeddings are generated lazily via the
/// LLM provider's `embed()` endpoint.
///
/// Call this once after building the storage layer and LLM provider.
/// The default interval is 5 minutes.
pub fn start_memory_indexer(
    config: Arc<AssistantConfig>,
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
    interval: Option<Duration>,
) {
    let indexer = Arc::new(MemoryIndexer::new(config, storage, llm));
    let interval = interval.unwrap_or(Duration::from_secs(5 * 60));
    spawn_memory_indexer(indexer, interval);
}

/// Return the runtime skill search directories.
///
/// Builtin skills are embedded into the binary via [`embedded_builtin_skills`]
/// and do not require a filesystem path.  This function only returns directories
/// for runtime-discovered skills:
///
/// * `<exe_dir>/skills/` — optional sidecar skills shipped alongside the binary
/// * `~/.assistant/skills/` — user-installed skills
pub fn skill_dirs() -> Vec<(PathBuf, SkillSource)> {
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
