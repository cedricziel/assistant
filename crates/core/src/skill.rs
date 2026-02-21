use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::ExecutionContext;

/// How a skill gets executed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SkillTier {
    /// The LLM interprets SKILL.md body as a sub-prompt (no external execution)
    Prompt,
    /// A script in `scripts/<entrypoint>` is run as a sandboxed subprocess
    Script { entrypoint: PathBuf },
    /// An extism WASM plugin at `plugin.wasm` in the skill dir
    Wasm { plugin: PathBuf },
    /// Rust handler registered at startup; SKILL.md is synthetic documentation
    Builtin,
}

impl SkillTier {
    /// Human-readable short label for display
    pub fn label(&self) -> &'static str {
        match self {
            SkillTier::Prompt => "prompt",
            SkillTier::Script { .. } => "script",
            SkillTier::Wasm { .. } => "wasm",
            SkillTier::Builtin => "builtin",
        }
    }
}

impl std::fmt::Display for SkillTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

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

/// A parsed skill definition (from SKILL.md frontmatter + body)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    // === Agent Skills spec fields ===
    /// kebab-case name, max 64 chars, matches directory name
    pub name: String,
    /// Human-readable description (max 1024 chars) — injected into LLM system prompt
    pub description: String,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    /// Tools this skill is allowed to call (Agent Skills spec field)
    pub allowed_tools: Vec<String>,
    /// Raw frontmatter metadata key/value pairs
    pub metadata: HashMap<String, String>,

    // === Parsed body ===
    /// The Markdown instructions body from SKILL.md (used for prompt-tier and sub-prompt injection)
    pub body: String,

    // === Runtime extensions ===
    /// Filesystem path to the skill directory (e.g. ~/.assistant/skills/web-fetch/)
    pub dir: PathBuf,
    /// Execution tier (derived from metadata.tier)
    pub tier: SkillTier,
    /// Whether this skill mutates state (from metadata.mutating)
    pub mutating: bool,
    /// Whether this skill requires user confirmation before execution (from metadata.confirmation-required)
    pub confirmation_required: bool,
    /// Where this skill was loaded from
    pub source: SkillSource,
}

impl SkillDef {
    /// Returns the JSON schema for this skill's parameters, if defined in metadata
    pub fn params_schema(&self) -> Option<Value> {
        self.metadata
            .get("params")
            .and_then(|s| serde_json::from_str(s).ok())
    }

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

/// The output of a skill execution
#[derive(Debug, Clone)]
pub struct SkillOutput {
    /// The text content returned by the skill
    pub content: String,
    /// Whether the skill completed successfully
    pub success: bool,
    /// Optional structured data alongside text content
    pub data: Option<Value>,
}

impl SkillOutput {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            success: true,
            data: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: message.into(),
            success: false,
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// The trait every skill handler must implement
#[async_trait]
pub trait SkillHandler: Send + Sync {
    /// The skill name this handler handles (must match SkillDef.name)
    fn skill_name(&self) -> &str;

    /// Execute the skill with the given parameters
    async fn execute(
        &self,
        def: &SkillDef,
        params: HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> anyhow::Result<SkillOutput>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn make_skill(dir: &std::path::Path) -> SkillDef {
        SkillDef {
            name: "test-skill".to_string(),
            description: "Test skill".to_string(),
            license: None,
            compatibility: None,
            allowed_tools: vec![],
            metadata: HashMap::new(),
            body: String::new(),
            dir: dir.to_path_buf(),
            tier: SkillTier::Builtin,
            mutating: false,
            confirmation_required: false,
            source: SkillSource::Builtin,
        }
    }

    #[test]
    fn auxiliary_files_empty_when_no_subdirs() {
        let tmp = TempDir::new().unwrap();
        let skill = make_skill(tmp.path());
        assert!(skill.auxiliary_files().is_empty());
        assert!(!skill.has_auxiliary_files());
    }

    #[test]
    fn auxiliary_files_returns_references() {
        let tmp = TempDir::new().unwrap();
        let refs_dir = tmp.path().join("references");
        std::fs::create_dir(&refs_dir).unwrap();
        std::fs::write(refs_dir.join("REFERENCE.md"), "# Reference").unwrap();

        let skill = make_skill(tmp.path());
        let files = skill.auxiliary_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, AuxFileCategory::References);
        assert_eq!(
            files[0].1,
            std::path::PathBuf::from("references/REFERENCE.md")
        );
    }

    #[test]
    fn auxiliary_files_skips_hidden_files() {
        let tmp = TempDir::new().unwrap();
        let refs_dir = tmp.path().join("references");
        std::fs::create_dir(&refs_dir).unwrap();
        std::fs::write(refs_dir.join(".hidden"), "hidden").unwrap();
        std::fs::write(refs_dir.join("visible.md"), "visible").unwrap();

        let skill = make_skill(tmp.path());
        let files = skill.auxiliary_files();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].1.file_name().unwrap().to_str().unwrap(),
            "visible.md"
        );
    }

    #[test]
    fn auxiliary_files_all_categories() {
        let tmp = TempDir::new().unwrap();
        for dir_name in &["scripts", "references", "assets"] {
            let sub_dir = tmp.path().join(dir_name);
            std::fs::create_dir(&sub_dir).unwrap();
            std::fs::write(sub_dir.join("file.txt"), "content").unwrap();
        }

        let skill = make_skill(tmp.path());
        let files = skill.auxiliary_files();
        assert_eq!(files.len(), 3);

        let category_dirs: std::collections::HashSet<String> = files
            .iter()
            .map(|(cat, _)| cat.dir_name().to_string())
            .collect();
        assert!(category_dirs.contains("scripts"));
        assert!(category_dirs.contains("references"));
        assert!(category_dirs.contains("assets"));
    }

    #[test]
    fn auxiliary_files_ignores_unknown_subdirs() {
        let tmp = TempDir::new().unwrap();
        // Create a dir that is NOT one of the three known categories
        let unknown = tmp.path().join("unknown");
        std::fs::create_dir(&unknown).unwrap();
        std::fs::write(unknown.join("file.txt"), "content").unwrap();

        let skill = make_skill(tmp.path());
        assert!(skill.auxiliary_files().is_empty());
    }
}
