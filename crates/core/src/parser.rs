use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use gray_matter::{engine::YAML, Matter};
use serde::Deserialize;

use crate::skill::{SkillDef, SkillSource, SkillTier};

/// The raw YAML frontmatter fields from a SKILL.md (Agent Skills spec)
#[derive(Debug, Deserialize)]
struct Frontmatter {
    name: String,
    description: String,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    compatibility: Option<String>,
    #[serde(rename = "allowed-tools", default)]
    allowed_tools: Vec<String>,
    #[serde(default)]
    metadata: HashMap<String, String>,
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
    let matter = Matter::<YAML>::new().parse(content);

    let front: Frontmatter = matter
        .data
        .ok_or_else(|| anyhow::anyhow!("SKILL.md has no YAML frontmatter"))?
        .deserialize()
        .context("Failed to deserialize SKILL.md frontmatter")?;

    // Derive execution tier from metadata.tier
    let tier = derive_tier(&front.metadata, dir);

    // Derive boolean flags from metadata
    let mutating = front
        .metadata
        .get("mutating")
        .map(|v| v == "true")
        .unwrap_or(false);

    let confirmation_required = front
        .metadata
        .get("confirmation-required")
        .map(|v| v == "true")
        .unwrap_or(false);

    Ok(SkillDef {
        name: front.name,
        description: front.description,
        license: front.license,
        compatibility: front.compatibility,
        allowed_tools: front.allowed_tools,
        metadata: front.metadata,
        body: matter.content,
        dir: dir.to_path_buf(),
        tier,
        mutating,
        confirmation_required,
        source,
    })
}

/// Derive a SkillTier from the `metadata.tier` field and the skill directory.
fn derive_tier(metadata: &HashMap<String, String>, dir: &Path) -> SkillTier {
    match metadata.get("tier").map(String::as_str) {
        Some("script") => {
            // Look for an entrypoint in scripts/
            let scripts_dir = dir.join("scripts");
            let entrypoint = if let Some(ep) = metadata.get("entrypoint") {
                scripts_dir.join(ep)
            } else {
                // Default: look for any executable in scripts/
                scripts_dir.join("run")
            };
            SkillTier::Script { entrypoint }
        }
        Some("wasm") => {
            let plugin = dir.join("plugin.wasm");
            SkillTier::Wasm { plugin }
        }
        Some("builtin") => SkillTier::Builtin,
        // Default for user skills with no tier specified: prompt
        _ => SkillTier::Prompt,
    }
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
  mutating: "false"
  confirmation-required: "false"
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
        assert_eq!(skill.tier, SkillTier::Builtin);
        assert!(!skill.mutating);
        assert!(!skill.confirmation_required);
        assert_eq!(skill.license, Some("MIT".to_string()));
        assert!(skill.body.contains("This is a test skill"));
    }

    #[test]
    fn test_parse_minimal_skill() {
        let dir = PathBuf::from("/tmp/minimal");
        let skill = parse_skill_content(MINIMAL_SKILL_MD, &dir, SkillSource::User).unwrap();

        assert_eq!(skill.name, "minimal");
        // Default tier when none specified: Prompt
        assert_eq!(skill.tier, SkillTier::Prompt);
        assert!(!skill.mutating);
    }

    #[test]
    fn test_parse_invalid_skill_fails() {
        let dir = PathBuf::from("/tmp/invalid");
        let result = parse_skill_content(INVALID_SKILL_MD, &dir, SkillSource::User);
        assert!(result.is_err());
    }
}
