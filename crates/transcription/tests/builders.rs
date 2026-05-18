//! Coverage for `is_audio_mime`, `build_tts_provider`, `build_provider`.
//!
//! These factories were 0% covered in workspace coverage. Lifting them
//! gets `crates/transcription` past the 80% floor.

use std::collections::HashMap;

use assistant_core::types::transcription::{
    TranscriptionConfig, TranscriptionProviderKind, TtsConfig, TtsProviderKind,
};
use assistant_transcription::{build_provider, build_tts_provider, is_audio_mime};

#[test]
fn is_audio_mime_accepts_common_audio_types() {
    assert!(is_audio_mime("audio/ogg"));
    assert!(is_audio_mime("audio/mp4"));
    assert!(is_audio_mime("audio/mpeg"));
    assert!(is_audio_mime("audio/wav"));
    assert!(is_audio_mime("audio/webm"));
}

#[test]
fn is_audio_mime_rejects_non_audio() {
    assert!(!is_audio_mime("text/plain"));
    assert!(!is_audio_mime("image/png"));
    assert!(!is_audio_mime("application/pdf"));
    assert!(!is_audio_mime(""));
}

// ── build_tts_provider ─────────────────────────────────────────────────────

#[test]
fn build_tts_openai_uses_api_key_from_config() {
    let config = TtsConfig {
        provider: TtsProviderKind::OpenAI,
        api_key: Some("sk-test".to_string()),
        base_url: Some("https://example.com".to_string()),
        model: Some("tts-1".to_string()),
        voice: Some("alloy".to_string()),
        voices: HashMap::new(),
    };
    let provider = build_tts_provider(&config).ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

// NOTE: A "missing api key returns error" path exists for OpenAI/Deepgram
// but is environment-dependent (the function falls back to OPENAI_API_KEY /
// DEEPGRAM_API_KEY env vars). Testing those requires clearing the env
// in a single-threaded section, which is racy across `cargo test`'s
// parallel runner. The happy-path test above exercises the bulk of the
// branch; the env-fallback line is intentionally not asserted here.

#[test]
fn build_tts_deepgram_uses_api_key_from_config() {
    let mut voices = HashMap::new();
    voices.insert("en".to_string(), "aura-asteria-en".to_string());
    let config = TtsConfig {
        provider: TtsProviderKind::Deepgram,
        api_key: Some("dg-test".to_string()),
        base_url: None,
        model: Some("aura-asteria-en".to_string()),
        voice: None,
        voices,
    };
    let provider = build_tts_provider(&config).ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

// Deepgram env-fallback path intentionally not asserted (same env-race
// constraint as the OpenAI variant above).

// ── build_provider (transcription) ─────────────────────────────────────────

#[test]
fn build_transcription_whisper_uses_api_key_from_config() {
    let config = TranscriptionConfig {
        provider: TranscriptionProviderKind::Whisper,
        api_key: Some("sk-test".to_string()),
        base_url: None,
        model: None,
        language: None,
    };
    let provider = build_provider(&config).ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

// Whisper env-fallback path intentionally not asserted.

#[test]
fn build_transcription_ollama_constructs_without_api_key() {
    let config = TranscriptionConfig {
        provider: TranscriptionProviderKind::Ollama,
        api_key: None,
        base_url: Some("http://localhost:11434".into()),
        model: Some("whisper".into()),
        language: None,
    };
    let provider = build_provider(&config).ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

#[test]
fn build_transcription_deepgram_uses_api_key_from_config() {
    let config = TranscriptionConfig {
        provider: TranscriptionProviderKind::Deepgram,
        api_key: Some("dg-test".to_string()),
        base_url: None,
        model: None,
        language: None,
    };
    let provider = build_provider(&config).ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

// Deepgram transcription env-fallback path intentionally not asserted.
