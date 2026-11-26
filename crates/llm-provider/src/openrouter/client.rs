//! HTTP client for OpenRouter's Chat Completions endpoint.

use reqwest_middleware::ClientWithMiddleware;
use tracing::debug;

use assistant_core::{LlmResponse, Secret};

use super::wire::{ChatRequest, ChatResponse};
use crate::retry::RetryConfig;

pub struct OpenRouterClient {
    http: ClientWithMiddleware,
    base_url: String,
    api_key: Secret,
    headers: Vec<(String, String)>,
}

impl OpenRouterClient {
    pub fn new(
        base_url: String,
        api_key: Secret,
        timeout_secs: u64,
        headers: Vec<(String, String)>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            http: crate::http::build_http_client(timeout_secs, &RetryConfig::default())?,
            base_url,
            api_key,
            headers,
        })
    }

    pub async fn chat(&self, request: &ChatRequest) -> anyhow::Result<LlmResponse> {
        debug!(model = %request.model, tools = request.tools.len(), "Sending OpenRouter request");
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut builder = self
            .http
            .post(&url)
            .bearer_auth(self.api_key.expose())
            .json(request);
        for (name, value) in &self.headers {
            builder = builder.header(name, value);
        }
        let resp = builder
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OpenRouter request failed: {e}"))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read OpenRouter response: {e}"))?;
        if !status.is_success() {
            anyhow::bail!("OpenRouter returned {status}: {body}");
        }
        let response: ChatResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenRouter response: {e}"))?;
        response.into_llm_response()
    }
}
