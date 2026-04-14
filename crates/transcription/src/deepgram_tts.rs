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
            .bearer_auth(&self.api_key)
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
