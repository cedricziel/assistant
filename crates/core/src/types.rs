use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    /// For tool messages: which skill produced this
    pub skill_name: Option<String>,
    pub turn: i64,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub fn new(conversation_id: Uuid, role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role,
            content: content.into(),
            skill_name: None,
            turn: 0,
            created_at: Utc::now(),
        }
    }

    pub fn user(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self::new(conversation_id, MessageRole::User, content)
    }

    pub fn assistant(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self::new(conversation_id, MessageRole::Assistant, content)
    }
}

/// An execution trace record (written to SQLite for self-improvement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub turn: i64,
    pub action_skill: String,
    pub action_params: serde_json::Value,
    pub observation: Option<String>,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
}

impl ExecutionTrace {
    pub fn new(
        conversation_id: Uuid,
        turn: i64,
        action_skill: impl Into<String>,
        action_params: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            turn,
            action_skill: action_skill.into(),
            action_params,
            observation: None,
            error: None,
            duration_ms: 0,
            created_at: Utc::now(),
        }
    }

    pub fn with_success(mut self, observation: impl Into<String>, duration_ms: i64) -> Self {
        self.observation = Some(observation.into());
        self.duration_ms = duration_ms;
        self
    }

    pub fn with_error(mut self, error: impl Into<String>, duration_ms: i64) -> Self {
        self.error = Some(error.into());
        self.duration_ms = duration_ms;
        self
    }
}

/// Runtime context passed to every skill execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub conversation_id: Uuid,
    pub turn: i64,
    /// The interface this turn originated from (cli, signal, mcp)
    pub interface: Interface,
    /// Whether the skill can prompt the user for confirmation
    pub interactive: bool,
}

/// Which interface originated the request
#[derive(Debug, Clone, PartialEq)]
pub enum Interface {
    Cli,
    Signal,
    Mcp,
    Slack,
    Mattermost,
}

/// Top-level assistant configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssistantConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub mirror: MirrorConfig,
    /// Signal messenger interface configuration (optional).
    /// Populated from the `[signal]` section of `config.toml`.
    pub signal: Option<SignalConfig>,
    /// Slack interface configuration (optional).
    /// Populated from the `[slack]` section of `config.toml`.
    pub slack: Option<SlackConfig>,
    /// Mattermost interface configuration (optional).
    /// Populated from the `[mattermost]` section of `config.toml`.
    pub mattermost: Option<MattermostConfig>,
}

/// Configuration for the Signal messenger interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalConfig {
    /// The phone number registered with Signal (e.g. `"+14155550123"`).
    pub phone_number: Option<String>,

    /// If non-empty, only messages from these sender identifiers are
    /// dispatched to the orchestrator.  An empty list accepts all contacts.
    #[serde(default)]
    pub allowed_senders: Vec<String>,

    /// Path where presage stores its Signal state.  Defaults to
    /// `~/.assistant/signal-store` (resolved at runtime by the interface
    /// crate, which has access to the `dirs` crate).
    pub store_path: Option<String>,
}

/// Configuration for the Slack interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlackConfig {
    /// Bot OAuth token (`xoxb-…`) for sending messages via the Web API.
    pub bot_token: Option<String>,
    /// App-level token (`xapp-…`) for Socket Mode connections.
    pub app_token: Option<String>,
    /// If non-empty, only dispatch messages from these channel IDs.
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    /// If non-empty, only dispatch messages from these Slack user IDs.
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

/// Configuration for the Mattermost interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MattermostConfig {
    /// Base URL of the Mattermost server (e.g. `"https://mattermost.example.com"`).
    pub server_url: Option<String>,
    /// Personal access token or bot token for authentication.
    pub token: Option<String>,
    /// If non-empty, only dispatch messages from these channel IDs.
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    /// If non-empty, only dispatch messages from these Mattermost user IDs.
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

fn default_llm_model() -> String {
    "qwen2.5:7b".to_string()
}
fn default_llm_base_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_llm_max_iterations() -> usize {
    10
}
fn default_llm_timeout_secs() -> u64 {
    120
}

/// LLM / Ollama configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_llm_base_url")]
    pub base_url: String,
    #[serde(default = "default_llm_max_iterations")]
    pub max_iterations: usize,
    #[serde(default = "default_llm_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: default_llm_model(),
            base_url: default_llm_base_url(),
            max_iterations: default_llm_max_iterations(),
            timeout_secs: default_llm_timeout_secs(),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageConfig {
    pub db_path: Option<String>,
}

/// Skills configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub extra_dirs: Vec<String>,
    pub disabled: Vec<String>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub enabled: bool,
    pub listen: String,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            listen: "127.0.0.1:3000".to_string(),
        }
    }
}

/// Self-improvement config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    pub trace_enabled: bool,
    pub analysis_window: usize,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            trace_enabled: true,
            analysis_window: 50,
        }
    }
}
