use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

/// Role of a message in the conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

/// A single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    /// For tool messages: which skill produced this result.
    pub skill_name: Option<String>,
    /// For assistant messages that contain tool calls: the serialised
    /// `Vec<ToolCallItem>` JSON.  Populated when the LLM response was a
    /// `ToolCalls` variant; `None` for plain text messages.
    pub tool_calls_json: Option<String>,
    pub turn: i64,
    pub created_at: DateTime<Utc>,
    /// The user who sent this message (multi-user scoping).
    /// `None` for assistant/system/tool messages and legacy data.
    pub sender_user_id: Option<String>,
}

impl Message {
    pub fn new(conversation_id: Uuid, role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role,
            content: content.into(),
            skill_name: None,
            tool_calls_json: None,
            turn: 0,
            created_at: Utc::now(),
            sender_user_id: None,
        }
    }

    pub fn user(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self::new(conversation_id, MessageRole::User, content)
    }

    pub fn assistant(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self::new(conversation_id, MessageRole::Assistant, content)
    }
}

/// Default maximum subagent nesting depth.
pub const DEFAULT_MAX_AGENT_DEPTH: u32 = 5;

/// Runtime context passed to every skill execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub conversation_id: Uuid,
    /// Active assistant agent context ID.
    pub agent_id: String,
    pub turn: i64,
    /// The interface this turn originated from (cli, signal, mcp)
    pub interface: Interface,
    /// Whether the skill can prompt the user for confirmation
    pub interactive: bool,
    /// When `Some`, only tools whose names appear in this list may be executed.
    /// `None` means all registered tools are available (the default).
    pub allowed_tools: Option<Vec<String>>,
    /// Current subagent nesting depth.  The root agent has depth `0`.
    pub depth: u32,

    // -- Identity fields (multi-user) ----------------------------------------
    /// The authenticated user executing this turn. `None` for legacy
    /// single-user or system-originated turns.
    pub user_id: Option<crate::identity::UserId>,
    /// The organization context. `None` for legacy single-user mode.
    pub org_id: Option<crate::identity::OrgId>,
    /// The space context. `None` for legacy single-user mode.
    pub space_id: Option<crate::identity::SpaceId>,
}

impl ExecutionContext {
    /// Populate the identity fields from an [`AuthContext`](crate::auth::AuthContext).
    ///
    /// Copies `user_id` and `org_id` directly; leaves `space_id` as-is since
    /// it depends on the routing context rather than the token.
    pub fn with_auth(mut self, auth: &crate::auth::AuthContext) -> Self {
        self.user_id = Some(auth.user_id.clone());
        self.org_id = Some(auth.org_id.clone());
        self
    }

    /// Populate the identity fields from a [`TurnIdentity`].
    pub fn with_identity(mut self, id: &TurnIdentity) -> Self {
        self.user_id = id.user_id.clone();
        self.org_id = id.org_id.clone();
        self.space_id = id.space_id.clone();
        self
    }
}

/// Lightweight bundle of optional identity fields threaded through the
/// orchestrator turn pipeline.
///
/// Callers that have an authenticated user populate this from
/// [`AuthContext`](crate::auth::AuthContext); legacy / system turns use
/// [`Default`].
#[derive(Debug, Clone, Default)]
pub struct TurnIdentity {
    pub user_id: Option<crate::identity::UserId>,
    pub org_id: Option<crate::identity::OrgId>,
    pub space_id: Option<crate::identity::SpaceId>,
}

impl TurnIdentity {
    /// Build a `TurnIdentity` from an [`AuthContext`](crate::auth::AuthContext).
    pub fn from_auth(auth: &crate::auth::AuthContext) -> Self {
        Self {
            user_id: Some(auth.user_id.clone()),
            org_id: Some(auth.org_id.clone()),
            space_id: None,
        }
    }
}

/// Which interface originated the request
#[derive(Debug, Clone, PartialEq)]
pub enum Interface {
    Cli,
    Signal,
    Mcp,
    Slack,
    Mattermost,
    /// Nextcloud Talk webhook-based bot interface.
    Nextcloud,
    /// Web UI chat interface.
    Web,
    /// Background scheduled tasks and heartbeats — non-interactive.
    Scheduler,
    /// Matrix messaging interface.
    Matrix,
}

/// Identifies the messaging platform for a `ChannelAdapter`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    Slack,
    Mattermost,
    Matrix,
    Nextcloud,
    Signal,
    /// Catch-all for future or custom platforms.
    Custom(String),
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Slack => write!(f, "slack"),
            ChannelType::Mattermost => write!(f, "mattermost"),
            ChannelType::Matrix => write!(f, "matrix"),
            ChannelType::Nextcloud => write!(f, "nextcloud"),
            ChannelType::Signal => write!(f, "signal"),
            ChannelType::Custom(name) => write!(f, "{name}"),
        }
    }
}

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
