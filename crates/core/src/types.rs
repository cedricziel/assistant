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
    /// For tool messages: which skill produced this result.
    pub skill_name: Option<String>,
    /// For assistant messages that contain tool calls: the serialised
    /// `Vec<ToolCallItem>` JSON.  Populated when the LLM response was a
    /// `ToolCalls` variant; `None` for plain text messages.
    pub tool_calls_json: Option<String>,
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
            tool_calls_json: None,
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
    #[serde(default)]
    pub memory: MemoryConfig,
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
fn default_llm_provider() -> LlmProviderKind {
    LlmProviderKind::Ollama
}
fn default_embedding_model() -> String {
    "nomic-embed-text".to_string()
}

/// Which LLM backend to use.
///
/// Set via `[llm] provider = "ollama"` in `config.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LlmProviderKind {
    #[default]
    Ollama,
    Anthropic,
}

/// LLM / provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Which backend to use (default: `ollama`).
    #[serde(default = "default_llm_provider")]
    pub provider: LlmProviderKind,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_llm_base_url")]
    pub base_url: String,
    #[serde(default = "default_llm_max_iterations")]
    pub max_iterations: usize,
    #[serde(default = "default_llm_timeout_secs")]
    pub timeout_secs: u64,
    /// Embedding model for vector search (default: `nomic-embed-text`).
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// API key for cloud providers (Anthropic, OpenAI, …).
    /// For Anthropic, also checked via `ANTHROPIC_API_KEY` env var.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Provider-specific Anthropic options.
    #[serde(default)]
    pub anthropic: AnthropicOptions,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            model: default_llm_model(),
            base_url: default_llm_base_url(),
            max_iterations: default_llm_max_iterations(),
            timeout_secs: default_llm_timeout_secs(),
            embedding_model: default_embedding_model(),
            api_key: None,
            anthropic: AnthropicOptions::default(),
        }
    }
}

/// Additional configuration for Anthropic-specific features.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnthropicOptions {
    #[serde(default)]
    pub web_search: AnthropicWebSearchOptions,
    #[serde(default)]
    pub web_fetch: AnthropicWebFetchOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicWebSearchOptions {
    #[serde(default)]
    pub enabled: bool,
    pub max_uses: Option<u32>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub blocked_domains: Vec<String>,
    pub user_location: Option<AnthropicUserLocation>,
}

impl Default for AnthropicWebSearchOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            max_uses: None,
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
            user_location: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnthropicUserLocation {
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicWebFetchOptions {
    #[serde(default)]
    pub enabled: bool,
    pub max_uses: Option<u32>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub blocked_domains: Vec<String>,
    #[serde(default)]
    pub citations: AnthropicCitationsOptions,
    pub max_content_tokens: Option<u32>,
}

impl Default for AnthropicWebFetchOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            max_uses: None,
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
            citations: AnthropicCitationsOptions::default(),
            max_content_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnthropicCitationsOptions {
    #[serde(default)]
    pub enabled: bool,
}

/// Storage configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageConfig {
    pub db_path: Option<String>,
}

/// Skills configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// Extra directories to scan for Agent Skills.
    /// Defaults cover Claude Code / NanoClaw shared skill folders.
    #[serde(default = "default_skill_extra_dirs")]
    pub extra_dirs: Vec<String>,
    /// Skills to disable globally.
    #[serde(default)]
    pub disabled: Vec<String>,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            extra_dirs: default_skill_extra_dirs(),
            disabled: Vec::new(),
        }
    }
}

fn default_skill_extra_dirs() -> Vec<String> {
    vec![
        "~/.claude/skills".to_string(),
        "./.claude/skills".to_string(),
    ]
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

fn default_true() -> bool {
    true
}

/// Configuration for the agent's persistent markdown memory files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Whether memory loading is enabled (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Path to AGENTS.md — workspace rules, session startup ritual, memory discipline
    pub agents_path: Option<String>,
    /// Path to SOUL.md — personality, values, core truths
    pub soul_path: Option<String>,
    /// Path to IDENTITY.md — name, role, structured identity profile
    pub identity_path: Option<String>,
    /// Path to USER.md — user profile, preferences, timezone
    pub user_path: Option<String>,
    /// Path to MEMORY.md — curated long-term memory
    pub memory_path: Option<String>,
    /// Directory for daily append-only notes (YYYY-MM-DD.md)
    pub notes_dir: Option<String>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            agents_path: None,
            soul_path: None,
            identity_path: None,
            user_path: None,
            memory_path: None,
            notes_dir: None,
        }
    }
}
