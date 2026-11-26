//! Coverage for `is_audio_mime`, `build_tts_provider`, `build_provider`.
//!
//! These factories were 0% covered in workspace coverage. Lifting them
//! gets `crates/transcription` past the 80% floor.

use std::collections::HashMap;

use assistant_core::Secret;
use assistant_core::types::llm::{LlmConfig, LlmProviderKind};
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
fn is_audio_mime_accepts_voice_note_and_video_containers() {
    for mime in [
        "audio/amr",
        "audio/amr-wb",
        "audio/3gpp",
        "audio/3gpp2",
        "audio/x-caf",
        "audio/aiff",
        "audio/x-aiff",
        "audio/wave",
        "video/mp4",
        "video/webm",
        "video/quicktime",
    ] {
        assert!(is_audio_mime(mime), "{mime}");
    }
}

#[test]
fn is_audio_mime_ignores_parameters_and_case() {
    assert!(is_audio_mime("audio/ogg; codecs=opus"));
    assert!(is_audio_mime("Audio/OGG"));
    assert!(!is_audio_mime("video/x-matroska"));
    assert!(!is_audio_mime("audio/mpegurl"), "a playlist is not audio");
}

#[test]
fn is_audio_mime_rejects_non_audio() {
    assert!(!is_audio_mime("text/plain"));
    assert!(!is_audio_mime("image/png"));
    assert!(!is_audio_mime("application/pdf"));
    assert!(!is_audio_mime(""));
}

// ── build_tts_provider ─────────────────────────────────────────────────────

#[tokio::test]
async fn build_tts_openai_uses_api_key_from_config() {
    let config = TtsConfig {
        provider: TtsProviderKind::OpenAI,
        api_key: Some("sk-test".into()),
        base_url: Some("https://example.com".to_string()),
        model: Some("tts-1".to_string()),
        voice: Some("alloy".to_string()),
        voices: HashMap::new(),
        format: None,
        speed: None,
        instructions: None,
        pcm_sample_rate: None,
    };
    let provider = build_tts_provider(&config, &LlmConfig::default())
        .await
        .ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

// NOTE: A "missing api key returns error" path exists for OpenAI/Deepgram
// but is environment-dependent (the function falls back to OPENAI_API_KEY /
// DEEPGRAM_API_KEY env vars). Testing those requires clearing the env
// in a single-threaded section, which is racy across `cargo test`'s
// parallel runner. The happy-path test above exercises the bulk of the
// branch; the env-fallback line is intentionally not asserted here.

#[tokio::test]
async fn build_tts_deepgram_uses_api_key_from_config() {
    let mut voices = HashMap::new();
    voices.insert("en".to_string(), "aura-asteria-en".to_string());
    let config = TtsConfig {
        provider: TtsProviderKind::Deepgram,
        api_key: Some("dg-test".into()),
        base_url: None,
        model: Some("aura-asteria-en".to_string()),
        voice: None,
        voices,
        format: None,
        speed: None,
        instructions: None,
        pcm_sample_rate: None,
    };
    let provider = build_tts_provider(&config, &LlmConfig::default())
        .await
        .ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

// Deepgram env-fallback path intentionally not asserted (same env-race
// constraint as the OpenAI variant above).

fn tts(provider: TtsProviderKind) -> TtsConfig {
    TtsConfig {
        provider,
        api_key: None,
        base_url: None,
        model: None,
        voice: None,
        voices: HashMap::new(),
        format: None,
        speed: None,
        instructions: None,
        pcm_sample_rate: None,
    }
}

#[tokio::test]
async fn build_tts_openrouter_reuses_the_llm_key_and_reads_the_catalog() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/audio/speech/models/openai/gpt-4o-mini-tts"))
        .and(header("authorization", "Bearer sk-or-chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "output_modalities": [{"type": "speech", "supported_parameters": {
                "response_format": {"type": "enum", "values": ["mp3"]},
                "voice": {"type": "enum", "values": ["alloy"]},
            }}],
        })))
        .expect(1)
        .mount(&server)
        .await;
    let config = TtsConfig {
        base_url: Some(server.uri()),
        ..tts(TtsProviderKind::OpenRouter)
    };
    let llm = LlmConfig {
        provider: LlmProviderKind::OpenRouter,
        api_key: Some(Secret::new("sk-or-chat")),
        ..LlmConfig::default()
    };

    let provider = build_tts_provider(&config, &llm).await.unwrap();
    assert_eq!(provider.name(), "openrouter");
}

#[tokio::test]
async fn build_tts_openrouter_without_a_key_names_openrouter() {
    let _env = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: ENV_LOCK keeps the other tests that touch this variable out;
    // no other test in this binary reads it.
    let prev = std::env::var("OPENROUTER_API_KEY").ok();
    unsafe { std::env::remove_var("OPENROUTER_API_KEY") };
    let err = build_tts_provider(&tts(TtsProviderKind::OpenRouter), &LlmConfig::default())
        .await
        .err()
        .expect("must error");
    assert!(err.to_string().contains("OpenRouter"), "{err}");
    if let Some(v) = prev {
        unsafe { std::env::set_var("OPENROUTER_API_KEY", v) };
    }
}

