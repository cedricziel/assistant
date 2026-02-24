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
    /// Captures all other frontmatter fields as raw JSON values.
    #[serde(flatten, default)]
    extra: HashMap<String, Value>,
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

    Ok(SkillDef {
        name: front.name,
        description: front.description,
        license: front.license,
        compatibility: front.compatibility,
        metadata: front.extra,
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

/// The bundled `skills/` directory embedded into the binary at compile time.
static EMBEDDED_SKILLS: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../skills");

/// Parse and return all skills embedded in the binary via [`EMBEDDED_SKILLS`].
///
/// These are the `skills/` entries compiled into the binary at build time.
/// Each sub-directory containing a `SKILL.md` file is parsed and returned as
/// a [`SkillDef`] with [`SkillSource::Builtin`].  Skills that fail to parse
/// are logged as warnings and skipped.
pub fn embedded_builtin_skills() -> Vec<SkillDef> {
    use std::path::PathBuf;

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

        let dir = PathBuf::from(entry.path());

        match parse_skill_content(content, &dir, SkillSource::Builtin) {
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
metadata:
  tier: builtin
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
        assert!(skill.body.contains("This is a test skill"));
    }

    #[test]
    fn test_parse_minimal_skill() {
        let dir = PathBuf::from("/tmp/minimal");
        let skill = parse_skill_content(MINIMAL_SKILL_MD, &dir, SkillSource::User).unwrap();

        assert_eq!(skill.name, "minimal");
        assert!(skill.body.contains("Body text"));
    }

    #[test]
    fn test_parse_invalid_skill_fails() {
        let dir = PathBuf::from("/tmp/invalid");
        let result = parse_skill_content(INVALID_SKILL_MD, &dir, SkillSource::User);
        assert!(result.is_err());
    }
}
