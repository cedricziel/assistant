//! Filesystem-backed store for managing multiple local agent cards.
//!
//! Each agent is persisted as a markdown file in `~/.assistant/agents/`.
//! The file uses YAML frontmatter for structured [`AgentCard`] fields and the
//! markdown body as the agent description.
//!
//! File naming convention: `<slugified-name>.md` (e.g., `my-cool-agent.md`).
//! The slug also serves as the agent ID.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use assistant_a2a_json_schema::agent_card::*;
use assistant_a2a_json_schema::security::{SecurityRequirement, SecurityScheme};

// -- Constants --

/// Name of the marker file that records which agent is the default.
const DEFAULT_MARKER: &str = ".default";

// -- Public types --

/// A registered agent entry loaded from disk.
#[derive(Debug, Clone)]
pub struct RegisteredAgent {
    /// The agent's ID (filename stem, e.g. `my-agent`).
    pub id: String,
    /// The full agent card.
    pub card: AgentCard,
    /// Whether this agent is the current default.
    pub is_default: bool,
}

/// Filesystem-backed store for multiple local agent cards.
///
/// Thread-safe and cheap to clone (just wraps a `PathBuf`).
#[derive(Debug, Clone)]
pub struct AgentStore {
    /// Root directory for agent markdown files.
    agents_dir: PathBuf,
}

// -- Frontmatter structure --

/// YAML frontmatter stored in each agent markdown file.
///
/// The `description` field lives in the markdown body, not the frontmatter.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentFrontmatter {
    name: String,
    version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    supported_interfaces: Vec<AgentInterface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider: Option<AgentProvider>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    documentation_url: Option<String>,
    #[serde(default)]
    capabilities: AgentCapabilities,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    security_schemes: HashMap<String, SecurityScheme>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    security_requirements: Vec<SecurityRequirement>,
    #[serde(default)]
    default_input_modes: Vec<String>,
    #[serde(default)]
    default_output_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skills: Vec<AgentSkill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon_url: Option<String>,
}

