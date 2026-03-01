//! `MoonshotProvider` — thin facade over [`OpenAIProvider`] for the Moonshot AI
//! (Kimi) chat completions API.
//!
//! Moonshot exposes an OpenAI-compatible `/v1/chat/completions` endpoint, so we
//! delegate all heavy lifting to the existing OpenAI provider and only override
//! construction defaults and OTel metadata.

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::LlmConfig;
use assistant_llm::{Capabilities, ChatHistoryMessage, LlmProvider, LlmResponse, ToolSpec};
use assistant_provider_openai::{OpenAIProvider, OpenAIProviderConfig};

// ── Defaults ──────────────────────────────────────────────────────────────────

const DEFAULT_BASE_URL: &str = "https://api.moonshot.ai/v1";
const DEFAULT_MAX_TOKENS: u32 = 8192;

// ── MoonshotProvider ──────────────────────────────────────────────────────────

/// [`LlmProvider`] backed by the Moonshot AI (Kimi) chat completions API.
///
/// Internally delegates to [`OpenAIProvider`] since the wire protocol is
/// identical.  Provides Moonshot-specific defaults for base URL, model, and
/// API-key resolution (`MOONSHOT_API_KEY` env var).
pub struct MoonshotProvider {
    inner: OpenAIProvider,
    /// Kept separately so `server_address()` returns the Moonshot URL, not
    /// whatever the inner provider normalised.
    base_url: String,
    model: String,
}

impl MoonshotProvider {
    /// Create from explicit config values.
    pub fn new(
        model: String,
        base_url: String,
        api_key: &str,
        timeout_secs: u64,
        max_tokens: u32,
    ) -> anyhow::Result<Self> {
        let openai_cfg = OpenAIProviderConfig {
            model: model.clone(),
            base_url: base_url.clone(),
            timeout_secs,
            max_tokens,
            embedding_model: String::new(), // Moonshot has no embedding endpoint
        };

        let inner = OpenAIProvider::new(openai_cfg, api_key)?;

        debug!(
            model = %model,
            base_url = %base_url,
            "Moonshot provider initialised (delegating to OpenAI provider)"
        );

        Ok(Self {
            inner,
            base_url,
            model,
        })
    }

    /// Convenience constructor directly from [`LlmConfig`].
    ///
    /// Resolves the API key from `config.api_key` or the `MOONSHOT_API_KEY`
    /// environment variable.
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        let api_key = cfg
            .api_key
            .clone()
            .or_else(|| std::env::var("MOONSHOT_API_KEY").ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Moonshot API key not found. Set api_key in [llm] config or \
                     MOONSHOT_API_KEY environment variable."
                )
            })?;

        // If the base_url is still the default Ollama value, swap in the
        // Moonshot default.
        let base_url = if cfg.base_url == "http://localhost:11434" {
            DEFAULT_BASE_URL.to_string()
        } else {
            cfg.base_url.clone()
        };

        let max_tokens = cfg.moonshot.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

        Self::new(
            cfg.model.clone(),
            base_url,
            &api_key,
            cfg.timeout_secs,
            max_tokens,
        )
    }
}

// ── LlmProvider ───────────────────────────────────────────────────────────────

#[async_trait]
impl LlmProvider for MoonshotProvider {
    fn capabilities(&self) -> Capabilities {
        // Same as OpenAI — native tool support, streaming, vision.
        self.inner.capabilities()
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        self.inner.chat(system_prompt, history, tools).await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        self.inner
            .chat_streaming(system_prompt, history, tools, token_sink)
            .await
    }

    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!(
            "Moonshot AI does not support text embeddings"
        ))
    }

    fn provider_name(&self) -> &str {
        "moonshot"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn server_address(&self) -> &str {
        &self.base_url
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constants_are_sensible() {
        assert_eq!(DEFAULT_BASE_URL, "https://api.moonshot.ai/v1");
        assert!(DEFAULT_MAX_TOKENS > 0);
    }
}
