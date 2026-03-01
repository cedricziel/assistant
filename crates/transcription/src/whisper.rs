//! OpenAI Whisper API transcription provider.
//!
//! Uses the `POST /v1/audio/transcriptions` endpoint.  Compatible with
//! OpenAI's hosted API and any server that exposes the same endpoint
//! (e.g. LocalAI, vLLM with Whisper).

use anyhow::{bail, Context};
use async_trait::async_trait;
use reqwest::multipart;
use reqwest_middleware::ClientWithMiddleware;
use tracing::{debug, warn};

use crate::provider::{TranscriptionProvider, TranscriptionRequest, TranscriptionResult};

/// Default timeout for transcription requests (120 s — audio can be long).
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// OpenAI Whisper transcription provider.
pub struct WhisperProvider {
    /// Base URL (default: `https://api.openai.com/v1`).
    base_url: String,
    /// Bearer token for authentication.
    api_key: String,
    /// Model name (default: `whisper-1`).
    model: String,
    client: ClientWithMiddleware,
}

impl WhisperProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        let client = assistant_llm::build_http_client(DEFAULT_TIMEOUT_SECS)
            .expect("Failed to build HTTP client for Whisper provider");
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: api_key.into(),
            model: "whisper-1".to_string(),
            client,
        }
    }

    /// Override the base URL (useful for LocalAI or other compatible servers).
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

/// Guess a reasonable filename extension from a MIME type so the Whisper API
/// can detect the codec when the original filename is unavailable.
fn extension_for_mime(mime: &str) -> &str {
    match mime {
        "audio/ogg" | "audio/opus" => "ogg",
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/mp4" | "audio/m4a" | "audio/x-m4a" => "m4a",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/webm" => "webm",
        _ => "bin",
    }
}

/// JSON response from the Whisper transcriptions endpoint.
#[derive(serde::Deserialize)]
struct WhisperResponse {
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
}

#[async_trait]
impl TranscriptionProvider for WhisperProvider {
    fn name(&self) -> &str {
        "whisper"
    }

    async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> anyhow::Result<TranscriptionResult> {
        let filename = request
            .filename
            .unwrap_or_else(|| format!("audio.{}", extension_for_mime(&request.mime_type)));

        debug!(
            provider = "whisper",
            model = %self.model,
            mime = %request.mime_type,
            filename = %filename,
            size = request.audio_data.len(),
            "Transcribing audio"
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
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .context("Whisper API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(status = %status, body = %body, "Whisper API error");
            bail!("Whisper API returned {status}: {body}");
        }

        let whisper: WhisperResponse = resp
            .json()
            .await
            .context("Failed to parse Whisper API response")?;

        debug!(
            text_len = whisper.text.len(),
            language = ?whisper.language,
            duration = ?whisper.duration,
            "Transcription complete"
        );

        Ok(TranscriptionResult {
            text: whisper.text,
            language: whisper.language,
            duration_secs: whisper.duration,
        })
    }
}
