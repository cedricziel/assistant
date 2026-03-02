use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use gray_matter::{engine::YAML, Matter};
use serde::Deserialize;
use serde_json::Value;

use crate::skill::{SkillDef, SkillSource};

/// The raw YAML frontmatter fields from a SKILL.md (Agent Skills spec)
#[derive(Debug, Deserialize)]
struct Frontmatter {
    name: String,
    description: String,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    compatibility: Option<String>,
    #[serde(default, rename = "allowed-tools")]
    allowed_tools: Option<String>,
    #[serde(default)]
    metadata: HashMap<String, Value>,
    /// Capture unknown future frontmatter keys so deserialisation is forward-compatible.
    #[serde(flatten, default)]
    _extra: HashMap<String, Value>,
}

/// Parse a skill directory containing a SKILL.md file into a SkillDef.
///
/// The `source` parameter specifies where this skill was discovered
/// (builtin, user, project, installed).
pub fn parse_skill_dir(dir: &Path, source: SkillSource) -> Result<SkillDef> {
    let skill_md_path = dir.join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md_path)
        .with_context(|| format!("Failed to read SKILL.md at {}", skill_md_path.display()))?;

    parse_skill_content(&content, dir, source)
}

/// Parse SKILL.md content directly (useful for testing)
pub fn parse_skill_content(content: &str, dir: &Path, source: SkillSource) -> Result<SkillDef> {
    let matter: gray_matter::ParsedEntity<serde_json::Value> = Matter::<YAML>::new()
        .parse(content)
        .context("Failed to parse SKILL.md frontmatter")?;

    let front: Frontmatter = serde_json::from_value(
        matter
            .data
            .ok_or_else(|| anyhow::anyhow!("SKILL.md has no YAML frontmatter"))?,
    )
    .context("Failed to deserialize SKILL.md frontmatter")?;

    // Validate name: must be kebab-case and match the directory name when non-empty.
    if !is_kebab_case(&front.name) {
        anyhow::bail!(
            "SKILL.md name '{}' must be kebab-case (lowercase letters, digits, hyphens only)",
            front.name
        );
    }
    let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if !dir_name.is_empty() && front.name != dir_name {
        anyhow::bail!(
            "SKILL.md name '{}' must match the directory name '{}'",
            front.name,
            dir_name
        );
    }

    let description = normalize_description(&front.description)?;
    let compatibility = normalize_compatibility(front.compatibility)?;
    let allowed_tools = parse_allowed_tools(front.allowed_tools);
    let metadata = front.metadata;

    Ok(SkillDef {
        name: front.name,
        description,
        license: front.license,
        compatibility,
        allowed_tools,
        metadata,
        body: matter.content,
        dir: dir.to_path_buf(),
        source,
    })
}

/// Discover and parse all skill directories under a given root directory.
/// Returns only successfully parsed skills (logs errors for failed ones).
pub fn discover_skills(skills_root: &Path, source: SkillSource) -> Vec<SkillDef> {
    let Ok(entries) = std::fs::read_dir(skills_root) else {
        tracing::debug!(
            "Skill directory not found or unreadable: {}",
            skills_root.display()
        );
        return Vec::new();
    };

    let mut skills = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("SKILL.md").exists() {
            match parse_skill_dir(&path, source.clone()) {
                Ok(skill) => {
                    tracing::debug!("Loaded skill: {} from {}", skill.name, path.display());
                    skills.push(skill);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse skill at {}: {}", path.display(), e);
                }
            }
        }
    }
    skills
}

/// Returns `true` if `name` is a valid kebab-case identifier:
/// lowercase ASCII letters, digits, and interior hyphens only.
fn is_kebab_case(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn normalize_description(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("SKILL.md description must not be empty");
    }
    if trimmed.chars().count() > 1024 {
        anyhow::bail!("SKILL.md description must be ≤ 1024 characters");
    }
    Ok(trimmed.to_string())
}

fn normalize_compatibility(raw: Option<String>) -> Result<Option<String>> {
    match raw.map(|s| s.trim().to_string()) {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => {
            if s.chars().count() > 500 {
                anyhow::bail!("compatibility field must be ≤ 500 characters");
            }
            Ok(Some(s))
        }
        None => Ok(None),
    }
}

fn parse_allowed_tools(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect()
}

