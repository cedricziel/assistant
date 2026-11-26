//! Process-wide ([`ServerConfig`]) and per-organization ([`OrgConfig`])
//! configuration, and the active agent context (`AgentConfig`).

use serde::{Deserialize, Serialize};

use super::channels::{MatrixConfig, MattermostConfig, NextcloudConfig, SignalConfig, SlackConfig};
use super::features::{
    CompactionConfig, IntegrationsConfig, LearningConfig, MemoryConfig, NotificationsConfig,
    SuggestionsConfig, TitlingConfig, UiConfig,
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
/// the Web Push signing keys, and the integrations catalog source.
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
    /// Integrations catalog (`[integrations.catalog]`).
    #[serde(default)]
    pub integrations: IntegrationsConfig,
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
    /// Follow-up suggestion configuration (`[suggestions]` section).
    #[serde(default)]
    pub suggestions: SuggestionsConfig,
    /// Generative-UI settings (`[ui]` section); `None` when absent.
    #[serde(default)]
    pub ui: Option<UiConfig>,
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
    use std::time::Duration;

    use super::{OrgConfig, ServerConfig};
    use crate::types::features::{SUGGESTIONS_MAX_TIMEOUT_SECS, SuggestionsConfig};
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

    // -- SuggestionsConfig defaults / overrides / clamps ----------------------

    #[test]
    fn test_suggestions_defaults_when_block_absent() {
        let cfg: OrgConfig = toml::from_str("llm_provider = \"ollama\"").unwrap();
        let suggestions = cfg.suggestions;
        assert!(suggestions.enabled, "default: enabled = true");
        assert_eq!(suggestions.max_suggestions, 3);
        assert_eq!(suggestions.timeout_secs, 10);
    }

    #[test]
    fn test_suggestions_explicit_block_overrides_defaults() {
        let toml_str = r#"
            [suggestions]
            enabled = false
            max_suggestions = 5
            timeout_secs = 12
        "#;
        let cfg: OrgConfig = toml::from_str(toml_str).unwrap();
        assert!(!cfg.suggestions.enabled);
        assert_eq!(cfg.suggestions.max_suggestions, 5);
        assert_eq!(cfg.suggestions.timeout_secs, 12);
    }

    #[test]
    fn test_suggestions_effective_max_is_clamped() {
        let mut cfg = SuggestionsConfig::default();
        cfg.max_suggestions = 9;
        assert_eq!(cfg.effective_max(), 5, "9 clamps to 5");
        cfg.max_suggestions = 0;
        assert_eq!(cfg.effective_max(), 1, "0 clamps to 1");
        cfg.max_suggestions = 4;
        assert_eq!(cfg.effective_max(), 4, "in-range value is kept");
    }

    #[test]
    fn test_suggestions_effective_timeout_is_clamped() {
        let mut cfg = SuggestionsConfig::default();
        cfg.timeout_secs = 0;
        assert_eq!(cfg.effective_timeout(), Duration::from_secs(1));
        cfg.timeout_secs = 60;
        assert_eq!(
            cfg.effective_timeout(),
            Duration::from_secs(SUGGESTIONS_MAX_TIMEOUT_SECS)
        );
        assert_eq!(SUGGESTIONS_MAX_TIMEOUT_SECS, 15);
        cfg.timeout_secs = 7;
        assert_eq!(cfg.effective_timeout(), Duration::from_secs(7));
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

    // -- UiConfig (`[ui]`) ---------------------------------------------------

    #[test]
    fn ui_section_loads_both_host_lists() {
        let cfg: OrgConfig = toml::from_str(
            r#"
            [ui]
            image_hosts = ["a.example"]
            link_hosts = ["docs.example"]
            "#,
        )
        .unwrap();
        let ui = cfg.ui.expect("section present");
        assert_eq!(ui.image_hosts, vec!["a.example"]);
        assert_eq!(ui.link_hosts, vec!["docs.example"]);
    }

    #[test]
    fn ui_section_absent_yields_none() {
        let cfg: OrgConfig = toml::from_str("").unwrap();
        assert!(cfg.ui.is_none(), "no [ui] section means no UiConfig");
    }

    #[test]
    fn ui_absent_keys_yield_empty_lists() {
        let cfg: OrgConfig = toml::from_str("[ui]").unwrap();
        let ui = cfg.ui.expect("section present");
        assert!(
            ui.image_hosts.is_empty(),
            "empty image_hosts means any https host"
        );
        assert!(
            ui.link_hosts.is_empty(),
            "empty link_hosts means every link asks"
        );
    }

    #[test]
    fn ui_non_string_entry_fails_to_load() {
        let err = toml::from_str::<OrgConfig>("[ui]\nimage_hosts = [\"a.example\", 7]\n");
        assert!(err.is_err(), "a non-string host must not load");
    }

    #[test]
    fn ui_hosts_are_lower_cased_on_load() {
        let cfg: OrgConfig = toml::from_str(
            r#"
            [ui]
            image_hosts = ["Images.Example"]
            link_hosts = ["Docs.EXAMPLE"]
            "#,
        )
        .unwrap();
        let ui = cfg.ui.expect("section present");
        assert_eq!(ui.image_hosts, vec!["images.example"]);
        assert_eq!(ui.link_hosts, vec!["docs.example"]);
    }

    // -- Integrations catalog ------------------------------------------------

    #[test]
    fn integrations_catalog_defaults_when_section_absent() {
        let cfg: ServerConfig = toml::from_str("").unwrap();

        assert!(cfg.integrations.catalog.enabled, "default: enabled = true");
        assert_eq!(
            cfg.integrations.catalog.source_url,
            "https://integrations.sh/api.json"
        );
    }

    #[test]
    fn integrations_catalog_defaults_when_section_is_empty() {
        let cfg: ServerConfig = toml::from_str("[integrations.catalog]").unwrap();

        assert!(cfg.integrations.catalog.enabled);
        assert_eq!(
            cfg.integrations.catalog.source_url,
            "https://integrations.sh/api.json"
        );
    }

    #[test]
    fn integrations_catalog_can_be_disabled() {
        let toml_str = r#"
            [integrations.catalog]
            enabled = false
        "#;
        let cfg: ServerConfig = toml::from_str(toml_str).unwrap();

        assert!(!cfg.integrations.catalog.enabled);
        assert_eq!(
            cfg.integrations.catalog.source_url, "https://integrations.sh/api.json",
            "source_url keeps its default"
        );
    }

    #[test]
    fn integrations_catalog_source_url_can_point_at_a_mirror() {
        let toml_str = r#"
            [integrations.catalog]
            source_url = "https://mirror.internal/api.json"
        "#;
        let cfg: ServerConfig = toml::from_str(toml_str).unwrap();

        assert!(cfg.integrations.catalog.enabled);
        assert_eq!(
            cfg.integrations.catalog.source_url,
            "https://mirror.internal/api.json"
        );
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
