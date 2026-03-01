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

/// Default maximum subagent nesting depth.
pub const DEFAULT_MAX_AGENT_DEPTH: u32 = 5;

/// Runtime context passed to every skill execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub conversation_id: Uuid,
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
}

/// Which interface originated the request
#[derive(Debug, Clone, PartialEq)]
pub enum Interface {
    Cli,
    Signal,
    Mcp,
    Slack,
    Mattermost,
    /// Web UI chat interface.
    Web,
    /// Background scheduled tasks and heartbeats — non-interactive.
    Scheduler,
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

/// Controls which messages the Slack bot reacts to.
///
/// - `Mention` (default) — respond only when `@`-mentioned, in DMs, or in
///   threads the bot is already participating in.
/// - `All` — respond to every message in allowed channels (previous default).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SlackListenMode {
    /// Respond only to `@`-mentions, DMs, and thread replies.
    #[default]
    Mention,
    /// Respond to every message in allowed channels.
    All,
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
    /// Which messages the bot should react to.
    #[serde(default)]
    pub mode: SlackListenMode,
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
    80
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
    /// OpenAI Chat Completions API (API key or OAuth).
    #[serde(alias = "openai-codex")]
    OpenAI,
    /// Moonshot AI (Kimi) — OpenAI-compatible chat completions.
    Moonshot,
}

/// Which embedding backend to use when configured separately from the main
/// LLM provider.
///
/// Set via `[llm.embeddings] provider = "voyage"` in `config.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProviderKind {
    /// Local Ollama server (default model: `nomic-embed-text`).
    Ollama,
    /// OpenAI embeddings API (default model: `text-embedding-3-small`).
    OpenAI,
    /// Voyage AI embeddings (default model: `voyage-3-lite`).
    /// Recommended by Anthropic for use alongside Claude.
    Voyage,
}

/// Optional dedicated embedding provider configuration.
///
/// When present under `[llm.embeddings]`, overrides the main LLM provider's
/// `embed()` method.  Useful when the main provider (e.g. Anthropic) lacks
/// native embedding support, or when a specialised embedding model is desired.
///
/// ```toml
/// [llm.embeddings]
/// provider = "voyage"
/// model = "voyage-3-lite"
/// # api_key = "pa-..."  # or set VOYAGE_API_KEY env var
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Which embedding backend to use.
    pub provider: EmbeddingProviderKind,
    /// Model name (uses provider-specific default if omitted).
    pub model: Option<String>,
    /// Base URL override (uses provider-specific default if omitted).
    pub base_url: Option<String>,
    /// API key (also checked via provider-specific env vars:
    /// `OPENAI_API_KEY`, `VOYAGE_API_KEY`).
    pub api_key: Option<String>,
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
    /// For OpenAI, also checked via `OPENAI_API_KEY` env var.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Provider-specific Anthropic options.
    #[serde(default)]
    pub anthropic: AnthropicOptions,
    /// Provider-specific OpenAI options.
    #[serde(default)]
    pub openai: OpenAIOptions,
    /// Provider-specific Moonshot options.
    #[serde(default)]
    pub moonshot: MoonshotOptions,
    /// Optional dedicated embedding provider override.
    ///
    /// When set, embeddings are served by this provider instead of the main
    /// LLM provider.  Useful with Anthropic (which lacks native embeddings)
    /// or when a specialised embedding model is desired.
    #[serde(default)]
    pub embeddings: Option<EmbeddingConfig>,
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
            openai: OpenAIOptions::default(),
            moonshot: MoonshotOptions::default(),
            embeddings: None,
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

// ── OpenAI-specific options ───────────────────────────────────────────────────

/// Additional configuration for OpenAI-specific features.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAIOptions {
    /// Authentication mode: `"api-key"` (default) or `"oauth"` (Codex subscription).
    #[serde(default)]
    pub auth_mode: OpenAIAuthMode,
    /// OAuth client ID for the PKCE flow.  Required when `auth_mode = "oauth"`.
    pub oauth_client_id: Option<String>,
    /// Maximum completion tokens per response (default: 8192).
    pub max_tokens: Option<u32>,
}

/// How the OpenAI provider authenticates.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum OpenAIAuthMode {
    /// Standard `OPENAI_API_KEY` bearer token (pay-per-use).
    #[default]
    ApiKey,
    /// OAuth 2.0 PKCE via ChatGPT sign-in (Codex subscription).
    OAuth,
}

// ── Moonshot-specific options ─────────────────────────────────────────────────

/// Additional configuration for Moonshot-specific features.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoonshotOptions {
    /// Maximum completion tokens per response (default: 8192).
    pub max_tokens: Option<u32>,
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
    /// When `true`, LLM span events include full message content
    /// (`gen_ai.input.messages`, `gen_ai.output.messages`, etc.).
    /// Off by default because content may contain PII.
    #[serde(default)]
    pub trace_content: bool,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            trace_enabled: true,
            analysis_window: 50,
            trace_content: false,
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
    /// Path to TOOLS.md — environment-specific tool notes (SSH hosts, devices, etc.)
    pub tools_path: Option<String>,
    /// Path to MEMORY.md — curated long-term memory
    pub memory_path: Option<String>,
    /// Directory for daily append-only notes (YYYY-MM-DD.md)
    pub notes_dir: Option<String>,
    /// Path to BOOTSTRAP.md — first-run onboarding ritual (self-deleting)
    pub bootstrap_path: Option<String>,
    /// Path to HEARTBEAT.md — periodic task checklist for the scheduler
    pub heartbeat_path: Option<String>,
    /// Path to BOOT.md — per-session startup hook
    pub boot_path: Option<String>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            agents_path: None,
            soul_path: None,
            identity_path: None,
            user_path: None,
            tools_path: None,
            memory_path: None,
            notes_dir: None,
            bootstrap_path: None,
            heartbeat_path: None,
            boot_path: None,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
