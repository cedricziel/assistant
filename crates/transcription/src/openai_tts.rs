//! OpenAI TTS provider — `POST /v1/audio/speech`.

use anyhow::{bail, Context};
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
