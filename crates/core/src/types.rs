use serde::{Deserialize, Serialize};

use self::channels::{MatrixConfig, MattermostConfig, NextcloudConfig, SignalConfig, SlackConfig};
use self::features::{
    CompactionConfig, LearningConfig, MemoryConfig, NotificationsConfig, TitlingConfig,
};
use self::llm::LlmConfig;
use self::observability::ObservabilityConfig;
use self::skills_mcp::{McpConfig, MirrorConfig, SkillsConfig};
use self::storage::{BusConfig, StorageConfig};
use self::transcription::{TranscriptionConfig, TtsConfig};

#[doc(hidden)]
pub(crate) fn default_true() -> bool {
    true
}

// ── Conversation types ────────────────────────────────────────────────────────
//
// Moved to the `conversation` submodule. Import via
// `use assistant_core::types::conversation::{Message, MessageRole, ...};`
pub mod conversation;

/// Top-level assistant configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssistantConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub bus: BusConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    /// Deprecated — use `[observability]` instead. Kept for backwards compatibility.
    #[serde(default, alias = "self_improvement", skip_serializing)]
    pub mirror: MirrorConfig,
    /// Observability / telemetry configuration (`[observability]` section).
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
    /// Signal messenger interface configuration (optional).
    /// Populated from the `[signal]` section of `config.toml`.
    pub signal: Option<SignalConfig>,
    /// Slack interface configuration (optional).
    /// Populated from the `[slack]` section of `config.toml`.
    pub slack: Option<SlackConfig>,
    /// Mattermost interface configuration (optional).
    /// Populated from the `[mattermost]` section of `config.toml`.
    pub mattermost: Option<MattermostConfig>,
    /// Nextcloud Talk interface configuration (optional).
    /// Populated from the `[nextcloud]` section of `config.toml`.
    pub nextcloud: Option<NextcloudConfig>,
    /// Matrix interface configuration (optional).
    /// Populated from the `[matrix]` section of `config.toml`.
    pub matrix: Option<MatrixConfig>,
    /// Audio transcription configuration (optional).
    /// Populated from the `[transcription]` section of `config.toml`.
    #[serde(default)]
    pub transcription: Option<TranscriptionConfig>,
    /// Text-to-speech configuration (optional).
    /// Populated from the `[tts]` section of `config.toml`.
    #[serde(default)]
    pub tts: Option<TtsConfig>,
    /// Push notification / VAPID configuration (`[notifications]` section).
    #[serde(default)]
    pub notifications: NotificationsConfig,
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

// ── Channel adapter configuration ─────────────────────────────────────────────
//
// Moved to the `channels` submodule. Import via
// `use assistant_core::types::channels::{SlackConfig, MatrixConfig, ...};`
pub mod channels;

// ── Transcription / TTS configuration ─────────────────────────────────────────
//
// Moved to the `transcription` submodule. Import via
// `use assistant_core::types::transcription::{TranscriptionConfig, TtsConfig, ...};`
pub mod transcription;

// ── LLM / embedding configuration ─────────────────────────────────────────────
//
// Moved to the `llm` submodule. Import via
// `use assistant_core::types::llm::{LlmConfig, AnthropicOptions, ...};`
pub mod llm;

// ── Storage / message-bus configuration ───────────────────────────────────────
//
// Moved to the `storage` submodule. Import via
// `use assistant_core::types::storage::{StorageConfig, BusConfig, BusKind};`
pub mod storage;

// ── Skills / MCP / mirror configuration ───────────────────────────────────────
//
// Moved to the `skills_mcp` submodule. Import via
// `use assistant_core::types::skills_mcp::{SkillsConfig, McpConfig, ...};`
pub mod skills_mcp;

// ── Observability / telemetry configuration ──────────────────────────────────
//
// Moved to the `observability` submodule. Import via
// `use assistant_core::types::observability::{OtelExporter, ObservabilityConfig, ...};`
pub mod observability;

// ── Per-feature runtime configuration ─────────────────────────────────────────
//
// Moved to the `features` submodule. Import via
// `use assistant_core::types::features::{MemoryConfig, CompactionConfig, ...};`
pub mod features;

