//! HTTP client for OpenRouter's Chat Completions endpoint.

use futures::StreamExt as _;
use reqwest_middleware::ClientWithMiddleware;
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::types::llm::OpenRouterRouting;
use assistant_core::{LlmResponse, Secret, StreamChunk};

use super::errors::http_error;
use super::media;
use super::stream::{LineBuffer, StreamAccumulator};
use super::wire::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse};

/// OpenRouter accepts at most this many texts per embeddings request.
const MAX_EMBEDDING_INPUTS: usize = 2048;
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
        let mut response: ChatResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenRouter response: {e}"))?;
        let image_urls = response.take_image_urls();
        let response = response.into_llm_response(&request.model, shows_reasoning(request))?;
        self.with_images(response, &image_urls).await
    }

    /// Attaches the images at `urls` to `response`.
    async fn with_images(
        &self,
        mut response: LlmResponse,
        urls: &[String],
    ) -> anyhow::Result<LlmResponse> {
        if !urls.is_empty() {
            let images = media::resolve(&self.http, urls).await?;
            response.meta_mut().media.extend(images);
        }
        Ok(response)
    }

    /// Streams `request`, forwarding text deltas to `sink`. Returns the
    /// response and the PCM16 of a spoken reply, if the stream carried one.
    pub async fn chat_stream(
        &self,
        request: &ChatRequest,
        sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<(LlmResponse, Option<Vec<u8>>)> {
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
                    break;
                }
            }
            if acc.is_done() {
                break;
            }
        }
        let image_urls = acc.take_image_urls();
        let pcm = acc.take_audio();
        let response = finish(acc, sink.as_ref()).await?;
        Ok((self.with_images(response, &image_urls).await?, pcm))
    }

    /// Posts a chat `request`, retrying transient failures.
    async fn send(&self, request: &ChatRequest) -> anyhow::Result<reqwest::Response> {
        debug!(
            model = %request.model,
            tools = request.tools.len(),
            stream = request.stream,
            "Sending OpenRouter request"
        );
        self.post("chat/completions", request).await
    }

    /// Embeds `texts` with `model`, in input order, in requests of at most
    /// [`MAX_EMBEDDING_INPUTS`] texts, sending `routing` as `provider`.
    pub async fn embed(
        &self,
        model: &str,
        texts: &[&str],
        routing: Option<&OpenRouterRouting>,
    ) -> anyhow::Result<Vec<Vec<f32>>> {
        let mut vectors = Vec::with_capacity(texts.len());
        for batch in texts.chunks(MAX_EMBEDDING_INPUTS) {
            let request = EmbeddingRequest {
                model,
                input: batch,
                encoding_format: "float",
                provider: routing,
            };
            let response: EmbeddingResponse = self
                .post("embeddings", &request)
                .await
                .map_err(|e| e.context(format!("OpenRouter embedding model `{model}`")))?
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to parse OpenRouter embeddings: {e}"))?;
            vectors.extend(response.into_vectors(batch.len())?);
        }
        Ok(vectors)
    }

    /// Posts `body` to `{base_url}/{path}`, retrying transient failures.
    async fn post<B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<reqwest::Response> {
        with_retry_hinted(&self.retry, "OpenRouter", classify_provider_error, || {
            self.send_once(path, body)
        })
        .await
    }

    /// Posts `request` once and fails on a non-success status.
    async fn send_once<B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<reqwest::Response> {
        let url = format!("{}/{path}", self.base_url.trim_end_matches('/'));
        let mut builder = self
            .http
            .post(&url)
            .bearer_auth(self.api_key.expose())
            .json(body);
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

/// Streams the Sources block, if any, then returns the final response.
async fn finish(
    acc: StreamAccumulator,
    sink: Option<&mpsc::Sender<StreamChunk>>,
) -> anyhow::Result<LlmResponse> {
    if let (Some(block), Some(sink)) = (acc.sources_block(), sink) {
        let _ = sink.send(StreamChunk::Text(block)).await;
    }
    acc.finish()
}
