//! `OllamaProvider` — [`LlmProvider`] implementation backed by Ollama.

pub mod client;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::mpsc;

use assistant_core::{
    Capabilities, ChatHistoryMessage, LlmConfig, LlmProvider, LlmResponse, StreamChunk, ToolSpec,
    ToolSupport,
};

use self::client::{LlmClient, LlmClientConfig};

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
pub struct OllamaProvider {
    inner: LlmClient,
    base_url: String,
    embedding_model: String,
    http: reqwest_middleware::ClientWithMiddleware,
}

impl OllamaProvider {
    /// Create a new provider from explicit configuration.
    pub fn new(config: OllamaConfig) -> anyhow::Result<Self> {
        let client_config = LlmClientConfig {
            model: config.model,
            base_url: config.base_url.clone(),
            timeout_secs: config.timeout_secs,
            retry_config: crate::retry::RetryConfig::default(),
        };
        let http = crate::http::build_http_client(
            config.timeout_secs,
            &crate::retry::RetryConfig::default(),
        )?;
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
            hosted_tools: Vec::new(),
            context_window_tokens: None, // varies by model
        }
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
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        self.inner
            .chat_streaming(system_prompt, history, tools, chunk_sink)
            .await
    }

    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let vectors = self.embed_batch(&[text]).await?;
        vectors
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty embeddings array in response"))
    }

    async fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Ollama's `/api/embed` accepts a string OR an array of strings under
        // `"input"` and returns `{"embeddings": [[…], [...]]}`. Sending the
        // whole batch in one call avoids `texts.len()` round-trips (#30).
        let url = format!("{}/api/embed", self.base_url);
        let body = serde_json::json!({
            "model": self.embedding_model,
            "input": texts,
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

        if embed_resp.embeddings.len() != texts.len() {
            anyhow::bail!(
                "Ollama returned {} embeddings for {} inputs",
                embed_resp.embeddings.len(),
                texts.len()
            );
        }
        Ok(embed_resp.embeddings)
    }

    fn provider_name(&self) -> &str {
        self.inner.provider_name()
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn server_address(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::LlmProvider;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider_pointed_at(base_url: &str) -> OllamaProvider {
        OllamaProvider::new(OllamaConfig {
            model: "qwen2.5:7b".to_string(),
            base_url: base_url.to_string(),
            timeout_secs: 5,
            embedding_model: "nomic-embed-text".to_string(),
        })
        .unwrap()
    }

    /// `embed_batch` SHALL send a single request with `"input": [...]` and
    /// return all vectors in input order (#30).
    #[tokio::test]
    async fn embed_batch_sends_array_input_in_one_request() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/embed"))
            .and(body_json(serde_json::json!({
                "model": "nomic-embed-text",
                "input": ["foo", "bar", "baz"],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "embeddings": [
                    [1.0, 2.0],
                    [3.0, 4.0],
                    [5.0, 6.0],
                ],
            })))
            .expect(1)
            .mount(&server)
            .await;

        let provider = provider_pointed_at(&server.uri());
        let vectors = provider.embed_batch(&["foo", "bar", "baz"]).await.unwrap();

        assert_eq!(
            vectors,
            vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]],
            "vectors must be returned in input order"
        );
    }

    /// Single-input `embed()` continues to work and routes through the same
    /// batched endpoint.
    #[tokio::test]
    async fn embed_single_still_works() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/embed"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "embeddings": [[0.1, 0.2, 0.3]],
            })))
            .expect(1)
            .mount(&server)
            .await;

        let provider = provider_pointed_at(&server.uri());
        let vec = provider.embed("hello").await.unwrap();
        assert_eq!(vec, vec![0.1, 0.2, 0.3]);
    }

    /// Empty batch SHALL short-circuit without an HTTP request.
    #[tokio::test]
    async fn embed_batch_empty_skips_request() {
        let server = MockServer::start().await;
        // No mock mounted — any request hits a 404 from wiremock and fails
        // the test. The call must return [] without touching the server.

        let provider = provider_pointed_at(&server.uri());
        let vectors: Vec<_> = vec![];
        let texts: Vec<&str> = vectors.iter().copied().collect();
        let result = provider.embed_batch(&texts).await.unwrap();
        assert!(result.is_empty());
    }

    /// Mismatched response length SHALL surface as an error rather than
    /// silently corrupting downstream embeddings.
    #[tokio::test]
    async fn embed_batch_rejects_mismatched_response_length() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/embed"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "embeddings": [[1.0]],  // only one — but we asked for two
            })))
            .mount(&server)
            .await;

        let provider = provider_pointed_at(&server.uri());
        let err = provider.embed_batch(&["a", "b"]).await.unwrap_err();
        assert!(
            err.to_string().contains("mismatch") || err.to_string().contains("1 embeddings for 2"),
            "expected length-mismatch error, got: {err}"
        );
    }
}