// ── Tests ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::llm::{EmbeddingConfig, EmbeddingProviderKind, LlmProviderKind};
    use super::skills_mcp::{McpTransportConfig, McpTrustLevel};
    use super::*;

    // -- TitlingConfig defaults / overrides ----------------------------------

    #[test]
    fn test_titling_defaults_when_block_absent() {
        // An OrgConfig with no [titling] section must deserialize cleanly and
        // the worker should pick up the default values.
        let toml_str = "llm_provider = \"ollama\"";
        let cfg: AssistantConfig = toml::from_str(toml_str).unwrap();
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
        let cfg: AssistantConfig = toml::from_str(toml_str).unwrap();
        let titling = cfg.titling;
        assert!(!titling.enabled);
        assert_eq!(titling.min_turns, 4);
        assert_eq!(titling.long_first_message_chars, 500);
    }

    // -- EmbeddingConfig deserialization --------------------------------------

    #[test]
    fn embedding_config_voyage_all_fields() {
        let toml_str = r#"
            provider = "voyage"
            model = "voyage-3-large"
            base_url = "https://custom.voyage.example.com"
            api_key = "pa-test-key"
        "#;
        let cfg: EmbeddingConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.provider, EmbeddingProviderKind::Voyage);
        assert_eq!(cfg.model.as_deref(), Some("voyage-3-large"));
        assert_eq!(
            cfg.base_url.as_deref(),
            Some("https://custom.voyage.example.com")
        );
        assert_eq!(cfg.api_key.as_deref(), Some("pa-test-key"));
    }

    #[test]
    fn embedding_config_ollama_minimal() {
        let toml_str = r#"provider = "ollama""#;
        let cfg: EmbeddingConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.provider, EmbeddingProviderKind::Ollama);
        assert!(cfg.model.is_none());
        assert!(cfg.base_url.is_none());
        assert!(cfg.api_key.is_none());
    }

    #[test]
    fn embedding_config_openai_with_model() {
        let toml_str = r#"
            provider = "openai"
            model = "text-embedding-3-large"
        "#;
        let cfg: EmbeddingConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.provider, EmbeddingProviderKind::OpenAI);
        assert_eq!(cfg.model.as_deref(), Some("text-embedding-3-large"));
    }

    #[test]
    fn embedding_config_invalid_provider_errors() {
        let toml_str = r#"provider = "nonexistent""#;
        let result = toml::from_str::<EmbeddingConfig>(toml_str);
        assert!(
            result.is_err(),
            "Unknown provider should fail deserialization"
        );
    }

    // -- LlmConfig with embeddings section -----------------------------------

    #[test]
    fn llm_config_without_embeddings_defaults_to_none() {
        let toml_str = r#"
            provider = "anthropic"
            model = "claude-opus-4-6"
        "#;
        let cfg: LlmConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.provider, LlmProviderKind::Anthropic);
        assert!(
            cfg.embeddings.is_none(),
            "Embeddings should default to None"
        );
    }

    #[test]
    fn llm_config_with_embeddings_section() {
        let toml_str = r#"
            provider = "anthropic"
            model = "claude-opus-4-6"

            [embeddings]
            provider = "voyage"
            model = "voyage-3-lite"
            api_key = "pa-secret"
        "#;
        let cfg: LlmConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.provider, LlmProviderKind::Anthropic);
        let emb = cfg.embeddings.expect("embeddings should be Some");
        assert_eq!(emb.provider, EmbeddingProviderKind::Voyage);
        assert_eq!(emb.model.as_deref(), Some("voyage-3-lite"));
        assert_eq!(emb.api_key.as_deref(), Some("pa-secret"));
    }

    #[test]
    fn llm_config_with_ollama_embeddings_override() {
        let toml_str = r#"
            provider = "anthropic"
            model = "claude-opus-4-6"

            [embeddings]
            provider = "ollama"
            model = "nomic-embed-text"
            base_url = "http://localhost:11434"
        "#;
        let cfg: LlmConfig = toml::from_str(toml_str).unwrap();
        let emb = cfg.embeddings.expect("embeddings should be Some");
        assert_eq!(emb.provider, EmbeddingProviderKind::Ollama);
        assert_eq!(emb.model.as_deref(), Some("nomic-embed-text"));
        assert_eq!(emb.base_url.as_deref(), Some("http://localhost:11434"));
    }

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
        let cfg: AssistantConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.llm.provider, LlmProviderKind::Anthropic);
        let emb = cfg.llm.embeddings.expect("embeddings should be Some");
        assert_eq!(emb.provider, EmbeddingProviderKind::OpenAI);
        assert_eq!(emb.model.as_deref(), Some("text-embedding-3-small"));
        assert_eq!(emb.api_key.as_deref(), Some("sk-openai-test"));
    }

    #[test]
    fn full_assistant_config_without_embeddings() {
        let toml_str = r#"
            [llm]
            provider = "ollama"
            model = "qwen2.5:7b"
        "#;
        let cfg: AssistantConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.llm.provider, LlmProviderKind::Ollama);
        assert!(cfg.llm.embeddings.is_none());
    }

    // -- Default values ------------------------------------------------------

    #[test]
    fn llm_config_default_has_no_embeddings() {
        let cfg = LlmConfig::default();
        assert!(
            cfg.embeddings.is_none(),
            "Default config should have no embedding override"
        );
    }

    // -- MemoryConfig indexing_interval_seconds --------------------------------

    #[test]
    fn memory_config_default_has_indexing_interval() {
        let cfg = MemoryConfig::default();
        assert_eq!(cfg.indexing_interval_seconds, Some(300));
    }

    #[test]
    fn memory_config_omitted_interval_uses_serde_default() {
        let toml_str = r#"enabled = true"#;
        let cfg: MemoryConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            cfg.indexing_interval_seconds,
            Some(300),
            "omitted field should default to 300 via serde"
        );
    }

    #[test]
    fn memory_config_explicit_interval_is_preserved() {
        let toml_str = r#"
            enabled = true
            indexing_interval_seconds = 60
        "#;
        let cfg: MemoryConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.indexing_interval_seconds, Some(60));
    }

    // -- McpConfig with external servers ---------------------------------------

    #[test]
    fn mcp_config_default_has_empty_servers() {
        let cfg = McpConfig::default();
        assert!(cfg.servers.is_empty());
    }

    #[test]
    fn mcp_config_stdio_server() {
        let toml_str = r#"
            [[servers]]
            name = "github"
            command = ["npx", "-y", "@modelcontextprotocol/server-github"]

            [servers.env]
            GITHUB_TOKEN = "gh-token-123"
        "#;
        let cfg: McpConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.servers.len(), 1);
        let s = &cfg.servers[0];
        assert_eq!(s.name, "github");
        assert!(s.enabled);
        assert_eq!(s.trust, McpTrustLevel::Confirm);
        match &s.transport {
            McpTransportConfig::Stdio { command } => {
                assert_eq!(
                    command,
                    &["npx", "-y", "@modelcontextprotocol/server-github"]
                );
            }
            other => panic!("expected Stdio, got {other:?}"),
        }
        assert_eq!(s.env.get("GITHUB_TOKEN").unwrap(), "gh-token-123");
    }

    #[test]
    fn mcp_config_http_server() {
        let toml_str = r#"
            [[servers]]
            name = "remote-db"
            url = "https://db.example.com/mcp/sse"
            trust = "trust"

            [servers.headers]
            Authorization = "Bearer secret"
        "#;
        let cfg: McpConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.servers.len(), 1);
        let s = &cfg.servers[0];
        assert_eq!(s.name, "remote-db");
        assert_eq!(s.trust, McpTrustLevel::Trust);
        match &s.transport {
            McpTransportConfig::Http { url, headers } => {
                assert_eq!(url, "https://db.example.com/mcp/sse");
                assert_eq!(headers.get("Authorization").unwrap(), "Bearer secret");
            }
            other => panic!("expected Http, got {other:?}"),
        }
    }

    #[test]
    fn mcp_config_multiple_servers() {
        let toml_str = r#"
            [[servers]]
            name = "fs"
            command = ["mcp-server-fs", "/tmp"]

            [[servers]]
            name = "api"
            url = "https://api.example.com/mcp"
            enabled = false
        "#;
        let cfg: McpConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.servers.len(), 2);
        assert!(cfg.servers[0].enabled);
        assert!(!cfg.servers[1].enabled);
    }

    #[test]
    fn mcp_config_no_servers_section_defaults_empty() {
        let toml_str = r#""#;
        let cfg: McpConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.servers.is_empty());
    }

    #[test]
    fn mcp_trust_level_default_is_confirm() {
        let level = McpTrustLevel::default();
        assert_eq!(level, McpTrustLevel::Confirm);
    }

    #[test]
    fn embedding_provider_kind_serializes_lowercase() {
        let json = serde_json::to_string(&EmbeddingProviderKind::Voyage).unwrap();
        assert_eq!(json, "\"voyage\"");
        let json = serde_json::to_string(&EmbeddingProviderKind::Ollama).unwrap();
        assert_eq!(json, "\"ollama\"");
        let json = serde_json::to_string(&EmbeddingProviderKind::OpenAI).unwrap();
        assert_eq!(json, "\"openai\"");
    }
}
