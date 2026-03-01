//! Deepgram speech-to-text transcription provider.
//!
//! Uses Deepgram's `POST /v1/listen` pre-recorded audio endpoint.
//! See <https://developers.deepgram.com/reference/listen-file> for API docs.

use anyhow::{bail, Context};
use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use tracing::{debug, warn};
use url::Url;

use crate::provider::{TranscriptionProvider, TranscriptionRequest, TranscriptionResult};

/// Default timeout for transcription requests (120 s — audio can be long).
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Deepgram transcription provider.
pub struct DeepgramProvider {
    /// API key for authentication.
    api_key: String,
    /// Model to use (default: `nova-3`).
    model: String,
    /// Base URL (default: `https://api.deepgram.com/v1`).
    base_url: String,
    client: ClientWithMiddleware,
}

impl DeepgramProvider {
    pub fn new(api_key: impl Into<String>) -> anyhow::Result<Self> {
        let client = assistant_llm::build_http_client(DEFAULT_TIMEOUT_SECS)?;
        Ok(Self {
            api_key: api_key.into(),
            model: "nova-3".to_string(),
            base_url: "https://api.deepgram.com/v1".to_string(),
            client,
        })
    }

    /// Override the model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Override the base URL (useful for on-prem Deepgram deployments).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

// ── Deepgram response types ──────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct DeepgramResponse {
    results: Option<DeepgramResults>,
    metadata: Option<DeepgramMetadata>,
}

#[derive(serde::Deserialize)]
struct DeepgramResults {
    channels: Vec<DeepgramChannel>,
}

#[derive(serde::Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
    #[serde(default)]
    detected_language: Option<String>,
}

#[derive(serde::Deserialize)]
struct DeepgramAlternative {
    transcript: String,
}

#[derive(serde::Deserialize)]
struct DeepgramMetadata {
    #[serde(default)]
    duration: Option<f64>,
}

#[async_trait]
impl TranscriptionProvider for DeepgramProvider {
    fn name(&self) -> &str {
        "deepgram"
    }

    async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> anyhow::Result<TranscriptionResult> {
        debug!(
            provider = "deepgram",
            model = %self.model,
            mime = %request.mime_type,
            size = request.audio_data.len(),
            "Transcribing audio via Deepgram"
        );

        let mut url = Url::parse(&format!("{}/listen", self.base_url))
            .context("Invalid Deepgram base URL")?;
        url.query_pairs_mut()
            .append_pair("model", &self.model)
            .append_pair("smart_format", "true")
            .append_pair("detect_language", "true");
        if let Some(lang) = &request.language {
            url.query_pairs_mut().append_pair("language", lang);
        }

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", &request.mime_type)
            .body(request.audio_data)
            .send()
            .await
            .context("Deepgram API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(status = %status, body = %body, "Deepgram API error");
            bail!("Deepgram API returned {status}: {body}");
        }

        let dg: DeepgramResponse = resp
            .json()
            .await
            .context("Failed to parse Deepgram response")?;

        let (text, language) = dg
            .results
            .and_then(|r| r.channels.into_iter().next())
            .map(|ch| {
                let transcript = ch
                    .alternatives
                    .into_iter()
                    .next()
                    .map(|a| a.transcript)
                    .unwrap_or_default();
                (transcript, ch.detected_language)
            })
            .unwrap_or_default();

        let duration_secs = dg.metadata.and_then(|m| m.duration);

        debug!(
            text_len = text.len(),
            language = ?language,
            duration = ?duration_secs,
            "Deepgram transcription complete"
        );

        Ok(TranscriptionResult {
            text,
            language,
            duration_secs,
        })
    }
}
