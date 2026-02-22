//! `OllamaProvider` — [`LlmProvider`] implementation backed by Ollama.

use async_trait::async_trait;
use tokio::sync::mpsc;

use assistant_core::{LlmConfig, SkillDef};
use assistant_llm::{
    Capabilities, ChatHistoryMessage, LlmClient, LlmClientConfig, LlmProvider, LlmResponse,
    ToolSupport,
};

// ── OllamaConfig ─────────────────────────────────────────────────────────────

/// Configuration for the Ollama backend.
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Ollama model name (e.g. `"qwen2.5:7b"`).
    pub model: String,
    /// Base URL of the Ollama server (e.g. `"http://localhost:11434"`).
    pub base_url: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            model: "qwen2.5:7b".to_string(),
            base_url: "http://localhost:11434".to_string(),
            timeout_secs: 120,
        }
    }
}

impl From<&LlmConfig> for OllamaConfig {
    fn from(cfg: &LlmConfig) -> Self {
        Self {
            model: cfg.model.clone(),
            base_url: cfg.base_url.clone(),
            timeout_secs: cfg.timeout_secs,
        }
    }
}

// ── OllamaProvider ────────────────────────────────────────────────────────────

/// [`LlmProvider`] implementation backed by the Ollama `/api/chat` endpoint.
///
/// Wraps [`LlmClient`] from `assistant-llm`.  Once the HTTP logic is fully
/// migrated here, `LlmClient` can be inlined or removed.
pub struct OllamaProvider {
    inner: LlmClient,
}

impl OllamaProvider {
    /// Create a new provider from explicit configuration.
    pub fn new(config: OllamaConfig) -> anyhow::Result<Self> {
        let client_config = LlmClientConfig {
            model: config.model,
            base_url: config.base_url,
            timeout_secs: config.timeout_secs,
        };
        Ok(Self {
            inner: LlmClient::new(client_config)?,
        })
    }

    /// Convenience constructor directly from [`LlmConfig`].
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        Self::new(OllamaConfig::from(cfg))
    }
}

// ── LlmProvider impl ─────────────────────────────────────────────────────────

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: ToolSupport::Native,
            streaming: true,
            vision: false,
        }
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
    ) -> anyhow::Result<LlmResponse> {
        self.inner.chat(system_prompt, history, skills).await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        skills: &[&SkillDef],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        self.inner
            .chat_streaming(system_prompt, history, skills, token_sink)
            .await
    }
}
