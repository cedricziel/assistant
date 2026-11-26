//! Configuration for speech-to-text (`Transcription*`) and text-to-speech
//! (`Tts*`) providers.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::secret::Secret;

// ── Speech-to-text ────────────────────────────────────────────────────────────

/// Which transcription backend to use.
///
/// Set via `[transcription] provider = "whisper"` in `org.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionProviderKind {
    /// OpenAI Whisper API (or any compatible endpoint).
    Whisper,
    /// Local Ollama server running a whisper-compatible model.
    Ollama,
    /// Deepgram hosted speech-to-text API.
    Deepgram,
}

/// Configuration for audio transcription.
///
/// When present, interfaces that receive audio attachments (Slack voice
/// messages, Signal voice notes, …) will automatically transcribe them
/// and inject the transcript into the conversation as text.
///
/// ```toml
/// [transcription]
/// provider = "whisper"
/// # model = "whisper-1"
/// # api_key = "sk-..."  # or set OPENAI_API_KEY env var
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Which transcription backend to use.
    pub provider: TranscriptionProviderKind,
    /// Model name (uses provider-specific default if omitted).
    pub model: Option<String>,
    /// Base URL override (uses provider-specific default if omitted).
    pub base_url: Option<String>,
    /// API key (also checked via provider-specific env vars:
    /// `OPENAI_API_KEY` for Whisper, `DEEPGRAM_API_KEY` for Deepgram).
    pub api_key: Option<Secret>,
    /// Optional BCP-47 language hint (e.g. `"en"`, `"de"`).
    pub language: Option<String>,
    /// Vocabulary or style hint sent with each request (OpenRouter).
    #[serde(default)]
    pub prompt: Option<String>,
    /// Longest chunk sent in one request, in seconds (OpenRouter; default
    /// 50, at most 55 because upstreams time out after 60 seconds).
    #[serde(default)]
    pub max_chunk_secs: Option<u32>,
}

/// Default for [`TranscriptionConfig::max_chunk_secs`].
pub const DEFAULT_MAX_CHUNK_SECS: u32 = 50;
/// Upper bound for [`TranscriptionConfig::max_chunk_secs`].
pub const MAX_CHUNK_SECS_LIMIT: u32 = 55;

impl TranscriptionConfig {
    /// `max_chunk_secs`, defaulted and capped at [`MAX_CHUNK_SECS_LIMIT`].
    pub fn effective_max_chunk_secs(&self) -> u32 {
        self.max_chunk_secs
            .unwrap_or(DEFAULT_MAX_CHUNK_SECS)
            .clamp(1, MAX_CHUNK_SECS_LIMIT)
    }
}

// ── Text-to-speech ────────────────────────────────────────────────────────────

/// Which TTS backend to use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TtsProviderKind {
    /// OpenAI TTS API (`POST /v1/audio/speech`).
    OpenAI,
    /// Deepgram TTS API (`POST /v1/speak`).
    Deepgram,
}

/// Configuration for text-to-speech synthesis.
///
/// When present, the web UI will offer voice playback on assistant messages.
///
/// ```toml
/// [tts]
/// provider = "openai"
/// voice = "nova"
/// # api_key = "sk-..."  # or set OPENAI_API_KEY env var
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// Which TTS backend to use.
    pub provider: TtsProviderKind,
    /// Model name (provider-specific default if omitted; e.g. `"tts-1"` for OpenAI).
    pub model: Option<String>,
    /// Voice name (provider-specific; e.g. `"nova"`, `"alloy"` for OpenAI).
    pub voice: Option<String>,
    /// Base URL override (uses provider-specific default if omitted).
    pub base_url: Option<String>,
    /// API key (also checked via provider-specific env vars: `OPENAI_API_KEY` for OpenAI,
    /// `DEEPGRAM_API_KEY` for Deepgram).
    pub api_key: Option<Secret>,
    /// Per-language voice overrides (Deepgram only).
    ///
    /// ```toml
    /// [tts.voices]
    /// en = "aura-2-zeus-en"
    /// de = "aura-2-aurelia-de"
    /// ```
    #[serde(default)]
    pub voices: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::{TranscriptionConfig, TranscriptionProviderKind};

    fn with_chunk(max_chunk_secs: Option<u32>) -> TranscriptionConfig {
        TranscriptionConfig {
            provider: TranscriptionProviderKind::Whisper,
            model: None,
            base_url: None,
            api_key: None,
            language: None,
            prompt: None,
            max_chunk_secs,
        }
    }

    #[test]
    fn max_chunk_secs_defaults_to_50_and_is_capped_at_55() {
        assert_eq!(with_chunk(None).effective_max_chunk_secs(), 50);
        assert_eq!(with_chunk(Some(30)).effective_max_chunk_secs(), 30);
        assert_eq!(with_chunk(Some(120)).effective_max_chunk_secs(), 55);
    }
}