/// The bundled `skills/` directory embedded into the binary at compile time.
static EMBEDDED_SKILLS: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../skills");

/// Sync embedded builtin skills to a target directory on disk.
///
/// For each embedded skill, compare the on-disk `SKILL.md` content with the
/// embedded version.  If they differ (or the file is missing), write the
/// embedded version to disk, including any auxiliary files (scripts/,
/// references/, assets/).  Skills that are already up-to-date are skipped.
///
/// Returns the names of skills that were written or updated.
pub fn sync_builtins_to_disk(target_dir: &Path) -> Result<Vec<String>> {
    use std::fs;

    let mut updated = Vec::new();

    for entry in EMBEDDED_SKILLS.dirs() {
        let Some(skill_name) = entry.path().file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        let skill_md_rel = entry.path().join("SKILL.md");
        let Some(skill_md) = EMBEDDED_SKILLS.get_file(&skill_md_rel) else {
            continue;
        };
        let Some(embedded_content) = skill_md.contents_utf8() else {
            continue;
        };

        let target_skill_dir = target_dir.join(skill_name);
        let on_disk_path = target_skill_dir.join("SKILL.md");

        // Check whether the on-disk version matches the embedded content.
        let needs_update = match fs::read_to_string(&on_disk_path) {
            Ok(disk_content) => disk_content != embedded_content,
            Err(_) => true, // Missing or unreadable — write it.
        };

        if !needs_update {
            tracing::debug!(skill = %skill_name, "Built-in skill is up-to-date on disk");
            continue;
        }

        // Write SKILL.md.
        fs::create_dir_all(&target_skill_dir)?;
        fs::write(&on_disk_path, embedded_content)?;

        // Sync all other files in the embedded skill directory (auxiliary
        // files in scripts/, references/, assets/ subdirectories).
        sync_embedded_dir_recursive(entry, &target_skill_dir)?;

        tracing::info!(skill = %skill_name, "Updated built-in skill on disk");
        updated.push(skill_name.to_string());
    }

    Ok(updated)
}

/// Recursively write all files from an embedded `include_dir::Dir` to a
/// target filesystem directory, creating subdirectories as needed.
/// Skips `SKILL.md` (handled separately by the caller).
fn sync_embedded_dir_recursive(
    embedded_dir: &include_dir::Dir<'static>,
    target_dir: &Path,
) -> Result<()> {
    use std::fs;

    for file in embedded_dir.files() {
        let rel = file
            .path()
            .strip_prefix(embedded_dir.path())
            .unwrap_or(file.path());
        if rel == Path::new("SKILL.md") {
            continue;
        }
        let target_file = target_dir.join(rel);
        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target_file, file.contents())?;
    }

    // Recurse into subdirectories.
    for sub_dir in embedded_dir.dirs() {
        let sub_name = sub_dir
            .path()
            .strip_prefix(embedded_dir.path())
            .unwrap_or(sub_dir.path());
        let sub_target = target_dir.join(sub_name);
        // Re-use the same function, but pass the sub_dir as the root.
        // Since sub_dir.files() returns files relative to sub_dir.path(),
        // we handle them inline.
        for file in sub_dir.files() {
            let rel = file
                .path()
                .strip_prefix(embedded_dir.path())
                .unwrap_or(file.path());
            let target_file = target_dir.join(rel);
            if let Some(parent) = target_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target_file, file.contents())?;
        }
        // include_dir doesn't nest deeply for skills, but handle it.
        let _ = sub_target; // suppress unused warning
    }

    Ok(())
}

/// Parse and return all skills embedded in the binary via [`EMBEDDED_SKILLS`].
///
/// These are the `skills/` entries compiled into the binary at build time.
/// Each sub-directory containing a `SKILL.md` file is parsed and returned as
/// a [`SkillDef`] with [`SkillSource::Builtin`].  Skills that fail to parse
/// are logged as warnings and skipped.
pub fn embedded_builtin_skills() -> Vec<SkillDef> {
    let mut skills = Vec::new();

    for entry in EMBEDDED_SKILLS.dirs() {
        let skill_md_path = entry.path().join("SKILL.md");
        let Some(skill_md) = EMBEDDED_SKILLS.get_file(&skill_md_path) else {
            continue;
        };
        let Some(content) = skill_md.contents_utf8() else {
            tracing::warn!(
                "Embedded SKILL.md for '{}' is not valid UTF-8, skipping",
                entry.path().display()
            );
            continue;
        };

        match parse_skill_content(content, entry.path(), SkillSource::Builtin) {
            Ok(def) => {
                tracing::debug!("Embedded builtin skill loaded: {}", def.name);
                skills.push(def);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse embedded SKILL.md for '{}': {}",
                    entry.path().display(),
                    e
                );
            }
        }
    }

    skills
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const VALID_SKILL_MD: &str = r#"---
name: test-skill
description: A test skill for unit testing
license: MIT
allowed-tools: Bash jq
metadata:
  category: testing
