//! HTTP client for OpenRouter's Chat Completions endpoint.

use futures::StreamExt as _;
use reqwest_middleware::ClientWithMiddleware;
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::{LlmResponse, Secret, StreamChunk};

use super::stream::{LineBuffer, StreamAccumulator};
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
        let body = self
            .send(request)
            .await?
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read OpenRouter response: {e}"))?;
        let response: ChatResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenRouter response: {e}"))?;
        response.into_llm_response()
    }

    /// Streams `request`, forwarding text deltas to `sink`.
    pub async fn chat_stream(
        &self,
        request: &ChatRequest,
        sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        let mut bytes = self.send(request).await?.bytes_stream();
        let mut lines = LineBuffer::default();
        let mut acc = StreamAccumulator::default();
        while let Some(chunk) = bytes.next().await {
            let chunk = chunk.map_err(|e| anyhow::anyhow!("OpenRouter stream failed: {e}"))?;
            for line in lines.push(&chunk) {
                if let Some(text) = acc.feed_line(&line)?
                    && let Some(sink) = &sink
                {
                    let _ = sink.send(StreamChunk::Text(text)).await;
                }
                if acc.is_done() {
                    return acc.finish();
                }
            }
        }
        acc.finish()
    }

    /// Posts `request` and fails on a non-success status.
    async fn send(&self, request: &ChatRequest) -> anyhow::Result<reqwest::Response> {
        debug!(
            model = %request.model,
            tools = request.tools.len(),
            stream = request.stream,
            "Sending OpenRouter request"
        );
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
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter returned {status}: {body}");
        }
        Ok(resp)
    }
}
