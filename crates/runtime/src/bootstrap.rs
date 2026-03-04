//! Shared bootstrap helpers for interface binaries.
//!
//! These utilities are used by the Slack and Mattermost interface binaries to
//! reduce code duplication for common startup tasks: loading config, finding
//! skill directories, and providing an auto-deny confirmation callback.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::{expand_tilde, AssistantConfig, MemoryConfig};
use assistant_llm::LlmProvider;
use assistant_skills::SkillSource;
use assistant_storage::StorageLayer;
use tracing::info;

use crate::memory_indexer::MemoryIndexer;
use crate::orchestrator::ConfirmationCallback;

// ── Auto-deny confirmation callback ───────────────────────────────────────────

/// A [`ConfirmationCallback`] that always denies, suitable for automated
/// interfaces where interactive prompts are not possible.
///
/// This callback handles tools marked `requires_confirmation` in automated
/// interfaces where the user cannot be interactively prompted.
pub struct AutoDenyConfirmation {
    /// Human-readable interface name used in log messages (e.g. `"Slack"`).
    pub interface_name: &'static str,
}

impl ConfirmationCallback for AutoDenyConfirmation {
    fn confirm(&self, tool_name: &str, _params: &serde_json::Value) -> bool {
        tracing::warn!(
            tool = tool_name,
            "{} interface: auto-denying confirmation-required tool",
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

/// Return the runtime skill search directories.
///
/// Builtin skills are embedded into the binary via [`embedded_builtin_skills`]
/// and do not require a filesystem path.  This function only returns directories
/// for runtime-discovered skills:
///
/// * Entries from `skills.extra_dirs` (home-relative, absolute, or project-relative)
/// * `~/.assistant/skills/` — user-installed skills
/// * `<project>/.assistant/skills/` — project-scoped skills (if available)
/// * `<exe_dir>/skills/` — optional sidecar skills shipped alongside the binary
pub fn skill_dirs(
    config: &AssistantConfig,
    project_root: Option<&Path>,
) -> Vec<(PathBuf, SkillSource)> {
    let mut dirs: Vec<(PathBuf, SkillSource)> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    let resolved_project_root = project_root
        .map(|p| p.to_path_buf())
        .or_else(|| std::env::current_dir().ok());

    let mut push_dir = |path: PathBuf, source: SkillSource| {
        if path.is_dir() && seen.insert(path.clone()) {
            dirs.push((path, source));
        }
    };

    for raw in &config.skills.extra_dirs {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let resolved = resolve_extra_dir(trimmed, resolved_project_root.as_deref());
        push_dir(resolved, SkillSource::Installed);
    }

    if let Some(home) = dirs::home_dir() {
        push_dir(home.join(".assistant").join("skills"), SkillSource::User);
    }

    if let Some(project) = resolved_project_root.as_ref() {
        push_dir(
            project.join(".assistant").join("skills"),
            SkillSource::Project,
        );
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            push_dir(exe_dir.join("skills"), SkillSource::Builtin);
        }
    }

    dirs
}

/// Spawn the memory indexer as a background task.
///
/// The indexer periodically scans memory files, computes content hashes,
/// and generates embeddings for changed content.
pub fn spawn_memory_indexer(
    config: &MemoryConfig,
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
) -> tokio::task::JoinHandle<()> {
    let interval = Duration::from_secs(config.indexing_interval_seconds.unwrap_or(300));
    let enabled = config.enabled;

    // Create a minimal AssistantConfig with just the memory section
    // for the MemoryIndexer
    let assistant_config = Arc::new(AssistantConfig {
        memory: config.clone(),
        ..AssistantConfig::default()
    });

    let indexer = Arc::new(MemoryIndexer::new(assistant_config, storage, llm));

    tokio::spawn(async move {
        if !enabled {
            info!("Memory indexer disabled, not starting");
            return;
        }

        info!("Memory indexer started (interval: {:?})", interval);

        // Run initial indexing
        if let Err(e) = indexer.index_all().await {
            tracing::warn!("Initial memory indexing failed: {}", e);
        }

        loop {
            tokio::time::sleep(interval).await;

            if let Err(e) = indexer.index_all().await {
                tracing::warn!("Memory indexing failed: {}", e);
            }
        }
    })
}

fn resolve_extra_dir(raw: &str, project_root: Option<&Path>) -> PathBuf {
    if raw.starts_with("./") || raw.starts_with("../") {
        if let Some(root) = project_root {
            return root.join(raw);
        }
        return PathBuf::from(raw);
    }
    expand_tilde(raw)
}