---

## Instructions

This is a test skill.
"#;

    const MINIMAL_SKILL_MD: &str = r#"---
name: minimal
description: Minimal skill
---

Body text.
"#;

    const INVALID_SKILL_MD: &str = r#"No frontmatter here"#;

    #[test]
    fn test_parse_valid_skill() {
        let dir = PathBuf::from("/tmp/test-skill");
        let skill = parse_skill_content(VALID_SKILL_MD, &dir, SkillSource::Builtin).unwrap();

        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "A test skill for unit testing");
        assert_eq!(skill.license, Some("MIT".to_string()));
        assert_eq!(skill.allowed_tools, vec!["Bash", "jq"]);
        assert!(skill.body.contains("This is a test skill"));
    }

    #[test]
    fn test_parse_minimal_skill() {
        let dir = PathBuf::from("/tmp/minimal");
        let skill = parse_skill_content(MINIMAL_SKILL_MD, &dir, SkillSource::User).unwrap();

        assert_eq!(skill.name, "minimal");
        assert!(skill.body.contains("Body text"));
        assert!(skill.allowed_tools.is_empty());
    }

    #[test]
    fn test_parse_invalid_skill_fails() {
        let dir = PathBuf::from("/tmp/invalid");
        let result = parse_skill_content(INVALID_SKILL_MD, &dir, SkillSource::User);
        assert!(result.is_err());
    }

    #[test]
    fn test_description_validation() {
        let invalid = "---\nname: bad\ndescription:   \n---\n";
        let dir = PathBuf::from("/tmp/bad");
        let result = parse_skill_content(invalid, &dir, SkillSource::User);
        assert!(result.is_err());
    }

    #[test]
    fn test_compatibility_limit() {
        let long = format!(
            "---\nname: compat\ndescription: ok\ncompatibility: {}\n---\n",
            "x".repeat(501)
        );
        let dir = PathBuf::from("/tmp/compat");
        let result = parse_skill_content(&long, &dir, SkillSource::User);
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_builtins_to_disk_writes_missing_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let updated = sync_builtins_to_disk(tmp.path()).unwrap();

        // Should have written at least one skill.
        assert!(
            !updated.is_empty(),
            "sync should write at least one embedded skill"
        );

        // Each updated skill should have a SKILL.md on disk.
        for name in &updated {
            let skill_md = tmp.path().join(name).join("SKILL.md");
            assert!(skill_md.exists(), "SKILL.md should exist for {name}");
        }
    }

    #[test]
    fn test_sync_builtins_to_disk_skips_up_to_date() {
        let tmp = tempfile::tempdir().unwrap();

        // First sync — writes everything.
        let first = sync_builtins_to_disk(tmp.path()).unwrap();
        assert!(!first.is_empty());

        // Second sync — everything is up-to-date, nothing written.
        let second = sync_builtins_to_disk(tmp.path()).unwrap();
        assert!(second.is_empty(), "second sync should update nothing");
    }

    #[test]
    fn test_sync_builtins_to_disk_overwrites_stale() {
        let tmp = tempfile::tempdir().unwrap();

        // First sync.
        let first = sync_builtins_to_disk(tmp.path()).unwrap();
        assert!(!first.is_empty());

        // Tamper with the first skill's SKILL.md.
        let stale_path = tmp.path().join(&first[0]).join("SKILL.md");
        std::fs::write(&stale_path, "stale content").unwrap();

        // Second sync — should overwrite the stale file.
        let second = sync_builtins_to_disk(tmp.path()).unwrap();
        assert!(
            second.contains(&first[0]),
            "stale skill should be re-synced"
        );

        // Content should now match the embedded version again.
        let restored = std::fs::read_to_string(&stale_path).unwrap();
        assert_ne!(restored, "stale content");
    }
}
