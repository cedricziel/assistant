//! HTTP client for OpenRouter's Chat Completions endpoint.

use futures::StreamExt as _;
use reqwest_middleware::ClientWithMiddleware;
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::{LlmResponse, Secret, StreamChunk};

use super::errors::http_error;
use super::stream::{LineBuffer, StreamAccumulator};
use super::wire::{ChatRequest, ChatResponse};
use crate::retry::{RetryConfig, classify_provider_error, parse_retry_after, with_retry_hinted};

pub struct OpenRouterClient {
    http: ClientWithMiddleware,
    base_url: String,
    api_key: Secret,
    headers: Vec<(String, String)>,
    pub(super) retry: RetryConfig,
}

impl OpenRouterClient {
    pub fn new(
        base_url: String,
        api_key: Secret,
        timeout_secs: u64,
        headers: Vec<(String, String)>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            // Retries happen in `send`, where `Retry-After` is honored.
            http: crate::http::build_http_client(timeout_secs, &RetryConfig::disabled())?,
            base_url,
            api_key,
            headers,
            retry: RetryConfig::default(),
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
        response.into_llm_response(&request.model, shows_reasoning(request))
    }

    /// Streams `request`, forwarding text deltas to `sink`.
    pub async fn chat_stream(
        &self,
        request: &ChatRequest,
        sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        let mut bytes = self.send(request).await?.bytes_stream();
        let mut lines = LineBuffer::default();
        let mut acc = StreamAccumulator::new(&request.model, shows_reasoning(request));
        while let Some(chunk) = bytes.next().await {
            let chunk = chunk.map_err(|e| anyhow::anyhow!("OpenRouter stream failed: {e}"))?;
            for line in lines.push(&chunk) {
                for chunk in acc.feed_line(&line)? {
                    if let Some(sink) = &sink {
                        let _ = sink.send(chunk).await;
                    }
                }
                if acc.is_done() {
                    return acc.finish();
                }
            }
        }
        acc.finish()
    }

    /// Posts `request`, retrying transient failures.
    async fn send(&self, request: &ChatRequest) -> anyhow::Result<reqwest::Response> {
        with_retry_hinted(&self.retry, "OpenRouter", classify_provider_error, || {
            self.send_once(request)
        })
        .await
    }

    /// Posts `request` once and fails on a non-success status.
    async fn send_once(&self, request: &ChatRequest) -> anyhow::Result<reqwest::Response> {
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
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_retry_after);
            let body = resp.text().await.unwrap_or_default();
            return Err(http_error(status.as_u16(), retry_after, &body).into());
        }
        Ok(resp)
    }
}

/// Reasoning is shown unless `[llm.openrouter.reasoning] exclude` is set.
fn shows_reasoning(request: &ChatRequest) -> bool {
    !request
        .reasoning
        .as_ref()
        .and_then(|r| r.exclude)
        .unwrap_or(false)
}
