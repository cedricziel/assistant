//! Shared bootstrap helpers for interface binaries.
//!
//! These utilities are used by the Slack and Mattermost interface binaries to
//! reduce code duplication for common startup tasks: finding skill
//! directories.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use assistant_core::types::features::MemoryConfig;
use assistant_core::{LlmProvider, expand_tilde, types::agent::OrgConfig};
use assistant_skills::SkillSource;
use assistant_storage::StorageLayer;

// ── Skill directories ──────────────────────────────────────────────────────────

/// Return the runtime skill search directories.
///
/// Builtin skills are embedded into the binary via [`embedded_builtin_skills`]
/// and do not require a filesystem path.  This function only returns directories
/// for runtime-discovered skills:
///
/// * Entries from `skills.extra_dirs` (home-relative, absolute, or project-relative)
/// * `<install_root>/skills/` — user-installed skills
/// * `<project>/.assistant/skills/` — project-scoped skills (if available)
/// * `<exe_dir>/skills/` — optional sidecar skills shipped alongside the binary
pub fn skill_dirs(
    install_root: &Path,
    config: &OrgConfig,
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

    push_dir(install_root.join("skills"), SkillSource::User);

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
    fn skill_dirs_includes_the_install_roots_skills_dir() {
        let install_root = TempDir::new().unwrap();
        let user_skills = install_root.path().join("skills");
        std::fs::create_dir_all(&user_skills).unwrap();

        let dirs = skill_dirs(install_root.path(), &OrgConfig::default(), None);
        let user = dirs.iter().find(|(p, _)| p == &user_skills);
        assert!(user.is_some(), "user skills dir must appear: {dirs:?}");
        assert!(matches!(user.unwrap().1, SkillSource::User));
    }

    #[test]
    fn skill_dirs_includes_project_skills_dir_when_present() {
        let tmp = TempDir::new().unwrap();
        let project_skills = tmp.path().join(".assistant").join("skills");
        std::fs::create_dir_all(&project_skills).unwrap();

        let cfg = OrgConfig::default();
        let dirs = skill_dirs(tmp.path(), &cfg, Some(tmp.path()));
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

        let mut cfg = OrgConfig::default();
        cfg.skills.extra_dirs = vec!["./extra-skills".to_string()];

        let dirs = skill_dirs(tmp.path(), &cfg, Some(tmp.path()));
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

        let mut cfg = OrgConfig::default();
        cfg.skills.extra_dirs = vec![
            extra.to_string_lossy().to_string(),
            extra.to_string_lossy().to_string(),
        ];

        let dirs = skill_dirs(tmp.path(), &cfg, Some(tmp.path()));
        let count = dirs.iter().filter(|(p, _)| p == &extra).count();
        assert_eq!(count, 1, "duplicate extra dir must be deduplicated");
    }

    #[test]
    fn skill_dirs_skips_empty_extra_dir_entries() {
        let tmp = TempDir::new().unwrap();
        let mut cfg = OrgConfig::default();
        cfg.skills.extra_dirs = vec!["".to_string(), "   ".to_string()];

        let dirs = skill_dirs(tmp.path(), &cfg, Some(tmp.path()));
        // Empty strings should not produce any entries.
        for (p, _) in &dirs {
            assert!(!p.as_os_str().is_empty());
        }
    }

    #[test]
    fn skill_dirs_skips_non_existent_extra_dir() {
        let tmp = TempDir::new().unwrap();
        let mut cfg = OrgConfig::default();
        // Point at a directory that does NOT exist.
        cfg.skills.extra_dirs = vec![tmp.path().join("not-here").to_string_lossy().to_string()];

        let dirs = skill_dirs(tmp.path(), &cfg, Some(tmp.path()));
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
