//! OpenAI TTS provider — `POST /v1/audio/speech`.

use anyhow::{Context, bail};
use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use tracing::debug;

use crate::provider::{TtsProvider, TtsRequest, TtsResult};

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const DEFAULT_MODEL: &str = "tts-1";
const DEFAULT_VOICE: &str = "nova";
const DEFAULT_FORMAT: &str = "mp3";

pub struct OpenAITtsProvider {
    base_url: String,
    api_key: String,
    model: String,
    default_voice: String,
    client: ClientWithMiddleware,
}

impl OpenAITtsProvider {
    pub fn new(api_key: impl Into<String>) -> anyhow::Result<Self> {
        let client = assistant_llm::build_http_client(
            DEFAULT_TIMEOUT_SECS,
            &assistant_llm::RetryConfig::default(),
        )?;
        Ok(Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            default_voice: DEFAULT_VOICE.to_string(),
            client,
        })
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.default_voice = voice.into();
        self
    }
}

#[async_trait]
impl TtsProvider for OpenAITtsProvider {
    fn name(&self) -> &str {
        "openai-tts"
    }

    async fn synthesize(&self, request: TtsRequest) -> anyhow::Result<TtsResult> {
        let voice = request.voice.unwrap_or_else(|| self.default_voice.clone());
        let format = request.format.unwrap_or_else(|| DEFAULT_FORMAT.to_string());

        debug!(
            provider = "openai-tts",
            model = %self.model,
            voice = %voice,
            text_len = request.text.len(),
            "Synthesizing speech"
        );

        let body = serde_json::json!({
            "model": self.model,
            "input": request.text,
            "voice": voice,
            "response_format": format,
        });

        let url = format!("{}/audio/speech", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("OpenAI TTS API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("OpenAI TTS API returned {status}: {body}");
        }

        let audio_data = resp
            .bytes()
            .await
            .context("Failed to read OpenAI TTS response bytes")?
            .to_vec();

        debug!(bytes = audio_data.len(), "TTS synthesis complete");

        Ok(TtsResult {
            audio_data,
            mime_type: "audio/mpeg".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn provider_name() {
        let p = OpenAITtsProvider::new("key").unwrap();
        assert_eq!(p.name(), "openai-tts");
    }

    #[tokio::test]
    async fn synthesize_returns_audio_bytes_on_success() {
        let server = MockServer::start().await;
        let fake_audio = b"FAKE_MP3".to_vec();

        Mock::given(method("POST"))
            .and(path("/audio/speech"))
            .and(header_exists("authorization"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(fake_audio.clone())
                    .insert_header("content-type", "audio/mpeg"),
            )
            .mount(&server)
            .await;

        let provider = OpenAITtsProvider::new("test-key")
            .unwrap()
            .with_base_url(server.uri());

        let result = provider
            .synthesize(TtsRequest {
                text: "Hello world".to_string(),
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
    async fn synthesize_uses_custom_voice() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/audio/speech"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"audio".to_vec()))
            .mount(&server)
            .await;

        let provider = OpenAITtsProvider::new("key")
            .unwrap()
            .with_base_url(server.uri());

        let result = provider
            .synthesize(TtsRequest {
                text: "Test".to_string(),
                voice: Some("alloy".to_string()),
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
            .and(path("/audio/speech"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let provider = OpenAITtsProvider::new("bad-key")
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
        assert!(result.unwrap_err().to_string().contains("401"));
    }
}
