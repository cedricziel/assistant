//! `voice-response` tool — synthesize a voiced reply for the assistant.
//!
//! When TTS is configured, the assistant can call this tool to produce an
//! audio response. The synthesized audio is stored in the [`AudioStore`] and
//! an `audio_id` is returned so the web UI can auto-play it.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use assistant_transcription::{AudioStore, TtsProvider, TtsRequest};
use async_trait::async_trait;
use serde_json::Value;
use tracing::warn;

pub struct VoiceResponseHandler {
    tts: Arc<dyn TtsProvider>,
    store: Arc<AudioStore>,
}

impl VoiceResponseHandler {
    pub fn new(tts: Arc<dyn TtsProvider>, store: Arc<AudioStore>) -> Self {
        Self { tts, store }
    }
}

#[async_trait]
impl ToolHandler for VoiceResponseHandler {
    fn name(&self) -> &str {
        "voice-response"
    }

    fn description(&self) -> &str {
        "Synthesize a voiced audio reply. Call this when you want to respond with \
         speech rather than (or in addition to) text. The audio will be played \
         automatically in voice-enabled clients."
    }

    fn params_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to synthesize as speech."
                },
                "voice": {
                    "type": "string",
                    "description": "Optional voice name (provider-specific, e.g. \"nova\", \"alloy\")."
                }
            },
            "required": ["text"]
        })
    }

    fn output_schema(&self) -> Option<Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "audio_id": {
                    "type": "string",
                    "description": "UUID of the stored audio blob. Retrieve via GET /api/audio/{audio_id}."
                },
                "voiced": {
                    "type": "boolean",
                    "description": "Always true on success."
                }
            },
            "required": ["audio_id", "voiced"]
        }))
    }

    async fn run(
        &self,
        params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let text = match params.get("text").and_then(|v| v.as_str()) {
            Some(t) if !t.trim().is_empty() => t.to_string(),
            _ => {
                return Ok(ToolOutput::error(
                    "voice-response: `text` parameter is required",
                ))
            }
        };
        let voice = params
            .get("voice")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let request = TtsRequest {
            text,
            voice,
            format: Some("mp3".to_string()),
            speed: None,
        };

        match self.tts.synthesize(request).await {
            Ok(result) => {
                let audio_id = self.store.insert(result.audio_data).await;
                Ok(
                    ToolOutput::success(format!("Audio synthesized (id={audio_id}).")).with_data(
                        serde_json::json!({
                            "audio_id": audio_id.to_string(),
                            "voiced": true,
                        }),
                    ),
                )
            }
            Err(e) => {
                warn!(error = %e, "voice-response: TTS synthesis failed");
                Ok(ToolOutput::error(format!("TTS synthesis failed: {e}")))
            }
        }
    }
}
