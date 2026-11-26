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
mod openrouter;
pub mod openrouter_tts;
pub mod pipeline;
mod provider;
mod whisper;

use std::sync::Arc;

use assistant_core::types::llm::{LlmConfig, OPENROUTER_API_KEY_ENV, resolve_openrouter_key};
use assistant_core::types::transcription::{
    TranscriptionConfig, TranscriptionProviderKind, TtsConfig, TtsFormat, TtsProviderKind,
};

pub use audio_store::AudioStore;
pub use converter::AudioFormat;
pub use converting::ConvertingTranscriber;
pub use deepgram::DeepgramProvider;
pub use deepgram_tts::DeepgramTtsProvider;
pub use ollama::OllamaTranscriptionProvider;
pub use openai_tts::OpenAITtsProvider;
pub use openrouter::OpenRouterTranscriber;
pub use openrouter_tts::{OpenRouterTts, OpenRouterTtsSettings};
use pipeline::AudioPipeline;
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

/// Build a [`TtsProvider`] from `[tts]`. `llm` supplies the OpenRouter key
/// when `[tts]` has none. The OpenRouter provider reads and validates the
/// model's speech-catalog entry, so building it is async.
pub async fn build_tts_provider(
    config: &TtsConfig,
    llm: &LlmConfig,
) -> anyhow::Result<Arc<dyn TtsProvider>> {
    match config.provider {
        TtsProviderKind::OpenRouter => {
            let api_key = resolve_openrouter_key(
                config.api_key.as_ref(),
                llm,
                OPENROUTER_API_KEY_ENV,
                |name| std::env::var(name).ok(),
            )
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "OpenRouter TTS requires an API key. Set api_key in [tts] or [llm], \
                     or the OPENROUTER_API_KEY env var."
                )
            })?;
            let settings = OpenRouterTtsSettings {
                base_url: config.base_url.clone().unwrap_or_else(|| {
                    assistant_llm_provider::openrouter::DEFAULT_BASE_URL.to_string()
                }),
                api_key,
                model: config
                    .model
                    .clone()
                    .unwrap_or_else(|| openrouter_tts::DEFAULT_MODEL.to_string()),
                voice: config
                    .voice
                    .clone()
                    .unwrap_or_else(|| openrouter_tts::DEFAULT_VOICE.to_string()),
                format: config.format.unwrap_or_default(),
                speed: config.speed,
                instructions: config.instructions.clone(),
                pcm_sample_rate: config.pcm_sample_rate,
            };
            let provider = OpenRouterTts::connect(settings, AudioPipeline::default()).await?;
            Ok(Arc::new(provider))
        }
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
            if let Some(format) = config.format {
                provider = provider.with_format(format);
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

            if let Some(format) = config.format.filter(|f| *f != TtsFormat::Mp3) {
                anyhow::bail!(
                    "Deepgram TTS delivers mp3 only; remove [tts] format = \"{}\"",
                    format.as_str()
                );
            }
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

/// Build a [`TranscriptionProvider`] from `[transcription]`. `llm` supplies
/// the OpenRouter key when `[transcription]` has none. Every provider is
/// wrapped in a [`ConvertingTranscriber`]. Fails when the configuration is
/// invalid, e.g. a missing API key.
pub async fn build_provider(
    config: &TranscriptionConfig,
    llm: &LlmConfig,
) -> anyhow::Result<Arc<dyn TranscriptionProvider>> {
    match config.provider {
        TranscriptionProviderKind::OpenRouter => {
            let api_key = resolve_openrouter_key(
                config.api_key.as_ref(),
                llm,
                OPENROUTER_API_KEY_ENV,
                |name| std::env::var(name).ok(),
            )
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "OpenRouter transcription requires an API key. Set api_key in \
                     [transcription] or [llm], or the OPENROUTER_API_KEY env var."
                )
            })?;
            let base_url = config.base_url.clone().unwrap_or_else(|| {
                assistant_llm_provider::openrouter::DEFAULT_BASE_URL.to_string()
            });
            let mut provider =
                OpenRouterTranscriber::new(base_url, api_key, config.effective_max_chunk_secs())?;
            if let Some(ref model) = config.model {
                provider = provider.with_model(model);
            }
            if let Some(ref language) = config.language {
                provider = provider.with_language(language);
            }
            if let Some(ref prompt) = config.prompt {
                provider = provider.with_prompt(prompt);
            }
            Ok(Arc::new(ConvertingTranscriber::new(provider)))
        }
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
