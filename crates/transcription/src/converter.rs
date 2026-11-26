//! Formats the OpenAI-compatible transcription APIs accept.

use assistant_audio::AudioFormat;

/// Formats OpenAI's `/audio/transcriptions` API accepts, which Whisper and
/// Ollama servers share.
pub const OPENAI_COMPATIBLE_FORMATS: &[AudioFormat] = &[
    AudioFormat::Flac,
    AudioFormat::M4a,
    AudioFormat::Mp3,
    AudioFormat::Mp4,
    AudioFormat::Ogg,
    AudioFormat::Wav,
    AudioFormat::Webm,
];
