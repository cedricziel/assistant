use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use self::features::{
    CompactionConfig, LearningConfig, MemoryConfig, NotificationsConfig, TitlingConfig,
};
use self::observability::ObservabilityConfig;
use self::storage::{BusConfig, StorageConfig};
use self::transcription::{TranscriptionConfig, TtsConfig};

#[doc(hidden)]
fn default_true() -> bool {
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

/// Configuration for the Signal messenger interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalConfig {
    /// The phone number registered with Signal (e.g. `"+14155550123"`).
    pub phone_number: Option<String>,

    /// If non-empty, only messages from these sender identifiers are
    /// dispatched to the orchestrator.  An empty list accepts all contacts.
    #[serde(default)]
    pub allowed_senders: Vec<String>,

    /// Base URL of the signal-cli-rest-api daemon.
    /// Defaults to `http://localhost:8080` if not set.
    pub api_url: Option<String>,

    /// HTTP Basic Auth username for signal-cli-rest-api (optional).
    pub api_user: Option<String>,

    /// HTTP Basic Auth password for signal-cli-rest-api (optional).
    pub api_password: Option<String>,
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

/// Configuration for the Matrix interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatrixConfig {
    /// Base URL of the Matrix homeserver (e.g. `"https://matrix.example.com"`).
    pub homeserver_url: Option<String>,
    /// Full Matrix user ID of the bot account (e.g. `"@assistant:example.com"`).
    pub username: Option<String>,
    /// Bot account password (used for initial login; session persisted to disk).
    /// Prefer `access_token` for production deployments.
    pub password: Option<String>,
    /// Pre-issued Matrix access token (skips password login on every start).
    pub access_token: Option<String>,
    /// Device ID for session restoration. Auto-generated on first login if omitted.
    pub device_id: Option<String>,
    /// Path for the matrix-sdk SQLite state store.
    /// Defaults to `~/.assistant/matrix-state/` at runtime.
    pub state_store_path: Option<String>,
    /// If non-empty, only dispatch messages from these Matrix room IDs.
    /// An empty list accepts all rooms.
    #[serde(default)]
    pub allowed_rooms: Vec<String>,
    /// If non-empty, only dispatch messages from these Matrix user IDs.
    /// An empty list accepts all users.
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

/// Configuration for the Nextcloud Talk interface.
///
/// The bot is installed on the Nextcloud server via
/// `occ talk:bot:install` which sets up the webhook URL and a shared secret.
/// The assistant runs an HTTP server that receives incoming webhook callbacks
/// and replies via the Nextcloud Talk Bot REST API.
///
/// ```toml
/// [nextcloud]
/// server_url = "https://nextcloud.example.com"
/// secret = "shared-secret-from-occ-install"
/// listen_addr = "0.0.0.0:8080"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextcloudConfig {
    /// Base URL of the Nextcloud server (e.g. `"https://nextcloud.example.com"`).
    pub server_url: Option<String>,
    /// Shared secret configured when registering the bot via `occ talk:bot:install`.
    /// Also checked via `NEXTCLOUD_TALK_SECRET` env var.
    pub secret: Option<String>,
    /// Socket address the webhook HTTP server listens on (default: `"0.0.0.0:8080"`).
    #[serde(default = "default_nextcloud_listen_addr")]
    pub listen_addr: String,
    /// If non-empty, only dispatch messages from these conversation tokens.
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    /// If non-empty, only dispatch messages from these Nextcloud user IDs.
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

impl Default for NextcloudConfig {
    fn default() -> Self {
        Self {
            server_url: None,
            secret: None,
            listen_addr: default_nextcloud_listen_addr(),
            allowed_channels: Vec::new(),
            allowed_users: Vec::new(),
        }
    }
}

fn default_nextcloud_listen_addr() -> String {
    "0.0.0.0:8080".to_string()
}

// ── Transcription / TTS configuration ─────────────────────────────────────────
//
// Moved to the `transcription` submodule. Import via
// `use assistant_core::types::transcription::{TranscriptionConfig, TtsConfig, ...};`
pub mod transcription;

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
    /// OpenRouter — unified API gateway for 300+ models.
    OpenRouter,
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
    /// Provider-specific OpenRouter options.
    #[serde(default)]
    pub openrouter: OpenRouterOptions,
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
            openrouter: OpenRouterOptions::default(),
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
    /// Hosted web-search configuration.
    ///
    /// Requires a search-capable model (`gpt-4o-search-preview`,
    /// `gpt-4o-mini-search-preview`, or `gpt-5-search-api`).
    #[serde(default)]
    pub web_search: OpenAIWebSearchOptions,
}

/// Configuration for OpenAI hosted web search via Chat Completions.
///
/// When enabled, the provider sets the `web_search_options` top-level parameter
/// on every Chat Completions request.  The model always searches the web before
/// responding (there is no per-turn opt-in with Chat Completions).
///
/// **Requires** one of: `gpt-4o-search-preview`, `gpt-4o-mini-search-preview`,
/// `gpt-5-search-api`.  Other models will ignore the parameter.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAIWebSearchOptions {
    #[serde(default)]
    pub enabled: bool,
    /// Search context size: `"low"`, `"medium"` (default), or `"high"`.
    pub search_context_size: Option<String>,
    /// Approximate user location for geographically relevant results.
    pub user_location: Option<OpenAIUserLocation>,
}