impl AgentStore {
    /// Creates a new store rooted at the given directory.
    ///
    /// The directory is created if it does not exist.
    pub fn new(agents_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&agents_dir)
            .with_context(|| format!("Failed to create agents dir: {}", agents_dir.display()))?;
        Ok(Self { agents_dir })
    }

    /// Creates a store using the default directory (`~/.assistant/agents/`).
    pub fn default_dir() -> Result<Self> {
        let base = dirs::home_dir()
            .map(|h| h.join(".assistant"))
            .unwrap_or_else(|| PathBuf::from(".assistant"));
        Self::new(base.join("agents"))
    }

    /// Returns the agents directory path.
    #[allow(dead_code)]
    pub fn dir(&self) -> &Path {
        &self.agents_dir
    }

    // -- CRUD operations --

    /// Registers a new agent card. Returns the assigned ID (filename slug).
    ///
    /// If `set_default` is true or this is the first agent, it becomes the
    /// default.
    pub async fn register(&self, card: AgentCard, set_default: bool) -> Result<String> {
        let id = slugify(&card.name);
        let path = self.agent_path(&id);

        // Avoid overwriting an existing agent -- append numeric suffix.
        let id = if path.exists() {
            let mut n = 2;
            loop {
                let candidate = format!("{id}-{n}");
                if !self.agent_path(&candidate).exists() {
                    break candidate;
                }
                n += 1;
            }
        } else {
            id
        };

        self.write_agent(&id, &card).await?;

        let is_first = self.list().await?.len() <= 1;
        if set_default || is_first {
            self.write_default_marker(&id).await?;
        }

        info!(agent_id = %id, name = %card.name, "Registered new agent");
        Ok(id)
    }

    /// Gets an agent by ID.
    pub async fn get(&self, id: &str) -> Option<RegisteredAgent> {
        let path = self.agent_path(id);
        if !path.exists() {
            return None;
        }
        let default_id = self.read_default_marker().await;
        match self.read_agent(id, &path).await {
            Ok(mut agent) => {
                agent.is_default = default_id.as_deref() == Some(id);
                Some(agent)
            }
            Err(e) => {
                warn!(agent_id = %id, err = %e, "Failed to read agent file");
                None
            }
        }
    }

    /// Gets the default agent.
    pub async fn get_default(&self) -> Option<RegisteredAgent> {
        let default_id = self.read_default_marker().await?;
        self.get(&default_id).await
    }

    /// Lists all registered agents, sorted by name.
    pub async fn list(&self) -> Result<Vec<RegisteredAgent>> {
        let default_id = self.read_default_marker().await;
        let mut agents = Vec::new();

        let entries = std::fs::read_dir(&self.agents_dir)
            .with_context(|| format!("Failed to read agents dir: {}", self.agents_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                continue;
            }
            match self.read_agent(&id, &path).await {
                Ok(mut agent) => {
                    agent.is_default = default_id.as_deref() == Some(&*id);
                    agents.push(agent);
                }
                Err(e) => {
                    warn!(path = %path.display(), err = %e, "Skipping malformed agent file");
                }
            }
        }

        agents.sort_by(|a, b| a.card.name.cmp(&b.card.name));
        Ok(agents)
    }

    /// Updates an existing agent's card. Returns `true` if found.
    pub async fn update(&self, id: &str, card: AgentCard) -> bool {
        let path = self.agent_path(id);
        if !path.exists() {
            return false;
        }
        match self.write_agent(id, &card).await {
            Ok(()) => {
                debug!(agent_id = %id, "Updated agent");
                true
            }
            Err(e) => {
                warn!(agent_id = %id, err = %e, "Failed to update agent");
                false
            }
        }
    }

    /// Sets an agent as the default. Returns `true` if the agent exists.
    pub async fn set_default(&self, id: &str) -> bool {
        let path = self.agent_path(id);
        if !path.exists() {
            return false;
        }
        match self.write_default_marker(id).await {
            Ok(()) => true,
            Err(e) => {
                warn!(agent_id = %id, err = %e, "Failed to set default agent");
                false
            }
        }
    }

    /// Removes an agent by ID. Returns `true` if it existed.
    pub async fn remove(&self, id: &str) -> bool {
        let path = self.agent_path(id);
        if !path.exists() {
            return false;
        }

        if let Err(e) = tokio::fs::remove_file(&path).await {
            warn!(agent_id = %id, err = %e, "Failed to remove agent file");
            return false;
        }

        info!(agent_id = %id, "Removed agent");

        // If we removed the default, pick a new one.
        let default_id = self.read_default_marker().await;
        if default_id.as_deref() == Some(id) {
            if let Ok(agents) = self.list().await {
                if let Some(first) = agents.first() {
                    let _ = self.write_default_marker(&first.id).await;
                } else {
                    // No agents left -- remove the marker.
                    let marker = self.agents_dir.join(DEFAULT_MARKER);
                    let _ = tokio::fs::remove_file(&marker).await;
                }
            }
        }

        true
    }

    /// Returns the number of registered agents.
    #[allow(dead_code)]
    pub async fn count(&self) -> usize {
        self.list().await.map(|v| v.len()).unwrap_or(0)
    }

    // -- Internal helpers --

    fn agent_path(&self, id: &str) -> PathBuf {
        self.agents_dir.join(format!("{id}.md"))
    }

    /// Writes an agent card to disk as a markdown file with YAML frontmatter.
    async fn write_agent(&self, id: &str, card: &AgentCard) -> Result<()> {
        let frontmatter = AgentFrontmatter {
            name: card.name.clone(),
            version: card.version.clone(),
            supported_interfaces: card.supported_interfaces.clone(),
            provider: card.provider.clone(),
            documentation_url: card.documentation_url.clone(),
            capabilities: card.capabilities.clone(),
            security_schemes: card.security_schemes.clone(),
            security_requirements: card.security_requirements.clone(),
            default_input_modes: card.default_input_modes.clone(),
            default_output_modes: card.default_output_modes.clone(),
            skills: card.skills.clone(),
            icon_url: card.icon_url.clone(),
        };

        let yaml = serde_yaml::to_string(&frontmatter)
            .with_context(|| format!("Failed to serialize frontmatter for agent '{id}'"))?;

        let content = format!("---\n{yaml}---\n\n{}", card.description);

        let path = self.agent_path(id);
        tokio::fs::write(&path, content.as_bytes())
            .await
            .with_context(|| format!("Failed to write agent file: {}", path.display()))?;

        Ok(())
    }

    /// Reads an agent card from a markdown file with YAML frontmatter.
    async fn read_agent(&self, id: &str, path: &Path) -> Result<RegisteredAgent> {
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read agent file: {}", path.display()))?;

        let (frontmatter, description) = parse_frontmatter(&content)
            .with_context(|| format!("Failed to parse frontmatter in: {}", path.display()))?;

        let fm: AgentFrontmatter = serde_yaml::from_str(&frontmatter)
            .with_context(|| format!("Invalid YAML frontmatter in: {}", path.display()))?;

        let card = AgentCard {
            name: fm.name,
            description,
            supported_interfaces: fm.supported_interfaces,
            provider: fm.provider,
            version: fm.version,
            documentation_url: fm.documentation_url,
            capabilities: fm.capabilities,
            security_schemes: fm.security_schemes,
            security_requirements: fm.security_requirements,
            default_input_modes: fm.default_input_modes,
            default_output_modes: fm.default_output_modes,
            skills: fm.skills,
            signatures: vec![],
            icon_url: fm.icon_url,
        };

        Ok(RegisteredAgent {
            id: id.to_string(),
            card,
            is_default: false, // caller sets this
        })
    }

    /// Reads the default agent ID from the marker file.
    async fn read_default_marker(&self) -> Option<String> {
        let marker = self.agents_dir.join(DEFAULT_MARKER);
        tokio::fs::read_to_string(&marker)
            .await
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Writes the default agent ID to the marker file.
    async fn write_default_marker(&self, id: &str) -> Result<()> {
        let marker = self.agents_dir.join(DEFAULT_MARKER);
        tokio::fs::write(&marker, id.as_bytes())
            .await
            .with_context(|| "Failed to write default agent marker")?;
        Ok(())
    }
}

