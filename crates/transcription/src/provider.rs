//! Core trait and types for audio transcription.

use async_trait::async_trait;

/// A request to transcribe audio data.
#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    /// Raw audio bytes (e.g. Ogg/Opus, MP3, WAV, M4A, FLAC, WebM).
    pub audio_data: Vec<u8>,
    /// MIME type of the audio (e.g. `"audio/ogg"`, `"audio/mp4"`).
    pub mime_type: String,
    /// Original filename, if available.  Some providers use the extension to
    /// detect the codec when MIME alone is ambiguous.
    pub filename: Option<String>,
    /// Optional BCP-47 language hint (e.g. `"en"`, `"de"`).
    /// Providers that support it will bias recognition towards this language.
    pub language: Option<String>,
}

/// The result of a transcription.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// The transcribed text.
    pub text: String,
    /// Detected or confirmed language (BCP-47), if the provider reports it.
    pub language: Option<String>,
    /// Transcription duration in seconds, if reported by the provider.
    pub duration_secs: Option<f64>,
}

/// Guess a reasonable filename extension from a MIME type so transcription
/// APIs can detect the codec when the original filename is unavailable.
pub(crate) fn extension_for_mime(mime: &str) -> &str {
    match mime {
        "audio/ogg" | "audio/opus" => "ogg",
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/mp4" | "audio/m4a" | "audio/x-m4a" => "m4a",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/webm" => "webm",
        "audio/aac" => "aac",
        _ => "bin",
    }
}

/// Pluggable backend for converting audio to text.
///
/// Follows the same `Arc<dyn Trait>` pattern as [`LlmProvider`] so providers
/// can be swapped via configuration without touching the interfaces.
#[async_trait]
pub trait TranscriptionProvider: Send + Sync {
    /// Human-readable provider name (e.g. `"whisper"`, `"ollama"`, `"deepgram"`).
    fn name(&self) -> &str;

    /// Transcribe audio bytes into text.
    async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> anyhow::Result<TranscriptionResult>;
}
