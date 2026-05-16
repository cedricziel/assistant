//! Top-level assistant configuration (`AssistantConfig`) and the active
//! agent context (`AgentConfig`).

use serde::{Deserialize, Serialize};

use super::channels::{MatrixConfig, MattermostConfig, NextcloudConfig, SignalConfig, SlackConfig};
use super::features::{
    CompactionConfig, LearningConfig, MemoryConfig, NotificationsConfig, TitlingConfig,
};
use super::llm::LlmConfig;
use super::observability::ObservabilityConfig;
use super::skills_mcp::{McpConfig, MirrorConfig, SkillsConfig};
use super::storage::{BusConfig, StorageConfig};
use super::transcription::{TranscriptionConfig, TtsConfig};

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
