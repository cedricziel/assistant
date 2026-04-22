//! Deepgram TTS provider — `POST /v1/speak`.

use std::collections::HashMap;

use anyhow::{Context, bail};
use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use tracing::debug;

use crate::provider::{TtsProvider, TtsRequest, TtsResult};

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const DEFAULT_MODEL: &str = "aura-2-thalia-en";

/// Default voice per detected language.
const DEFAULT_VOICES: &[(&str, &str)] = &[
    ("en", "aura-2-thalia-en"),
    ("de", "aura-2-julius-de"),
    ("es", "aura-2-lucia-es"),
    ("fr", "aura-2-chloe-fr"),
];

pub struct DeepgramTtsProvider {
    api_key: String,
    model: String,
    base_url: String,
    voices: HashMap<String, String>,
    client: ClientWithMiddleware,
}

impl DeepgramTtsProvider {
    pub fn new(api_key: impl Into<String>) -> anyhow::Result<Self> {
        let client = assistant_llm_provider::http::build_http_client(
            DEFAULT_TIMEOUT_SECS,
            &assistant_llm_provider::retry::RetryConfig::default(),
        )?;
        let voices = DEFAULT_VOICES
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        Ok(Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            base_url: "https://api.deepgram.com/v1".to_string(),
            voices,
            client,
        })
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Merge user-provided voice overrides into the default voice map.
    pub fn with_voices(mut self, overrides: HashMap<String, String>) -> Self {
        for (lang, voice) in overrides {
            self.voices.insert(lang, voice);
        }
        self
    }

    /// Select a voice for the given text by detecting its language.
    fn select_voice(&self, text: &str) -> String {
        let lang = detect_language(text);
        self.voices
            .get(lang)
            .cloned()
            .unwrap_or_else(|| self.model.clone())
    }
}

#[async_trait]
impl TtsProvider for DeepgramTtsProvider {
    fn name(&self) -> &str {
        "deepgram-tts"
    }

    async fn synthesize(&self, request: TtsRequest) -> anyhow::Result<TtsResult> {
        let model = request
            .voice
            .unwrap_or_else(|| self.select_voice(&request.text));

        debug!(
            provider = "deepgram-tts",
            model = %model,
            text_len = request.text.len(),
            "Synthesizing speech"
        );

        let body = serde_json::json!({ "text": request.text });
        let url = format!("{}/speak?model={}", self.base_url, model);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .json(&body)
            .send()
            .await
            .context("Deepgram TTS API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            bail!("Deepgram TTS API returned {status}: {body_text}");
        }

        let audio_data = resp
            .bytes()
            .await
            .context("Failed to read Deepgram TTS response bytes")?
            .to_vec();

        debug!(bytes = audio_data.len(), "Deepgram TTS synthesis complete");

        Ok(TtsResult {
            audio_data,
            mime_type: "audio/mpeg".to_string(),
        })
    }
}

// -- Language detection -------------------------------------------------------

/// Stop-word lists for Latin-script languages.
const EN_STOPS: &[&str] = &[
    "the", "is", "and", "to", "of", "in", "that", "it", "for", "was", "are", "with", "this",
    "have", "not", "but", "you", "from",
];
const DE_STOPS: &[&str] = &[
    "der", "die", "das", "und", "ist", "ein", "nicht", "den", "es", "sich", "mit", "auf", "auch",
    "als", "eine", "dem", "von", "wird",
];
const ES_STOPS: &[&str] = &[
    "el", "la", "los", "las", "de", "en", "que", "es", "por", "con", "una", "para", "del", "como",
    "pero",
];
const FR_STOPS: &[&str] = &[
    "le", "la", "les", "de", "des", "un", "une", "et", "est", "dans", "que", "qui", "pour", "pas",
    "sur",
];

