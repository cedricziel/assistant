//! Deepgram speech-to-text transcription provider.
//!
//! Uses Deepgram's `POST /v1/listen` pre-recorded audio endpoint.
//! See <https://developers.deepgram.com/reference/listen-file> for API docs.

use anyhow::{bail, Context};
use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use tracing::{debug, warn};
use url::Url;

use crate::converter::{AudioConverter, AudioFormat};
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
    /// Optional audio converter for formats not supported by Deepgram.
    converter: Option<AudioConverter>,
}

impl DeepgramProvider {
    pub fn new(api_key: impl Into<String>) -> anyhow::Result<Self> {
        let client = assistant_llm::build_http_client(DEFAULT_TIMEOUT_SECS)?;
        Ok(Self {
            api_key: api_key.into(),
            model: "nova-3".to_string(),
            base_url: "https://api.deepgram.com/v1".to_string(),
            client,
            converter: None,
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

    /// Enable audio format conversion for unsupported formats (e.g., M4A).
    ///
    /// When enabled, the provider will automatically convert audio formats
    /// that Deepgram doesn't support (like M4A/MP4/AAC) to WAV before sending.
    pub fn with_audio_conversion(mut self, converter: AudioConverter) -> Self {
        self.converter = Some(converter);
        self
    }

    /// Check if audio conversion is available.
    pub fn has_audio_conversion(&self) -> bool {
        self.converter.is_some()
    }

    /// Prepare audio data for transcription, converting if necessary.
    async fn prepare_audio(
        &self,
        audio_data: Vec<u8>,
        mime_type: &str,
    ) -> anyhow::Result<(Vec<u8>, String)> {
        let format = AudioFormat::from_mime(mime_type);

        // If the format is supported natively, return as-is
        if format.is_supported_by_deepgram() {
            debug!(
                format = ?format,
                mime_type = %mime_type,
                "Audio format supported natively by Deepgram"
            );
            return Ok((audio_data, mime_type.to_string()));
        }

        // If the format needs conversion but we don't have a converter, warn and try anyway
        if format.needs_conversion_for_deepgram() {
            if let Some(ref converter) = self.converter {
                debug!(
                    format = ?format,
                    mime_type = %mime_type,
                    "Converting audio for Deepgram"
                );

                // Check if FFmpeg is available before attempting conversion
                if let Err(e) = converter.check_ffmpeg().await {
                    warn!(
                        error = %e,
                        "FFmpeg not available for audio conversion. \
                         Sending original data to Deepgram, which may fail."
                    );
                    return Ok((audio_data, mime_type.to_string()));
                }

                return converter
                    .convert_for_deepgram_if_needed(&audio_data, mime_type)
                    .await;
            } else {
                warn!(
                    format = ?format,
                    mime_type = %mime_type,
                    "Audio format may not be supported by Deepgram and no converter configured. \
                     Sending original data, which may fail."
                );
            }
        }

        // Unknown format - try sending as-is
        Ok((audio_data, mime_type.to_string()))
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
            has_converter = self.converter.is_some(),
            "Transcribing audio via Deepgram"
        );

        // Prepare audio data (convert if needed)
        let original_size = request.audio_data.len();
        let (audio_data, mime_type) = self
            .prepare_audio(request.audio_data, &request.mime_type)
            .await
            .context("Failed to prepare audio for transcription")?;

        debug!(
            original_mime = %request.mime_type,
            final_mime = %mime_type,
            original_size = original_size,
            final_size = audio_data.len(),
            "Audio prepared for Deepgram"
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
            .header("Content-Type", &mime_type)
            .body(audio_data)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepgram_provider_creation() {
        let provider = DeepgramProvider::new("test-api-key").unwrap();
        assert_eq!(provider.name(), "deepgram");
        assert!(!provider.has_audio_conversion());
    }

    #[test]
    fn test_deepgram_provider_with_conversion() {
        let converter = AudioConverter::new();
        let provider = DeepgramProvider::new("test-api-key")
            .unwrap()
            .with_audio_conversion(converter);
        assert!(provider.has_audio_conversion());
    }

    #[tokio::test]
    async fn test_prepare_audio_supported_format() {
        let provider = DeepgramProvider::new("test-api-key").unwrap();
        let test_data = vec![1, 2, 3, 4, 5];

        // WAV is supported natively - should pass through unchanged
        let (data, mime) = provider
            .prepare_audio(test_data.clone(), "audio/wav")
            .await
            .unwrap();

        assert_eq!(data, test_data);
        assert_eq!(mime, "audio/wav");
    }

    #[tokio::test]
    async fn test_prepare_audio_mp3_format() {
        let provider = DeepgramProvider::new("test-api-key").unwrap();
        let test_data = vec![1, 2, 3, 4, 5];

        // MP3 is supported natively - should pass through unchanged
        let (data, mime) = provider
            .prepare_audio(test_data.clone(), "audio/mpeg")
            .await
            .unwrap();

        assert_eq!(data, test_data);
        assert_eq!(mime, "audio/mpeg");
    }

    #[tokio::test]
    async fn test_prepare_audio_m4a_without_converter() {
        let provider = DeepgramProvider::new("test-api-key").unwrap();
        let test_data = vec![1, 2, 3, 4, 5];

        // M4A needs conversion but no converter configured - should pass through
        let (data, mime) = provider
            .prepare_audio(test_data.clone(), "audio/m4a")
            .await
            .unwrap();

        assert_eq!(data, test_data);
        assert_eq!(mime, "audio/m4a");
    }
}
