//! Deepgram TTS provider — `POST /v1/speak`.

use anyhow::{bail, Context};
use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use tracing::debug;

use crate::provider::{TtsProvider, TtsRequest, TtsResult};

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const DEFAULT_MODEL: &str = "aura-2-en-us";

pub struct DeepgramTtsProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: ClientWithMiddleware,
}

impl DeepgramTtsProvider {
    pub fn new(api_key: impl Into<String>) -> anyhow::Result<Self> {
        let client = assistant_llm::build_http_client(
            DEFAULT_TIMEOUT_SECS,
            &assistant_llm::RetryConfig::default(),
        )?;
        Ok(Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            base_url: "https://api.deepgram.com/v1".to_string(),
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
}

#[async_trait]
impl TtsProvider for DeepgramTtsProvider {
    fn name(&self) -> &str {
        "deepgram-tts"
    }

    async fn synthesize(&self, request: TtsRequest) -> anyhow::Result<TtsResult> {
        let model = request.voice.unwrap_or_else(|| self.model.clone());

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
}
