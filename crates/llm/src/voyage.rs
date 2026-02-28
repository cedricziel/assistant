//! Voyage AI embedding client.
//!
//! Voyage AI is the embedding provider recommended by Anthropic.  This module
//! provides a lightweight [`EmbeddingProvider`] implementation that calls the
//! Voyage AI `/v1/embeddings` REST endpoint.
//!
//! # Configuration
//!
//! ```toml
//! [llm.embeddings]
//! provider = "voyage"
//! model = "voyage-3-lite"        # optional, this is the default
//! # api_key = "pa-..."           # or set VOYAGE_API_KEY env var
//! ```

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tracing::debug;

use crate::embedding::EmbeddingProvider;

// ── Defaults ─────────────────────────────────────────────────────────────────

/// Default Voyage AI base URL.
pub const DEFAULT_VOYAGE_BASE_URL: &str = "https://api.voyageai.com";

/// Default embedding model (good balance of quality and cost).
pub const DEFAULT_VOYAGE_MODEL: &str = "voyage-3-lite";

// ── VoyageConfig ─────────────────────────────────────────────────────────────

/// Configuration for the Voyage AI embedder.
#[derive(Debug, Clone)]
pub struct VoyageConfig {
    /// Voyage AI API key.
    pub api_key: String,
    /// Base URL (default: `"https://api.voyageai.com"`).
    pub base_url: String,
    /// Model name (default: `"voyage-3-lite"`).
    pub model: String,
}

impl VoyageConfig {
    /// Create a new config with the given API key and sensible defaults.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: DEFAULT_VOYAGE_BASE_URL.to_string(),
            model: DEFAULT_VOYAGE_MODEL.to_string(),
        }
    }

    /// Override the base URL.
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Override the model name.
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

// ── Voyage API response types ────────────────────────────────────────────────

#[derive(Deserialize)]
struct VoyageEmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct VoyageEmbedResponse {
    data: Vec<VoyageEmbeddingData>,
}

// ── VoyageEmbedder ───────────────────────────────────────────────────────────

/// Voyage AI embedding client implementing [`EmbeddingProvider`].
pub struct VoyageEmbedder {
    config: VoyageConfig,
    http: reqwest::Client,
}

impl VoyageEmbedder {
    /// Create a new Voyage AI embedding client.
    pub fn new(config: VoyageConfig) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Voyage HTTP client: {e}"))?;
        Ok(Self { config, http })
    }
}

#[async_trait]
impl EmbeddingProvider for VoyageEmbedder {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let url = format!(
            "{}/v1/embeddings",
            self.config.base_url.trim_end_matches('/')
        );
        let body = json!({
            "input": [text],
            "model": self.config.model,
        });

        debug!(model = %self.config.model, "Sending embedding request to Voyage AI");

        let resp = self
            .http
            .post(&url)
            .header("authorization", format!("Bearer {}", self.config.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Voyage AI embedding request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Voyage AI returned {status}: {text}");
        }

        let embed_resp: VoyageEmbedResponse = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Voyage AI response: {e}"))?;

        embed_resp
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow::anyhow!("Voyage AI returned empty embedding data"))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn voyage_embed_parses_response() {
        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "embedding": [0.1, 0.2, 0.3, 0.4],
                    "index": 0
                }
            ],
            "model": "voyage-3-lite",
            "usage": { "total_tokens": 5 }
        });

        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let config = VoyageConfig::new("test-key".to_string()).with_base_url(mock_server.uri());
        let embedder = VoyageEmbedder::new(config).unwrap();

        let result = embedder.embed("hello world").await.unwrap();
        assert_eq!(result, vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[tokio::test]
    async fn voyage_embed_propagates_http_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let config = VoyageConfig::new("bad-key".to_string()).with_base_url(mock_server.uri());
        let embedder = VoyageEmbedder::new(config).unwrap();

        let result = embedder.embed("hello").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("401"),
            "Error should mention status code: {err_msg}"
        );
    }

    #[tokio::test]
    async fn voyage_embed_handles_empty_data() {
        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "object": "list",
            "data": [],
            "model": "voyage-3-lite",
            "usage": { "total_tokens": 0 }
        });

        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let config = VoyageConfig::new("test-key".to_string()).with_base_url(mock_server.uri());
        let embedder = VoyageEmbedder::new(config).unwrap();

        let result = embedder.embed("hello").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("empty embedding data"));
    }
}
