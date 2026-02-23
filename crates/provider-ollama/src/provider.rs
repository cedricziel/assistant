//! `OllamaProvider` — [`LlmProvider`] implementation backed by Ollama.

use async_trait::async_trait;
use serde::Deserialize;
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
    /// Embedding model name (e.g. `"nomic-embed-text"`).
    pub embedding_model: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            model: "qwen2.5:7b".to_string(),
            base_url: "http://localhost:11434".to_string(),
            timeout_secs: 120,
            embedding_model: "nomic-embed-text".to_string(),
        }
    }
}

impl From<&LlmConfig> for OllamaConfig {
    fn from(cfg: &LlmConfig) -> Self {
        Self {
            model: cfg.model.clone(),
            base_url: cfg.base_url.clone(),
            timeout_secs: cfg.timeout_secs,
            embedding_model: cfg.embedding_model.clone(),
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
    base_url: String,
    embedding_model: String,
    http: reqwest::Client,
}

impl OllamaProvider {
    /// Create a new provider from explicit configuration.
    pub fn new(config: OllamaConfig) -> anyhow::Result<Self> {
        let client_config = LlmClientConfig {
            model: config.model,
            base_url: config.base_url.clone(),
            timeout_secs: config.timeout_secs,
        };
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))?;
        Ok(Self {
            inner: LlmClient::new(client_config)?,
            base_url: config.base_url,
            embedding_model: config.embedding_model,
            http,
        })
    }

    /// Convenience constructor directly from [`LlmConfig`].
    pub fn from_llm_config(cfg: &LlmConfig) -> anyhow::Result<Self> {
        Self::new(OllamaConfig::from(cfg))
    }
}

// ── Ollama /api/embed response ────────────────────────────────────────────────

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
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

    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let url = format!("{}/api/embed", self.base_url);
        let body = serde_json::json!({
            "model": self.embedding_model,
            "input": text,
        });
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Embed request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Embed API error {status}: {text}"));
        }

        let embed_resp: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse embed response: {e}"))?;

        embed_resp
            .embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty embeddings array in response"))
    }
}
