//! LLM and embedding provider configuration, plus provider-specific option
//! sub-structs (Anthropic, OpenAI, Moonshot, OpenRouter).

use serde::{Deserialize, Serialize};

use super::default_true;
use crate::secret::Secret;

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
/// Set via `[llm] provider = "ollama"` in `org.toml`.
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
/// Set via `[llm.embeddings] provider = "voyage"` in `org.toml`.
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
    pub api_key: Option<Secret>,
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
    /// Whether the model may request several tool calls at once. Unset keeps
    /// the provider's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Embedding model for vector search (default: `nomic-embed-text`).
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// API key for cloud providers (Anthropic, OpenAI, …).
    /// For Anthropic, also checked via `ANTHROPIC_API_KEY` env var.
    /// For OpenAI, also checked via `OPENAI_API_KEY` env var.
    #[serde(default)]
    pub api_key: Option<Secret>,
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
            parallel_tool_calls: None,
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

// ── OpenRouter key resolution ─────────────────────────────────────────────────

/// Environment variable checked last for an OpenRouter API key.
pub const OPENROUTER_API_KEY_ENV: &str = "OPENROUTER_API_KEY";

impl LlmConfig {
    /// The `[llm]` API key, but only when `[llm]` itself is OpenRouter, so a key
    /// meant for another provider is never sent to OpenRouter.
    pub fn openrouter_api_key(&self) -> Option<&Secret> {
        match self.provider {
            LlmProviderKind::OpenRouter => self.api_key.as_ref(),
            _ => None,
        }
    }
}

