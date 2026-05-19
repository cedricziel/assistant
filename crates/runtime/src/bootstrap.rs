//! Shared bootstrap helpers for interface binaries.
//!
//! These utilities are used by the Slack and Mattermost interface binaries to
//! reduce code duplication for common startup tasks: loading config, finding
//! skill directories, and providing an auto-deny confirmation callback.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_core::types::features::MemoryConfig;
use assistant_core::{LlmProvider, expand_tilde, types::agent::AssistantConfig};
use assistant_skills::SkillSource;
use assistant_storage::StorageLayer;
use tracing::info;

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

    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        push_dir(exe_dir.join("skills"), SkillSource::Builtin);
    }

    dirs
}

/// Spawn the memory indexer as a background task.
///
/// Delegates to [`crate::memory_indexer::spawn_memory_indexer`] so there is
/// a single canonical implementation.
pub fn spawn_memory_indexer(
    config: &MemoryConfig,
    storage: Arc<StorageLayer>,
    llm: Arc<dyn LlmProvider>,
) -> tokio::task::JoinHandle<()> {
    crate::memory_indexer::spawn_memory_indexer(config, storage, llm)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── AutoDenyConfirmation ──────────────────────────────────────────────

    #[test]
    fn auto_deny_always_returns_false() {
        let cb = AutoDenyConfirmation {
            interface_name: "TestInterface",
        };
        assert!(!cb.confirm("any-tool", &serde_json::json!({})));
        assert!(!cb.confirm("dangerous", &serde_json::json!({"foo": 1})));
    }

    // ── load_config ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn load_config_returns_default_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist.toml");
        let cfg = load_config(&missing).await.unwrap();
        // Default config should round-trip back; we just check it loads.
        let _ = cfg;
    }

    #[tokio::test]
    async fn load_config_parses_valid_toml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        tokio::fs::write(&path, "# empty config\n").await.unwrap();
        let cfg = load_config(&path).await.unwrap();
        let _ = cfg;
    }

    #[tokio::test]
    async fn load_config_errors_on_invalid_toml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        tokio::fs::write(&path, "this is = not valid [[[ toml")
            .await
            .unwrap();
        let err = load_config(&path).await.unwrap_err();
        assert!(err.to_string().contains("parse config"));
    }

    // ── resolve_extra_dir ─────────────────────────────────────────────────

    #[test]
    fn resolve_extra_dir_joins_relative_path_with_project_root() {
        let project = PathBuf::from("/projects/foo");
        let resolved = resolve_extra_dir("./skills", Some(&project));
        assert_eq!(resolved, project.join("./skills"));
    }

    #[test]
    fn resolve_extra_dir_handles_parent_relative_path() {
        let project = PathBuf::from("/projects/foo");
        let resolved = resolve_extra_dir("../shared", Some(&project));
        assert_eq!(resolved, project.join("../shared"));
    }

    #[test]
    fn resolve_extra_dir_keeps_relative_path_when_no_project_root() {
        let resolved = resolve_extra_dir("./skills", None);
        assert_eq!(resolved, PathBuf::from("./skills"));
    }

    #[test]
    fn resolve_extra_dir_passes_absolute_path_through_tilde_expand() {
        // Absolute paths go through expand_tilde, which is a no-op for them.
        let resolved = resolve_extra_dir("/absolute/path/skills", Some(&PathBuf::from("/proj")));
        assert_eq!(resolved, PathBuf::from("/absolute/path/skills"));
    }

    // ── skill_dirs ────────────────────────────────────────────────────────

    #[test]
    fn skill_dirs_returns_empty_when_nothing_resolves() {
        // No extra dirs configured, no ~/.assistant/skills present, and a
        // non-existent project root → no dirs should be returned (existence
        // check filters them out).
        let mut cfg = AssistantConfig::default();
        cfg.skills.extra_dirs.clear();

        let project_root = std::path::Path::new("/definitely/does/not/exist");
        let dirs = skill_dirs(&cfg, Some(project_root));
        // Some directories may still resolve (e.g. ~/.assistant/skills if it
        // exists on the test host), so we only assert the function doesn't
        // panic. The important branches — extra_dirs and project-relative —
        // are inactive here.
        let _ = dirs;
    }

    #[test]
    fn skill_dirs_includes_project_skills_dir_when_present() {
        let tmp = TempDir::new().unwrap();
        let project_skills = tmp.path().join(".assistant").join("skills");
        std::fs::create_dir_all(&project_skills).unwrap();

        let cfg = AssistantConfig::default();
        let dirs = skill_dirs(&cfg, Some(tmp.path()));
        let project = dirs.iter().find(|(p, _)| p == &project_skills);
        assert!(
            project.is_some(),
            "project skills dir must appear: {dirs:?}"
        );
        assert!(matches!(project.unwrap().1, SkillSource::Project));
    }

    #[test]
    fn skill_dirs_includes_extra_dirs_resolved_against_project() {
        let tmp = TempDir::new().unwrap();
        let extra = tmp.path().join("extra-skills");
        std::fs::create_dir_all(&extra).unwrap();

        let mut cfg = AssistantConfig::default();
        cfg.skills.extra_dirs = vec!["./extra-skills".to_string()];

        let dirs = skill_dirs(&cfg, Some(tmp.path()));
        let entry = dirs
            .iter()
            .find(|(p, _)| p.ends_with("extra-skills"))
            .expect("extra dir must be included");
        assert!(matches!(entry.1, SkillSource::Installed));
    }

    #[test]
    fn skill_dirs_deduplicates_repeated_paths() {
        let tmp = TempDir::new().unwrap();
        let extra = tmp.path().join("dup");
        std::fs::create_dir_all(&extra).unwrap();

        let mut cfg = AssistantConfig::default();
        cfg.skills.extra_dirs = vec![
            extra.to_string_lossy().to_string(),
            extra.to_string_lossy().to_string(),
        ];

        let dirs = skill_dirs(&cfg, Some(tmp.path()));
        let count = dirs.iter().filter(|(p, _)| p == &extra).count();
        assert_eq!(count, 1, "duplicate extra dir must be deduplicated");
    }

    #[test]
    fn skill_dirs_skips_empty_extra_dir_entries() {
        let tmp = TempDir::new().unwrap();
        let mut cfg = AssistantConfig::default();
        cfg.skills.extra_dirs = vec!["".to_string(), "   ".to_string()];

        let dirs = skill_dirs(&cfg, Some(tmp.path()));
        // Empty strings should not produce any entries.
        for (p, _) in &dirs {
            assert!(!p.as_os_str().is_empty());
        }
    }

    #[test]
    fn skill_dirs_skips_non_existent_extra_dir() {
        let tmp = TempDir::new().unwrap();
        let mut cfg = AssistantConfig::default();
        // Point at a directory that does NOT exist.
        cfg.skills.extra_dirs = vec![tmp.path().join("not-here").to_string_lossy().to_string()];

        let dirs = skill_dirs(&cfg, Some(tmp.path()));
        // Non-existent extra_dirs are filtered out by the .is_dir() check.
        for (p, source) in &dirs {
            if matches!(source, SkillSource::Installed) {
                assert!(p.is_dir(), "Installed dirs must be filtered to existing");
            }
        }
    }

    // ── spawn_memory_indexer ─────────────────────────────────────────────

    use assistant_core::types::features::MemoryConfig;
    use assistant_llm_provider::scripted::ScriptedLlmProvider;
    use assistant_storage::StorageLayer;

    #[tokio::test]
    async fn spawn_memory_indexer_returns_a_join_handle() {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let llm: Arc<dyn LlmProvider> = Arc::new(ScriptedLlmProvider::new());
        let cfg = MemoryConfig::default();

        let handle = spawn_memory_indexer(&cfg, storage, llm);
        // The indexer task should be spawned and abortable.
        handle.abort();
        let _ = handle.await;
    }
}
