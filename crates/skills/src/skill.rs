use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Category of auxiliary files in a skill directory
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxFileCategory {
    Scripts,
    References,
    Assets,
}

impl AuxFileCategory {
    /// Returns the subdirectory name for this category
    pub fn dir_name(&self) -> &'static str {
        match self {
            AuxFileCategory::Scripts => "scripts",
            AuxFileCategory::References => "references",
            AuxFileCategory::Assets => "assets",
        }
    }

    /// Returns a default MIME type for files in this category
    pub fn mime_type(&self) -> &'static str {
        match self {
            AuxFileCategory::Scripts => "text/plain",
            AuxFileCategory::References => "text/markdown",
            AuxFileCategory::Assets => "application/octet-stream",
        }
    }
}

/// Where a skill was discovered
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillSource {
    /// Shipped with the binary
    Builtin,
    /// From ~/.assistant/skills/
    User,
    /// From <project>/.assistant/skills/
    Project,
    /// Installed via /install command
    Installed,
}

impl std::fmt::Display for SkillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillSource::Builtin => write!(f, "builtin"),
            SkillSource::User => write!(f, "user"),
            SkillSource::Project => write!(f, "project"),
            SkillSource::Installed => write!(f, "installed"),
        }
    }
}

/// A pure knowledge package parsed from a SKILL.md directory.
///
/// A `SkillDef` is *not* executable — it contains the description, body
/// text, and auxiliary files that teach the agent how to do something.
/// To make knowledge available to the LLM, load a skill's body into
/// context via the `load_skill` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    // === Agent Skills spec fields ===
    /// kebab-case name, max 64 chars, matches directory name
    pub name: String,
    /// Human-readable description (max 1024 chars)
    pub description: String,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    /// Optional allow-list of tools (frontmatter `allowed-tools`)
    pub allowed_tools: Vec<String>,
    /// Raw `metadata` frontmatter entries (arbitrary key/value map)
    pub metadata: HashMap<String, Value>,

    // === Parsed body ===
    /// The Markdown instructions body from SKILL.md
    pub body: String,

    // === Filesystem location ===
    /// Filesystem path to the skill directory (e.g. ~/.assistant/skills/web-fetch/)
    pub dir: PathBuf,
    /// Where this skill was loaded from
    pub source: SkillSource,
}

impl SkillDef {
    /// Check if the skill is from a specific source
    pub fn is_builtin(&self) -> bool {
        matches!(self.source, SkillSource::Builtin)
    }

    /// Returns files in scripts/, references/, and assets/ subdirectories,
    /// as (category, relative_path_from_skill_root) pairs.
    pub fn auxiliary_files(&self) -> Vec<(AuxFileCategory, PathBuf)> {
        let categories = [
            AuxFileCategory::Scripts,
            AuxFileCategory::References,
            AuxFileCategory::Assets,
        ];

        let mut result = Vec::new();
        for category in &categories {
            let sub_dir = self.dir.join(category.dir_name());
            if !sub_dir.is_dir() {
                continue;
            }
            let entries = match fs::read_dir(&sub_dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') {
                        continue;
                    }
                }
                let relative = PathBuf::from(category.dir_name()).join(entry.file_name());
                result.push((category.clone(), relative));
            }
        }
        result
    }

    /// Returns true if the skill directory contains any auxiliary files.
    pub fn has_auxiliary_files(&self) -> bool {
        !self.auxiliary_files().is_empty()
    }
}