// -- Utility functions --

/// Converts a name to a URL/filename-safe slug.
fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    // Collapse consecutive dashes and trim edges.
    let mut result = String::new();
    let mut prev_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash && !result.is_empty() {
                result.push('-');
            }
            prev_dash = true;
        } else {
            result.push(c);
            prev_dash = false;
        }
    }
    result.trim_end_matches('-').to_string()
}

/// Parses YAML frontmatter delimited by `---` from markdown content.
///
/// Returns `(frontmatter_yaml, body)`.
fn parse_frontmatter(content: &str) -> Result<(String, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        anyhow::bail!("Missing frontmatter delimiter");
    }

    let after_first = &trimmed[3..];
    let after_first = after_first.strip_prefix('\n').unwrap_or(after_first);

    let end = after_first
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("Missing closing frontmatter delimiter"))?;

    let frontmatter = after_first[..end].to_string();
    let body_start = end + 4; // skip "\n---"
    let body = if body_start < after_first.len() {
        after_first[body_start..].trim().to_string()
    } else {
        String::new()
    };

    Ok((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_card(name: &str) -> AgentCard {
        AgentCard {
            name: name.to_string(),
            description: format!("{name} agent"),
            supported_interfaces: vec![AgentInterface {
                url: "https://example.com".to_string(),
                protocol_binding: "HTTP+JSON".to_string(),
                tenant: None,
                protocol_version: "1.0".to_string(),
            }],
            provider: None,
            version: "1.0.0".to_string(),
            documentation_url: None,
            capabilities: AgentCapabilities::default(),
            security_schemes: HashMap::new(),
            security_requirements: vec![],
            default_input_modes: vec!["text/plain".to_string()],
            default_output_modes: vec!["text/plain".to_string()],
            skills: vec![],
            signatures: vec![],
            icon_url: None,
        }
    }

    #[tokio::test]
    async fn test_register_and_get() {
        let tmp = TempDir::new().unwrap();
        let store = AgentStore::new(tmp.path().to_path_buf()).unwrap();
        let id = store.register(make_card("Alpha"), false).await.unwrap();
        assert_eq!(id, "alpha");

        let agent = store.get(&id).await.unwrap();
        assert_eq!(agent.card.name, "Alpha");
        assert_eq!(agent.card.description, "Alpha agent");
        assert!(agent.is_default, "first agent should be default");
    }

    #[tokio::test]
    async fn test_default_assignment() {
        let tmp = TempDir::new().unwrap();
        let store = AgentStore::new(tmp.path().to_path_buf()).unwrap();
        let id1 = store.register(make_card("First"), false).await.unwrap();
        let id2 = store.register(make_card("Second"), true).await.unwrap();

        let a1 = store.get(&id1).await.unwrap();
        let a2 = store.get(&id2).await.unwrap();
        assert!(!a1.is_default);
        assert!(a2.is_default);

        let default = store.get_default().await.unwrap();
        assert_eq!(default.id, id2);
    }

    #[tokio::test]
    async fn test_update() {
        let tmp = TempDir::new().unwrap();
        let store = AgentStore::new(tmp.path().to_path_buf()).unwrap();
        let id = store.register(make_card("Old"), false).await.unwrap();
        assert!(store.update(&id, make_card("New")).await);
        assert_eq!(store.get(&id).await.unwrap().card.name, "New");
    }

    #[tokio::test]
    async fn test_remove_reassigns_default() {
        let tmp = TempDir::new().unwrap();
        let store = AgentStore::new(tmp.path().to_path_buf()).unwrap();
        let id1 = store.register(make_card("First"), false).await.unwrap();
        let _id2 = store.register(make_card("Second"), false).await.unwrap();

        assert!(store.remove(&id1).await);
        assert!(store.get(&id1).await.is_none());

        let default = store.get_default().await.unwrap();
        assert_eq!(default.card.name, "Second");
    }

    #[tokio::test]
    async fn test_list_sorted() {
        let tmp = TempDir::new().unwrap();
        let store = AgentStore::new(tmp.path().to_path_buf()).unwrap();
        store.register(make_card("Zeta"), false).await.unwrap();
        store.register(make_card("Alpha"), false).await.unwrap();
        store.register(make_card("Mid"), false).await.unwrap();

        let list = store.list().await.unwrap();
        let names: Vec<_> = list.iter().map(|a| a.card.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Mid", "Zeta"]);
    }

    #[tokio::test]
    async fn test_slugify() {
        assert_eq!(slugify("My Cool Agent"), "my-cool-agent");
        assert_eq!(slugify("  Hello  World  "), "hello-world");
        assert_eq!(slugify("Agent v2.0"), "agent-v2-0");
    }

    #[tokio::test]
    async fn test_duplicate_name_gets_suffix() {
        let tmp = TempDir::new().unwrap();
        let store = AgentStore::new(tmp.path().to_path_buf()).unwrap();
        let id1 = store.register(make_card("Agent"), false).await.unwrap();
        let id2 = store.register(make_card("Agent"), false).await.unwrap();
        assert_eq!(id1, "agent");
        assert_eq!(id2, "agent-2");
    }

    #[tokio::test]
    async fn test_frontmatter_roundtrip() {
        let content = "---\nname: Test\nversion: '1.0'\n---\n\nHello world";
        let (yaml, body) = parse_frontmatter(content).unwrap();
        assert!(yaml.contains("name: Test"));
        assert_eq!(body, "Hello world");
    }

    #[tokio::test]
    async fn test_file_persisted_on_disk() {
        let tmp = TempDir::new().unwrap();
        let store = AgentStore::new(tmp.path().to_path_buf()).unwrap();
        let id = store.register(make_card("Persisted"), false).await.unwrap();

        // Verify the file exists.
        let path = tmp.path().join(format!("{id}.md"));
        assert!(path.exists(), "Agent file should exist on disk");

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("name: Persisted"));
        assert!(content.contains("Persisted agent"));
    }
}
