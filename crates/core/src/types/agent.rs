//! Process-wide ([`ServerConfig`]) and per-organization ([`OrgConfig`])
//! configuration, and the active agent context (`AgentConfig`).

use serde::{Deserialize, Serialize};

use super::channels::{MatrixConfig, MattermostConfig, NextcloudConfig, SignalConfig, SlackConfig};
use super::features::{
    CompactionConfig, LearningConfig, MemoryConfig, NotificationsConfig, TitlingConfig,
};
use super::llm::LlmConfig;
use super::observability::ObservabilityConfig;
use super::permissions::PermissionsConfig;
use super::skills_mcp::{McpConfig, SkillsConfig};
use super::storage::BusConfig;
use super::transcription::{TranscriptionConfig, TtsConfig};

/// Process-wide settings, read once at startup from `~/.assistant/server.toml`.
///
/// Holds what every org and space in the process shares: the message-bus
/// backend, the external MCP servers whose tools are registered everywhere, and
/// the Web Push signing keys.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Message-bus backend (`[bus]`).
    #[serde(default)]
    pub bus: BusConfig,
    /// External MCP servers bridged into every space (`[mcp]`).
    #[serde(default)]
    pub mcp: McpConfig,
    /// Web Push / VAPID keys (`[notifications]`).
    #[serde(default)]
    pub notifications: NotificationsConfig,
}

/// Per-organization settings, read from `~/.assistant/orgs/{slug}/org.toml`.
///
/// Everything a tenant controls: which LLM and speech providers it uses, how
/// its persona and memory behave, and the credentials of its messenger
/// interfaces.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrgConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
    /// Observability settings that depend on the org's data (`[observability]`).
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub compaction: CompactionConfig,
    /// Autonomous learning configuration (`[learning]` section).
    #[serde(default)]
    pub learning: LearningConfig,
    /// Conversation title-generator configuration (`[titling]` section).
    #[serde(default)]
    pub titling: TitlingConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    /// Tool-permission rules and decision model (`[permissions]` section).
    #[serde(default)]
    pub permissions: PermissionsConfig,
    /// Signal messenger interface configuration (optional).
    pub signal: Option<SignalConfig>,
    /// Slack interface configuration (optional).
    pub slack: Option<SlackConfig>,
    /// Mattermost interface configuration (optional).
    pub mattermost: Option<MattermostConfig>,
    /// Nextcloud Talk interface configuration (optional).
    pub nextcloud: Option<NextcloudConfig>,
    /// Matrix interface configuration (optional).
    pub matrix: Option<MatrixConfig>,
    /// Audio transcription configuration (optional).
    #[serde(default)]
    pub transcription: Option<TranscriptionConfig>,
    /// Text-to-speech configuration (optional).
    #[serde(default)]
    pub tts: Option<TtsConfig>,
}

/// Active assistant agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_agent_id")]
    pub id: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            id: default_agent_id(),
        }
    }
}

fn default_agent_id() -> String {
    "default".to_string()
}

#[cfg(test)]
mod tests {
    use super::OrgConfig;
    use crate::types::llm::{EmbeddingProviderKind, LlmProviderKind};

    // -- TitlingConfig defaults / overrides ----------------------------------

    #[test]
    fn test_titling_defaults_when_block_absent() {
        // An OrgConfig with no [titling] section must deserialize cleanly and
        // the worker should pick up the default values.
        let toml_str = "llm_provider = \"ollama\"";
        let cfg: OrgConfig = toml::from_str(toml_str).unwrap();
        let titling = cfg.titling;
        assert!(titling.enabled, "default: enabled = true");
        assert_eq!(titling.min_turns, 2);
        assert_eq!(titling.long_first_message_chars, 200);
    }

    #[test]
    fn test_titling_explicit_block_overrides_defaults() {
        let toml_str = r#"
            llm_provider = "ollama"

            [titling]
            enabled = false
            min_turns = 4
            long_first_message_chars = 500
        "#;
        let cfg: OrgConfig = toml::from_str(toml_str).unwrap();
        let titling = cfg.titling;
        assert!(!titling.enabled);
        assert_eq!(titling.min_turns, 4);
        assert_eq!(titling.long_first_message_chars, 500);
    }

    // -- OrgConfig with embeddings section -----------------------------

    #[test]
    fn full_assistant_config_with_embeddings() {
        let toml_str = r#"
            [llm]
            provider = "anthropic"
            model = "claude-opus-4-6"
            api_key = "sk-ant-test"

            [llm.embeddings]
            provider = "openai"
            model = "text-embedding-3-small"
            api_key = "sk-openai-test"
        "#;
        let cfg: OrgConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.llm.provider, LlmProviderKind::Anthropic);
        let emb = cfg.llm.embeddings.expect("embeddings should be Some");
        assert_eq!(emb.provider, EmbeddingProviderKind::OpenAI);
        assert_eq!(emb.model.as_deref(), Some("text-embedding-3-small"));
        assert_eq!(
            emb.api_key.as_ref().map(crate::secret::Secret::expose),
            Some("sk-openai-test")
        );
    }

    #[test]
    fn full_assistant_config_without_embeddings() {
        let toml_str = r#"
            [llm]
            provider = "ollama"
            model = "qwen2.5:7b"
        "#;
        let cfg: OrgConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.llm.provider, LlmProviderKind::Ollama);
        assert!(cfg.llm.embeddings.is_none());
    }
}
