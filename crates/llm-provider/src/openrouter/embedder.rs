//! A dedicated OpenRouter embedding provider (`[llm.embeddings] provider = "openrouter"`).

use async_trait::async_trait;

use assistant_core::types::llm::OpenRouterRouting;
use assistant_core::{EmbeddingProvider, Secret};

use super::client::OpenRouterClient;

/// Embeds through OpenRouter's embeddings endpoint, independent of the chat
/// provider.
pub struct OpenRouterEmbedder {
    client: OpenRouterClient,
    model: String,
    routing: Option<OpenRouterRouting>,
}

impl OpenRouterEmbedder {
    pub fn new(
        base_url: String,
        api_key: Secret,
        timeout_secs: u64,
        model: String,
        routing: Option<OpenRouterRouting>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            client: OpenRouterClient::new(base_url, api_key, timeout_secs, Vec::new())?,
            model,
            routing,
        })
    }
}

#[async_trait]
impl EmbeddingProvider for OpenRouterEmbedder {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        self.embed_batch(&[text])
            .await?
            .pop()
            .ok_or_else(|| anyhow::anyhow!("OpenRouter returned no embedding"))
    }

    async fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        self.client
            .embed(&self.model, texts, self.routing.as_ref())
            .await
    }
}