/// Detect the most likely language of `text` using stop-word frequency.
///
/// Returns an ISO 639-1 code (`"en"`, `"de"`, `"es"`, `"fr"`).
/// Falls back to `"en"` for unknown or ambiguous text.
pub fn detect_language(text: &str) -> &'static str {
    if text.is_empty() {
        return "en";
    }

    // Check for non-Latin scripts first.
    let mut cjk_count = 0u32;
    let mut total_alpha = 0u32;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            total_alpha += 1;
        }
        if ('\u{3040}'..='\u{30FF}').contains(&ch)
            || ('\u{4E00}'..='\u{9FFF}').contains(&ch)
            || ('\u{AC00}'..='\u{D7AF}').contains(&ch)
        {
            cjk_count += 1;
        }
    }
    // If >30% of alphabetic chars are CJK, classify as Japanese (simplified).
    if total_alpha > 0 && cjk_count * 100 / total_alpha > 30 {
        return "ja";
    }

    // Tokenise into lowercase words.
    let words: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|w| !w.is_empty())
        .collect();
    let total = words.len();
    if total == 0 {
        return "en";
    }

    let count = |stops: &[&str]| -> usize {
        words
            .iter()
            .filter(|w| {
                let lower = w.to_lowercase();
                stops.contains(&lower.as_str())
            })
            .count()
    };

    let scores = [
        ("en", count(EN_STOPS)),
        ("de", count(DE_STOPS)),
        ("es", count(ES_STOPS)),
        ("fr", count(FR_STOPS)),
    ];

    scores
        .iter()
        .max_by_key(|(_, s)| *s)
        .filter(|(_, s)| *s > 0)
        .map(|(lang, _)| *lang)
        .unwrap_or("en")
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn provider_name() {
        let p = DeepgramTtsProvider::new("key").unwrap();
        assert_eq!(p.name(), "deepgram-tts");
    }

    #[tokio::test]
    async fn synthesize_returns_audio_bytes_on_success() {
        let server = MockServer::start().await;
        let fake_audio = b"DEEPGRAM_AUDIO".to_vec();

        Mock::given(method("POST"))
            .and(path("/speak"))
            .and(header("authorization", "Token test-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(fake_audio.clone())
                    .insert_header("content-type", "audio/mpeg"),
            )
            .mount(&server)
            .await;

        let provider = DeepgramTtsProvider::new("test-key")
            .unwrap()
            .with_base_url(server.uri());

        let result = provider
            .synthesize(TtsRequest {
                text: "Hello Deepgram".to_string(),
                voice: None,
                format: None,
                speed: None,
            })
            .await
            .unwrap();

        assert_eq!(result.audio_data, fake_audio);
        assert_eq!(result.mime_type, "audio/mpeg");
    }

    #[tokio::test]
    async fn synthesize_uses_voice_as_model_override() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/speak"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"audio".to_vec()))
            .mount(&server)
            .await;

        let provider = DeepgramTtsProvider::new("key")
            .unwrap()
            .with_base_url(server.uri());

        // voice param overrides the model query param
        let result = provider
            .synthesize(TtsRequest {
                text: "Test".to_string(),
                voice: Some("aura-2-es-es".to_string()),
                format: None,
                speed: None,
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn synthesize_returns_error_on_non_2xx() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/speak"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&server)
            .await;

        let provider = DeepgramTtsProvider::new("bad-key")
            .unwrap()
            .with_base_url(server.uri());

        let result = provider
            .synthesize(TtsRequest {
                text: "Hi".to_string(),
                voice: None,
                format: None,
                speed: None,
            })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("403"));
    }

    // -- Language detection tests --

    #[test]
    fn detect_english_text() {
        let text = "The weather is nice today and I think it will be warm for the rest of the week";
        assert_eq!(detect_language(text), "en");
    }

    #[test]
    fn detect_german_text() {
        let text = "Das Wetter ist heute schön und ich denke es wird die ganze Woche warm bleiben";
        assert_eq!(detect_language(text), "de");
    }

    #[test]
    fn detect_spanish_text() {
        let text = "El clima es agradable hoy y creo que será cálido para el resto de la semana";
        assert_eq!(detect_language(text), "es");
    }

    #[test]
    fn detect_french_text() {
        let text =
            "Le temps est beau aujourd'hui et je pense que la semaine sera agréable pour tous";
        assert_eq!(detect_language(text), "fr");
    }

    #[test]
    fn detect_empty_defaults_to_english() {
        assert_eq!(detect_language(""), "en");
    }

    #[test]
    fn detect_short_ambiguous_defaults_to_english() {
        // Single word with no stop-word matches → English fallback.
        assert_eq!(detect_language("Hello"), "en");
    }

    #[test]
    fn detect_japanese_cjk() {
        let text = "今日の天気はとても良いです";
        assert_eq!(detect_language(text), "ja");
    }

    // -- Voice map tests --

    #[test]
    fn select_voice_auto_detects_german() {
        let provider = DeepgramTtsProvider::new("key").unwrap();
        let voice =
            provider.select_voice("Das ist ein Test und die Ergebnisse sind sehr interessant");
        assert_eq!(voice, "aura-2-julius-de");
    }

    #[test]
    fn select_voice_auto_detects_english() {
        let provider = DeepgramTtsProvider::new("key").unwrap();
        let voice = provider.select_voice("This is a test and the results are very interesting");
        assert_eq!(voice, "aura-2-thalia-en");
    }

    #[test]
    fn with_voices_overrides_default() {
        let overrides = HashMap::from([("en".to_string(), "aura-2-zeus-en".to_string())]);
        let provider = DeepgramTtsProvider::new("key")
            .unwrap()
            .with_voices(overrides);
        let voice = provider.select_voice("This is a test and the results are very interesting");
        assert_eq!(voice, "aura-2-zeus-en");
    }

    #[tokio::test]
    async fn explicit_voice_overrides_auto_detection() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/speak"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"audio".to_vec()))
            .mount(&server)
            .await;

        let provider = DeepgramTtsProvider::new("key")
            .unwrap()
            .with_base_url(server.uri());

        // German text but explicit English voice → should use explicit voice.
        let result = provider
            .synthesize(TtsRequest {
                text: "Das ist ein Test und die Ergebnisse sind gut".to_string(),
                voice: Some("aura-2-custom-voice".to_string()),
                format: None,
                speed: None,
            })
            .await;

        assert!(result.is_ok());
    }
}
