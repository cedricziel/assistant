//! Consolidated LLM provider implementations.
//!
//! Houses all provider backends (Ollama, Anthropic, OpenAI, Moonshot,
//! OpenRouter) in a single crate with shared infrastructure.  Each provider
//! lives in its own submodule and implements [`assistant_core::LlmProvider`].

use std::sync::Arc;

use assistant_core::types::LlmProviderKind;
use assistant_core::{LlmConfig, LlmProvider};

// -- Provider implementations --
pub mod anthropic;
pub mod moonshot;
pub mod ollama;
pub mod openai;
pub mod openrouter;

// -- Shared provider infrastructure --
pub mod chat_completions;
pub mod http;
pub mod retry;
pub mod voyage;

pub use anthropic::{AnthropicConfig, AnthropicProvider};
pub use moonshot::MoonshotProvider;
pub use ollama::{OllamaConfig, OllamaProvider};
pub use openai::{OAuthManager, OpenAIProvider, OpenAIProviderConfig};
pub use openrouter::OpenRouterProvider;
pub use voyage::{VoyageConfig, VoyageEmbedder};

/// Create an [`LlmProvider`] from the given configuration.
///
/// Dispatches on [`LlmProviderKind`] and returns the appropriate provider
/// wrapped in `Arc<dyn LlmProvider>`.
pub fn create_provider(config: &LlmConfig) -> anyhow::Result<Arc<dyn LlmProvider>> {
    match config.provider {
        LlmProviderKind::Ollama => Ok(Arc::new(OllamaProvider::from_llm_config(config)?)),
        LlmProviderKind::Anthropic => Ok(Arc::new(AnthropicProvider::from_llm_config(config)?)),
        LlmProviderKind::OpenAI => Ok(Arc::new(OpenAIProvider::from_llm_config(config)?)),
        LlmProviderKind::Moonshot => Ok(Arc::new(MoonshotProvider::from_llm_config(config)?)),
        LlmProviderKind::OpenRouter => Ok(Arc::new(OpenRouterProvider::from_llm_config(config)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_creates_ollama_provider() {
        let config = LlmConfig {
            provider: LlmProviderKind::Ollama,
            ..LlmConfig::default()
        };
        let provider = create_provider(&config).expect("Ollama provider should succeed");
        assert_eq!(provider.provider_name(), "ollama");
    }

    #[test]
    fn factory_creates_anthropic_provider_with_key() {
        let config = LlmConfig {
            provider: LlmProviderKind::Anthropic,
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };
        let provider = create_provider(&config).expect("Anthropic provider should succeed");
        assert_eq!(provider.provider_name(), "anthropic");
    }

    #[test]
    fn factory_errors_on_missing_anthropic_key() {
        // Clear env to ensure no fallback key
        // SAFETY: this test runs serially and no other thread reads this var concurrently.
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
        let config = LlmConfig {
            provider: LlmProviderKind::Anthropic,
            api_key: None,
            ..LlmConfig::default()
        };
        let result = create_provider(&config);
        assert!(result.is_err(), "should fail without API key");
        let err = result.err().unwrap();
        assert!(
            err.to_string().contains("API key"),
            "error should mention API key: {err}"
        );
    }

    #[test]
    fn factory_creates_openai_provider_with_key() {
        let config = LlmConfig {
            provider: LlmProviderKind::OpenAI,
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };
        let provider = create_provider(&config).expect("OpenAI provider should succeed");
        assert_eq!(provider.provider_name(), "openai");
    }

    #[test]
    fn factory_creates_moonshot_provider_with_key() {
        let config = LlmConfig {
            provider: LlmProviderKind::Moonshot,
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };
        let provider = create_provider(&config).expect("Moonshot provider should succeed");
        assert_eq!(provider.provider_name(), "moonshot");
    }

    #[test]
    fn factory_creates_openrouter_provider_with_key() {
        let config = LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: Some("test-key".to_string()),
            ..LlmConfig::default()
        };
        let provider = create_provider(&config).expect("OpenRouter provider should succeed");
        assert_eq!(provider.provider_name(), "openrouter");
    }
}
