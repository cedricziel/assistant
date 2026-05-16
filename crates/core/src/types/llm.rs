//! LLM and embedding provider configuration, plus provider-specific option
//! sub-structs (Anthropic, OpenAI, Moonshot, OpenRouter).

use serde::{Deserialize, Serialize};

use super::default_true;

// ── Defaults ──────────────────────────────────────────────────────────────────

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

// ── Provider kinds ────────────────────────────────────────────────────────────

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

// ── Embedding config ──────────────────────────────────────────────────────────

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

// ── LLM config ────────────────────────────────────────────────────────────────

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

// ── Anthropic-specific options ────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::{EmbeddingConfig, EmbeddingProviderKind, LlmConfig, LlmProviderKind};

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
