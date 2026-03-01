//! Ollama transcription provider.
//!
//! Ollama exposes a Whisper-compatible `/v1/audio/transcriptions` endpoint
//! when running a whisper model (e.g. `whisper-large-v3-turbo`).
//!
//! The wire format is identical to OpenAI's Whisper API, so this
//! implementation shares the same multipart upload logic.

use anyhow::{bail, Context};
use async_trait::async_trait;
use reqwest::multipart;
use reqwest_middleware::ClientWithMiddleware;
use tracing::{debug, warn};

use crate::provider::{
    extension_for_mime, TranscriptionProvider, TranscriptionRequest, TranscriptionResult,
};

/// Default timeout for transcription requests (120 s — audio can be long).
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Ollama-hosted transcription provider.
///
/// Expects an Ollama server with a whisper-compatible model pulled
/// (e.g. `ollama pull whisper-large-v3-turbo`).
pub struct OllamaTranscriptionProvider {
    /// Base URL of the Ollama server (default: `http://localhost:11434/v1`).
    base_url: String,
    /// Model name (default: `whisper-large-v3-turbo`).
    model: String,
    client: ClientWithMiddleware,
}

impl OllamaTranscriptionProvider {
    pub fn new() -> anyhow::Result<Self> {
        let client = assistant_llm::build_http_client(DEFAULT_TIMEOUT_SECS)?;
        Ok(Self {
            base_url: "http://localhost:11434/v1".to_string(),
            model: "whisper-large-v3-turbo".to_string(),
            client,
        })
    }

    /// Override the base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Override the model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

/// JSON response (Whisper-compatible format).
#[derive(serde::Deserialize)]
struct TranscribeResponse {
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
}

#[async_trait]
impl TranscriptionProvider for OllamaTranscriptionProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> anyhow::Result<TranscriptionResult> {
        let filename = request
            .filename
            .unwrap_or_else(|| format!("audio.{}", extension_for_mime(&request.mime_type)));

        debug!(
            provider = "ollama",
            model = %self.model,
            mime = %request.mime_type,
            filename = %filename,
            size = request.audio_data.len(),
            "Transcribing audio via Ollama"
        );

        let file_part = multipart::Part::bytes(request.audio_data)
            .file_name(filename)
            .mime_str(&request.mime_type)
            .context("Invalid MIME type for multipart upload")?;

        let mut form = multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "verbose_json");

        if let Some(lang) = &request.language {
            form = form.text("language", lang.clone());
        }

        let url = format!("{}/audio/transcriptions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .context("Ollama transcription request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(status = %status, body = %body, "Ollama transcription error");
            bail!("Ollama transcription returned {status}: {body}");
        }

        let result: TranscribeResponse = resp
            .json()
            .await
            .context("Failed to parse Ollama transcription response")?;

        debug!(
            text_len = result.text.len(),
            language = ?result.language,
            duration = ?result.duration,
            "Ollama transcription complete"
        );

        Ok(TranscriptionResult {
            text: result.text,
            language: result.language,
            duration_secs: result.duration,
        })
    }
}