/// Approximate user location for OpenAI web search.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAIUserLocation {
    /// Two-letter ISO country code (e.g. `"US"`, `"GB"`).
    pub country: Option<String>,
    /// City name (e.g. `"London"`).
    pub city: Option<String>,
    /// Region/state name (e.g. `"California"`).
    pub region: Option<String>,
    /// IANA timezone (e.g. `"America/Chicago"`).
    pub timezone: Option<String>,
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
    /// Hosted web-search configuration (`$web_search` builtin).
    #[serde(default)]
    pub web_search: MoonshotWebSearchOptions,
}

/// Configuration for Moonshot's `$web_search` builtin function.
///
/// When enabled (the default), the provider injects a `builtin_function` tool
/// spec into every request and handles the server-side search echo-back loop
/// internally.  The orchestrator never sees `$web_search` tool calls.
///
/// The model decides per-turn whether to search — there is no extra cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoonshotWebSearchOptions {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for MoonshotWebSearchOptions {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ── OpenRouter-specific options ─────────────────────────────────────────────

/// Additional configuration for OpenRouter-specific features.
///
/// ```toml
/// [llm]
/// provider = "openrouter"
/// model = "anthropic/claude-sonnet-4-20250514"
/// # api_key = "sk-or-..."  # or set OPENROUTER_API_KEY env var
///
/// [llm.openrouter]
/// referer = "https://my-app.example.com"
/// title = "My App"
/// max_tokens = 8192
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenRouterOptions {
    /// `HTTP-Referer` header sent with every request.
    /// Required by OpenRouter TOS for rankings/attribution.
    pub referer: Option<String>,
    /// `X-Title` header — shown in the OpenRouter dashboard.
    pub title: Option<String>,
    /// Maximum completion tokens per response (default: 8192).
    pub max_tokens: Option<u32>,
}

// ── Storage / message-bus configuration ───────────────────────────────────────
//
// Moved to the `storage` submodule. Import via
// `use assistant_core::types::storage::{StorageConfig, BusConfig, BusKind};`
pub mod storage;

/// Skills configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// Extra directories to scan for Agent Skills.
    /// Defaults cover Claude Code / NanoClaw shared skill folders.
    #[serde(default = "default_skill_extra_dirs")]
    pub extra_dirs: Vec<String>,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            extra_dirs: default_skill_extra_dirs(),
        }
    }
}

fn default_skill_extra_dirs() -> Vec<String> {
    vec![
        "~/.claude/skills".to_string(),
        "./.claude/skills".to_string(),
    ]
}

/// MCP configuration — covers external MCP client connections.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// External MCP servers to connect to as a client.
    /// Each entry spawns a connection at startup and bridges the server's tools
    /// into the assistant's tool registry.
    #[serde(default)]
    pub servers: Vec<McpServerEntry>,
}

/// An external MCP server the assistant connects to as a client.
///
/// Tools discovered from the server are registered with the prefix
/// `mcp__{name}__{tool_name}` to avoid collisions with builtin tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    /// Unique name for this server (used in tool prefix).
    /// Must be a valid identifier: lowercase alphanumeric + hyphens.
    pub name: String,
    /// Transport configuration — determines how we connect to the server.
    #[serde(flatten)]
    pub transport: McpTransportConfig,
    /// Extra environment variables passed to stdio-spawned servers.
    /// Values may reference environment variables with `${VAR}` syntax.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Whether this server is enabled (default: true).
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Trust level controlling safety-gate behaviour for this server's tools.
    #[serde(default)]
    pub trust: McpTrustLevel,
}

/// How to connect to an external MCP server.
///
/// Uses `#[serde(untagged)]` for compatibility with the standard MCP
/// configuration format (Claude Desktop, VS Code, etc.).  Serde tries
/// variants in order: if `command` is present the entry is treated as
/// `Stdio`; otherwise `url` selects `Http`.  If a config entry contains
/// *both* `command` and `url`, `Stdio` wins and `url` is silently
/// ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpTransportConfig {
    /// Spawn a local subprocess; communicate via JSON-RPC over stdin/stdout.
    Stdio {
        /// Command and its arguments (e.g. `["npx", "-y", "@mcp/server-github"]`).
        command: Vec<String>,
    },
    /// Connect to a remote HTTP endpoint (SSE or Streamable HTTP).
    Http {
        /// Server URL (e.g. `"https://example.com/mcp/sse"`).
        url: String,
        /// Extra HTTP headers sent with every request.
        /// Values may reference environment variables with `${VAR}` syntax.
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

/// Trust level for tools from an external MCP server.
///
/// Controls whether the safety gate requires user confirmation before executing
/// a tool call from this server.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum McpTrustLevel {
    /// Every tool call requires explicit user confirmation (default).
    #[default]
    Confirm,
    /// Tools may run without confirmation (use for trusted, read-only servers).
    Trust,
}

/// Self-improvement / tracing config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    pub trace_enabled: bool,
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
            trace_content: false,
        }
    }
}

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