/// Resolve the key for an OpenRouter-backed feature: the feature's own
/// `section` key, then the `[llm]` OpenRouter key, then `env_var` read through
/// `env`. Blank values count as unset.
pub fn resolve_openrouter_key(
    section: Option<&Secret>,
    llm: &LlmConfig,
    env_var: &str,
    env: impl Fn(&str) -> Option<String>,
) -> Option<Secret> {
    section
        .filter(|s| !s.is_blank())
        .or_else(|| llm.openrouter_api_key().filter(|s| !s.is_blank()))
        .cloned()
        .or_else(|| env(env_var).map(Secret::from).filter(|s| !s.is_blank()))
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
/// categories = ["cli-agent"]
/// max_tokens = 8192
/// fallback_models = ["openai/gpt-4o"]
///
/// [llm.openrouter.provider]
/// data_collection = "deny"
/// zdr = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenRouterOptions {
    /// `HTTP-Referer` header, used by OpenRouter for app attribution.
    pub referer: Option<String>,
    /// `X-OpenRouter-Title` header, shown in the OpenRouter dashboard.
    pub title: Option<String>,
    /// `X-OpenRouter-Categories` header; at most two.
    #[serde(default, deserialize_with = "at_most_two_categories")]
    pub categories: Vec<String>,
    /// Maximum completion tokens per response (default: 8192).
    pub max_tokens: Option<u32>,
    /// Models to try, in order, when the primary model fails. Sent as
    /// `models` after the primary model.
    #[serde(default)]
    pub fallback_models: Vec<String>,
    /// Provider routing, sent as the request's `provider` object.
    pub provider: Option<OpenRouterRouting>,
    /// Reasoning controls, sent as the request's `reasoning` object.
    pub reasoning: Option<OpenRouterReasoning>,
    /// Prompt-cache breakpoints (`[llm.openrouter.cache]`).
    #[serde(default)]
    pub cache: OpenRouterCache,
    /// OpenRouter's `web` plugin (`[llm.openrouter.web_search]`).
    #[serde(default)]
    pub web_search: OpenRouterWebSearch,
    /// Parser for PDF attachments. Unset: models with native file input read
    /// PDFs themselves, others use `cloudflare-ai`.
    pub pdf_engine: Option<OpenRouterPdfEngine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpenRouterPdfEngine {
    Native,
    CloudflareAi,
    MistralOcr,
}

/// `[llm.openrouter.web_search]`. `search_context_size` needs the native
/// engine, or no engine so OpenRouter may pick native search.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawOpenRouterWebSearch")]
pub struct OpenRouterWebSearch {
    pub enabled: bool,
    pub engine: Option<OpenRouterSearchEngine>,
    pub max_results: Option<u32>,
    pub search_prompt: Option<String>,
    pub include_domains: Option<Vec<String>>,
    pub exclude_domains: Option<Vec<String>>,
    pub search_context_size: Option<OpenRouterSearchContextSize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOpenRouterWebSearch {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    engine: Option<OpenRouterSearchEngine>,
    #[serde(default)]
    max_results: Option<u32>,
    #[serde(default)]
    search_prompt: Option<String>,
    #[serde(default)]
    include_domains: Option<Vec<String>>,
    #[serde(default)]
    exclude_domains: Option<Vec<String>>,
    #[serde(default)]
    search_context_size: Option<OpenRouterSearchContextSize>,
}

impl TryFrom<RawOpenRouterWebSearch> for OpenRouterWebSearch {
    type Error = String;

    fn try_from(raw: RawOpenRouterWebSearch) -> Result<Self, Self::Error> {
        if raw.search_context_size.is_some()
            && raw
                .engine
                .is_some_and(|e| e != OpenRouterSearchEngine::Native)
        {
            return Err("`search_context_size` works only with `engine = \"native\"`".to_owned());
        }
        Ok(Self {
            enabled: raw.enabled,
            engine: raw.engine,
            max_results: raw.max_results,
            search_prompt: raw.search_prompt,
            include_domains: raw.include_domains,
            exclude_domains: raw.exclude_domains,
            search_context_size: raw.search_context_size,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenRouterSearchEngine {
    Native,
    Exa,
    Firecrawl,
    Parallel,
    Perplexity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenRouterSearchContextSize {
    Low,
    Medium,
    High,
}

/// `[llm.openrouter.cache]`: where models that need explicit breakpoints
/// get `cache_control`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRouterCache {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub ttl: OpenRouterCacheTtl,
}

impl Default for OpenRouterCache {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: OpenRouterCacheTtl::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenRouterCacheTtl {
    #[default]
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "1h")]
    OneHour,
}

/// `[llm.openrouter.reasoning]`. `effort` and `max_tokens` are exclusive.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawOpenRouterReasoning")]
pub struct OpenRouterReasoning {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<OpenRouterEffort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Reason without returning the reasoning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOpenRouterReasoning {
    #[serde(default)]
    effort: Option<OpenRouterEffort>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    exclude: Option<bool>,
    #[serde(default)]
    enabled: Option<bool>,
}

impl TryFrom<RawOpenRouterReasoning> for OpenRouterReasoning {
    type Error = String;

    fn try_from(raw: RawOpenRouterReasoning) -> Result<Self, Self::Error> {
        if raw.effort.is_some() && raw.max_tokens.is_some() {
            return Err("`effort` and `max_tokens` cannot both be set".to_owned());
        }
        Ok(Self {
            effort: raw.effort,
            max_tokens: raw.max_tokens,
            exclude: raw.exclude,
            enabled: raw.enabled,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenRouterEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl OpenRouterEffort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }
}

fn at_most_two_categories<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let categories = Vec::<String>::deserialize(deserializer)?;
    if categories.len() > 2 {
        return Err(serde::de::Error::custom(format!(
            "`categories` takes at most two values, got {}",
            categories.len()
        )));
    }
    Ok(categories)
}

/// OpenRouter provider routing preferences (`[llm.openrouter.provider]`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRouterRouting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_collection: Option<OpenRouterDataCollection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zdr: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<OpenRouterSort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_price: Option<OpenRouterMaxPrice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenRouterDataCollection {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenRouterSort {
    Price,
    Throughput,
    Latency,
}

/// Highest acceptable prices, in US dollars per million tokens (or per
/// request or image).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRouterMaxPrice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::{
        EmbeddingConfig, EmbeddingProviderKind, LlmConfig, LlmProviderKind, OPENROUTER_API_KEY_ENV,
        resolve_openrouter_key,
    };
    use crate::secret::Secret;

    // -- OpenRouter options ----------------------------------------------------

    mod openrouter_options {
        use super::super::{
            OpenRouterDataCollection, OpenRouterOptions, OpenRouterRouting, OpenRouterSort,
        };

        fn parse(toml: &str) -> Result<OpenRouterOptions, toml::de::Error> {
            toml::from_str(toml)
        }

        #[test]
        fn provider_routing_parses() {
            let options = parse(
                r#"
                fallback_models = ["openai/gpt-4o"]
                categories = ["cli-agent", "chat"]
                [provider]
                order = ["anthropic"]
                data_collection = "deny"
                zdr = true
                sort = "price"
                max_price = { prompt = 1.0 }
                "#,
            )
            .unwrap();
            assert_eq!(options.fallback_models, ["openai/gpt-4o"]);
            assert_eq!(options.categories, ["cli-agent", "chat"]);
            let routing = options.provider.unwrap();
            assert_eq!(
                routing.data_collection,
                Some(OpenRouterDataCollection::Deny)
            );
            assert_eq!(routing.zdr, Some(true));
            assert_eq!(routing.sort, Some(OpenRouterSort::Price));
            assert_eq!(routing.max_price.unwrap().prompt, Some(1.0));
        }

        #[test]
        fn unknown_provider_key_is_rejected() {
            let err = parse("[provider]\nprefer = [\"x\"]").unwrap_err();
            assert!(err.to_string().contains("prefer"), "{err}");
        }

        #[test]
        fn more_than_two_categories_are_rejected() {
            let err = parse(r#"categories = ["a", "b", "c"]"#).unwrap_err();
            assert!(err.to_string().contains("at most two"), "{err}");
        }

        #[test]
        fn reasoning_parses_and_serializes_only_set_fields() {
            use super::super::OpenRouterEffort;

            let options = parse("[reasoning]\neffort = \"high\"\nexclude = true").unwrap();
            let reasoning = options.reasoning.unwrap();
            assert_eq!(reasoning.effort, Some(OpenRouterEffort::High));
            assert_eq!(
                serde_json::to_value(reasoning).unwrap(),
                serde_json::json!({"effort": "high", "exclude": true})
            );
        }

        #[test]
        fn cache_defaults_on_with_five_minutes() {
            use super::super::{OpenRouterCache, OpenRouterCacheTtl};

            assert_eq!(parse("").unwrap().cache, OpenRouterCache::default());
            let cache = parse("[cache]\nttl = \"1h\"").unwrap().cache;
            assert!(cache.enabled);
            assert_eq!(cache.ttl, OpenRouterCacheTtl::OneHour);
            assert!(parse("[cache]\nttl = \"2h\"").is_err());
            assert!(!parse("[cache]\nenabled = false").unwrap().cache.enabled);
        }

        #[test]
        fn web_search_parses_and_defaults_off() {
            use super::super::OpenRouterSearchEngine;

            assert!(!parse("").unwrap().web_search.enabled);
            let search = parse(
                "[web_search]\nenabled = true\nengine = \"exa\"\nmax_results = 3\nexclude_domains = [\"reddit.com\"]",
            )
            .unwrap()
            .web_search;
            assert!(search.enabled);
            assert_eq!(search.engine, Some(OpenRouterSearchEngine::Exa));
            assert_eq!(search.max_results, Some(3));
            assert!(parse("[web_search]\nsearch_context_size = \"high\"").is_ok());
            assert!(
                parse("[web_search]\nengine = \"native\"\nsearch_context_size = \"low\"").is_ok()
            );
        }

        #[test]
        fn search_context_size_with_a_non_native_engine_is_rejected() {
            let err = parse("[web_search]\nengine = \"exa\"\nsearch_context_size = \"high\"")
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("search_context_size") && err.contains("native"),
                "{err}"
            );
            assert!(
                parse("[web_search]\nengin = \"exa\"").is_err(),
                "unknown keys are rejected"
            );
        }

        #[test]
        fn pdf_engine_parses_kebab_case() {
            use super::super::OpenRouterPdfEngine;

            assert_eq!(parse("").unwrap().pdf_engine, None);
            assert_eq!(
                parse("pdf_engine = \"mistral-ocr\"").unwrap().pdf_engine,
                Some(OpenRouterPdfEngine::MistralOcr)
            );
            assert!(parse("pdf_engine = \"tesseract\"").is_err());
        }

        #[test]
        fn effort_with_max_tokens_is_rejected() {
            let err = parse("[reasoning]\neffort = \"low\"\nmax_tokens = 2000").unwrap_err();
            let err = err.to_string();
            assert!(
                err.contains("effort") && err.contains("max_tokens"),
                "{err}"
            );
        }

        #[test]
        fn unknown_reasoning_key_is_rejected() {
            let err = parse("[reasoning]\nbudget = 1").unwrap_err();
            assert!(err.to_string().contains("budget"), "{err}");
        }

        #[test]
        fn routing_serializes_only_set_fields() {
            let routing = OpenRouterRouting {
                data_collection: Some(OpenRouterDataCollection::Deny),
                zdr: Some(true),
                ..OpenRouterRouting::default()
            };
            assert_eq!(
                serde_json::to_value(routing).unwrap(),
                serde_json::json!({"data_collection": "deny", "zdr": true})
            );
        }
    }

    // -- OpenRouter key resolution --------------------------------------------

    fn llm(provider: LlmProviderKind, key: Option<&str>) -> LlmConfig {
        LlmConfig {
            provider,
            api_key: key.map(Secret::new),
            ..LlmConfig::default()
        }
    }

    fn env_with(value: Option<&'static str>) -> impl Fn(&str) -> Option<String> {
        move |name| {
            assert_eq!(name, OPENROUTER_API_KEY_ENV, "unexpected env var lookup");
            value.map(str::to_string)
        }
    }

    fn resolved(
        section: Option<&str>,
        llm: &LlmConfig,
        env: impl Fn(&str) -> Option<String>,
    ) -> Option<String> {
        let section = section.map(Secret::new);
        resolve_openrouter_key(section.as_ref(), llm, OPENROUTER_API_KEY_ENV, env)
            .map(|s| s.expose().to_string())
    }

    #[test]
    fn parallel_tool_calls_is_optional() {
        let unset: LlmConfig = toml::from_str("").unwrap();
        assert_eq!(
            unset.parallel_tool_calls, None,
            "unset keeps the provider default"
        );
        let off: LlmConfig = toml::from_str("parallel_tool_calls = false").unwrap();
        assert_eq!(off.parallel_tool_calls, Some(false));
    }

    #[test]
    fn openrouter_api_key_only_for_openrouter_provider() {
        assert_eq!(
            llm(LlmProviderKind::OpenRouter, Some("k2"))
                .openrouter_api_key()
                .map(Secret::expose),
            Some("k2"),
            "an OpenRouter [llm] key is reusable"
        );
        assert!(
            llm(LlmProviderKind::Anthropic, Some("sk-ant"))
                .openrouter_api_key()
                .is_none(),
            "a non-OpenRouter [llm] key must never be offered to OpenRouter"
        );
    }

    #[test]
    fn section_key_wins_over_llm_and_env() {
        let llm = llm(LlmProviderKind::OpenRouter, Some("k2"));
        assert_eq!(
            resolved(Some("k1"), &llm, env_with(Some("k3"))).as_deref(),
            Some("k1"),
            "the section's own api_key comes first"
        );
    }

    #[test]
    fn openrouter_llm_key_is_reused_before_env() {
        let llm = llm(LlmProviderKind::OpenRouter, Some("k2"));
        assert_eq!(
            resolved(None, &llm, env_with(Some("k3"))).as_deref(),
            Some("k2"),
            "[llm] key is reused when [llm] is OpenRouter"
        );
    }

    #[test]
    fn non_openrouter_llm_key_is_never_reused() {
        let llm = llm(LlmProviderKind::Anthropic, Some("sk-ant"));
        assert_eq!(
            resolved(None, &llm, env_with(None)),
            None,
            "an Anthropic key must not resolve as an OpenRouter key"
        );
        assert_eq!(
            resolved(None, &llm, env_with(Some("k3"))).as_deref(),
            Some("k3"),
            "env is the fallback when [llm] is not OpenRouter"
        );
    }

    #[test]
    fn blank_keys_count_as_unset() {
        let llm = llm(LlmProviderKind::OpenRouter, Some("  "));
        assert_eq!(
            resolved(Some(""), &llm, env_with(Some("k3"))).as_deref(),
            Some("k3"),
            "blank section and [llm] keys fall through to env"
        );
        assert_eq!(
            resolved(None, &llm, env_with(Some(" "))),
            None,
            "a blank env value is not a key"
        );
    }

    #[test]
    fn env_var_name_is_caller_supplied() {
        let llm = llm(LlmProviderKind::Ollama, None);
        let key = resolve_openrouter_key(None, &llm, "CUSTOM_KEY", |name| {
            (name == "CUSTOM_KEY").then(|| "k4".to_string())
        });
        assert_eq!(
            key.as_ref().map(Secret::expose),
            Some("k4"),
            "the decision model passes its own api_key_env"
        );
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
        assert_eq!(
            cfg.api_key.as_ref().map(Secret::expose),
            Some("pa-test-key")
        );
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
        assert_eq!(emb.api_key.as_ref().map(Secret::expose), Some("pa-secret"));
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
