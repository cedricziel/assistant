//! Pluggable audio transcription for the assistant.
//!
//! This crate defines a [`TranscriptionProvider`] trait and ships three
//! implementations:
//!
//! * **Whisper** — OpenAI's hosted Whisper API.
//! * **Ollama** — local Ollama server running a whisper-compatible model.
//! * **Deepgram** — Deepgram's hosted speech-to-text API.
//!
//! Interfaces (Slack, Signal, …) download an audio attachment, pass the raw
//! bytes to the configured provider, and inject the resulting transcript into
//! the normal text message flow.

mod deepgram;
mod ollama;
mod provider;
mod whisper;

use std::sync::Arc;

use assistant_core::{TranscriptionConfig, TranscriptionProviderKind};

pub use deepgram::DeepgramProvider;
pub use ollama::OllamaTranscriptionProvider;
pub use provider::{TranscriptionProvider, TranscriptionRequest, TranscriptionResult};
pub use whisper::WhisperProvider;

/// MIME type prefixes recognised as audio attachments.
pub const AUDIO_MIME_PREFIXES: &[&str] = &[
    "audio/ogg",
    "audio/opus",
    "audio/mpeg",
    "audio/mp3",
    "audio/mp4",
    "audio/m4a",
    "audio/x-m4a",
    "audio/wav",
    "audio/x-wav",
    "audio/flac",
    "audio/x-flac",
    "audio/webm",
    "audio/aac",
];

/// Returns `true` if the given MIME type is a recognised audio format.
pub fn is_audio_mime(mime: &str) -> bool {
    AUDIO_MIME_PREFIXES
        .iter()
        .any(|prefix| mime.starts_with(prefix))
}

/// Build a [`TranscriptionProvider`] from the assistant configuration.
///
/// Returns `None` when `config` is `None` (transcription not configured).
/// Returns an error if the configuration is invalid (e.g. missing API key).
pub fn build_provider(
    config: &TranscriptionConfig,
) -> anyhow::Result<Arc<dyn TranscriptionProvider>> {
    match config.provider {
        TranscriptionProviderKind::Whisper => {
            let api_key = config
                .api_key
                .clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Whisper transcription requires an API key. \
                         Set api_key in [transcription] or OPENAI_API_KEY env var."
                    )
                })?;

            let mut provider = WhisperProvider::new(api_key)?;
            if let Some(ref url) = config.base_url {
                provider = provider.with_base_url(url);
            }
            if let Some(ref model) = config.model {
                provider = provider.with_model(model);
            }
            Ok(Arc::new(provider))
        }
        TranscriptionProviderKind::Ollama => {
            let mut provider = OllamaTranscriptionProvider::new()?;
            if let Some(ref url) = config.base_url {
                provider = provider.with_base_url(url);
            }
            if let Some(ref model) = config.model {
                provider = provider.with_model(model);
            }
            Ok(Arc::new(provider))
        }
        TranscriptionProviderKind::Deepgram => {
            let api_key = config
                .api_key
                .clone()
                .or_else(|| std::env::var("DEEPGRAM_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Deepgram transcription requires an API key. \
                         Set api_key in [transcription] or DEEPGRAM_API_KEY env var."
                    )
                })?;

            let mut provider = DeepgramProvider::new(api_key)?;
            if let Some(ref url) = config.base_url {
                provider = provider.with_base_url(url);
            }
            if let Some(ref model) = config.model {
                provider = provider.with_model(model);
            }
            Ok(Arc::new(provider))
        }
    }
}