#[tokio::test]
async fn build_tts_rejects_pcm_as_the_configured_format() {
    let config = TtsConfig {
        api_key: Some("sk-test".into()),
        format: Some(assistant_core::types::transcription::TtsFormat::Pcm),
        ..tts(TtsProviderKind::OpenAI)
    };
    let err = build_tts_provider(&config, &LlmConfig::default())
        .await
        .err()
        .expect("must error");
    assert!(err.to_string().contains("pcm"), "{err}");
}

#[tokio::test]
async fn build_tts_deepgram_rejects_formats_other_than_mp3() {
    let config = TtsConfig {
        api_key: Some("dg-test".into()),
        format: Some(assistant_core::types::transcription::TtsFormat::Ogg),
        ..tts(TtsProviderKind::Deepgram)
    };
    let err = build_tts_provider(&config, &LlmConfig::default())
        .await
        .err()
        .expect("must error");
    assert!(err.to_string().contains("format"), "{err}");
}

// ── build_provider (transcription) ─────────────────────────────────────────

#[tokio::test]
async fn build_transcription_whisper_uses_api_key_from_config() {
    let config = TranscriptionConfig {
        provider: TranscriptionProviderKind::Whisper,
        api_key: Some("sk-test".into()),
        base_url: None,
        model: None,
        language: None,
        prompt: None,
        max_chunk_secs: None,
    };
    let provider = build_provider(&config, &LlmConfig::default()).await.ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

// Whisper env-fallback path intentionally not asserted.

#[tokio::test]
async fn build_transcription_ollama_constructs_without_api_key() {
    let config = TranscriptionConfig {
        provider: TranscriptionProviderKind::Ollama,
        api_key: None,
        base_url: Some("http://localhost:11434".into()),
        model: Some("whisper".into()),
        language: None,
        prompt: None,
        max_chunk_secs: None,
    };
    let provider = build_provider(&config, &LlmConfig::default()).await.ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

#[tokio::test]
async fn build_transcription_deepgram_uses_api_key_from_config() {
    let config = TranscriptionConfig {
        provider: TranscriptionProviderKind::Deepgram,
        api_key: Some("dg-test".into()),
        base_url: None,
        model: None,
        language: None,
        prompt: None,
        max_chunk_secs: None,
    };
    let provider = build_provider(&config, &LlmConfig::default()).await.ok();
    assert!(provider.is_some());
    assert!(!provider.unwrap().name().is_empty());
}

// Deepgram transcription env-fallback path intentionally not asserted.

/// Serializes the tests that change `OPENROUTER_API_KEY`, which share this
/// process's environment.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn openrouter_transcription(api_key: Option<&str>) -> TranscriptionConfig {
    TranscriptionConfig {
        provider: TranscriptionProviderKind::OpenRouter,
        api_key: api_key.map(Into::into),
        base_url: None,
        model: None,
        language: Some("de".into()),
        prompt: None,
        max_chunk_secs: Some(120),
    }
}

#[tokio::test]
async fn build_transcription_openrouter_reuses_the_llm_key() {
    let llm = LlmConfig {
        provider: LlmProviderKind::OpenRouter,
        api_key: Some(Secret::new("sk-or-chat")),
        ..LlmConfig::default()
    };
    let provider = build_provider(&openrouter_transcription(None), &llm)
        .await
        .unwrap();
    assert_eq!(provider.name(), "openrouter");
    assert!(
        provider.accepted_formats().is_some(),
        "wrapped so other formats are converted"
    );
}

#[tokio::test]
async fn build_transcription_openrouter_uses_its_own_key() {
    let llm = LlmConfig {
        provider: LlmProviderKind::Anthropic,
        ..LlmConfig::default()
    };
    build_provider(&openrouter_transcription(Some("sk-or-stt")), &llm)
        .await
        .unwrap();
}

#[tokio::test]
async fn build_transcription_openrouter_without_a_key_names_openrouter() {
    let _env = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: ENV_LOCK keeps the other tests that touch this variable out;
    // no other test in this binary reads it.
    let prev = std::env::var("OPENROUTER_API_KEY").ok();
    unsafe { std::env::remove_var("OPENROUTER_API_KEY") };
    let llm = LlmConfig {
        provider: LlmProviderKind::Anthropic,
        ..LlmConfig::default()
    };
    let err = build_provider(&openrouter_transcription(None), &llm)
        .await
        .err()
        .expect("must error");
    assert!(err.to_string().contains("OpenRouter"), "{err}");
    if let Some(v) = prev {
        unsafe { std::env::set_var("OPENROUTER_API_KEY", v) };
    }
}
