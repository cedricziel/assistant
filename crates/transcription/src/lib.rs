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

pub mod audio_store;
mod converter;
mod converting;
mod deepgram;
mod deepgram_tts;
mod ollama;
mod openai_tts;
pub mod pipeline;
mod provider;
mod whisper;

use std::sync::Arc;

use assistant_core::types::transcription::{
    TranscriptionConfig, TranscriptionProviderKind, TtsConfig, TtsProviderKind,
};

pub use audio_store::AudioStore;
pub use converter::AudioFormat;
pub use converting::ConvertingTranscriber;
pub use deepgram::DeepgramProvider;
pub use deepgram_tts::DeepgramTtsProvider;
pub use ollama::OllamaTranscriptionProvider;
pub use openai_tts::OpenAITtsProvider;
pub use provider::{
    TranscriptionProvider, TranscriptionRequest, TranscriptionResult, TtsProvider, TtsRequest,
    TtsResult,
};
pub use whisper::WhisperProvider;

/// MIME types transcribed as audio. Video containers are included because
/// only their audio stream is used.
pub const AUDIO_MIME_TYPES: &[&str] = &[
    "audio/ogg",
    "audio/opus",
    "audio/mpeg",
    "audio/mp3",
    "audio/mp4",
    "audio/m4a",
    "audio/x-m4a",
    "audio/wav",
    "audio/x-wav",
    "audio/wave",
    "audio/flac",
    "audio/x-flac",
    "audio/webm",
    "audio/aac",
    "audio/amr",
    "audio/amr-wb",
    "audio/3gpp",
    "audio/3gpp2",
    "audio/x-caf",
    "audio/aiff",
    "audio/x-aiff",
    "video/mp4",
    "video/webm",
    "video/quicktime",
];

/// `mime` lowercased, without parameters such as `; codecs=opus`.
pub fn essence(mime: &str) -> String {
    mime.split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase()
}

/// Returns `true` if `mime` is an audio (or video) type to transcribe.
pub fn is_audio_mime(mime: &str) -> bool {
    AUDIO_MIME_TYPES.contains(&essence(mime).as_str())
}

/// Build a [`TtsProvider`] from the assistant TTS configuration.
pub fn build_tts_provider(config: &TtsConfig) -> anyhow::Result<Arc<dyn TtsProvider>> {
    match config.provider {
        TtsProviderKind::OpenAI => {
            let api_key = config
                .api_key
                .as_ref()
                .map(|key| key.expose().to_owned())
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "OpenAI TTS requires an API key. \
                         Set api_key in [tts] or OPENAI_API_KEY env var."
                    )
                })?;

            let mut provider = OpenAITtsProvider::new(api_key)?;
            if let Some(ref url) = config.base_url {
                provider = provider.with_base_url(url);
            }
            if let Some(ref model) = config.model {
                provider = provider.with_model(model);
            }
            if let Some(ref voice) = config.voice {
                provider = provider.with_voice(voice);
            }
            Ok(Arc::new(provider))
        }
        TtsProviderKind::Deepgram => {
            let api_key = config
                .api_key
                .as_ref()
                .map(|key| key.expose().to_owned())
                .or_else(|| std::env::var("DEEPGRAM_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Deepgram TTS requires an API key. \
                         Set api_key in [tts] or DEEPGRAM_API_KEY env var."
                    )
                })?;

            let mut provider = DeepgramTtsProvider::new(api_key)?;
            if let Some(ref url) = config.base_url {
                provider = provider.with_base_url(url);
            }
            if let Some(ref model) = config.model {
                provider = provider.with_model(model);
            }
            if !config.voices.is_empty() {
                provider = provider.with_voices(config.voices.clone());
            }
            Ok(Arc::new(provider))
        }
    }
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
                .as_ref()
                .map(|key| key.expose().to_owned())
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
            Ok(Arc::new(ConvertingTranscriber::new(provider)))
        }
        TranscriptionProviderKind::Ollama => {
            let mut provider = OllamaTranscriptionProvider::new()?;
            if let Some(ref url) = config.base_url {
                provider = provider.with_base_url(url);
            }
            if let Some(ref model) = config.model {
                provider = provider.with_model(model);
            }
            Ok(Arc::new(ConvertingTranscriber::new(provider)))
        }
        TranscriptionProviderKind::Deepgram => {
            let api_key = config
                .api_key
                .as_ref()
                .map(|key| key.expose().to_owned())
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
            Ok(Arc::new(ConvertingTranscriber::new(provider)))
        }
    }
}
