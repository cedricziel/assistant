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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use assistant_core::{ExecutionContext, Interface};
    use assistant_transcription::{AudioStore, TtsProvider, TtsRequest, TtsResult};
    use async_trait::async_trait;
    use uuid::Uuid;

    use assistant_core::ToolHandler;

    use super::VoiceResponseHandler;

    struct FakeTts {
        audio: Vec<u8>,
    }

    #[async_trait]
    impl TtsProvider for FakeTts {
        fn name(&self) -> &str {
            "fake-tts"
        }

        async fn synthesize(&self, _request: TtsRequest) -> anyhow::Result<TtsResult> {
            Ok(TtsResult {
                audio_data: self.audio.clone(),
                mime_type: "audio/mpeg".to_string(),
            })
        }
    }

    struct FailingTts;

    #[async_trait]
    impl TtsProvider for FailingTts {
        fn name(&self) -> &str {
            "failing-tts"
        }

        async fn synthesize(&self, _request: TtsRequest) -> anyhow::Result<TtsResult> {
            anyhow::bail!("synthesis failed")
        }
    }

    fn make_ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "test".to_string(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: None,
            depth: 0,
        }
    }

    #[tokio::test]
    async fn tool_name_is_voice_response() {
        let handler = VoiceResponseHandler::new(
            Arc::new(FakeTts { audio: vec![] }),
            Arc::new(AudioStore::new()),
        );
        assert_eq!(handler.name(), "voice-response");
    }

    #[tokio::test]
    async fn run_synthesizes_and_returns_audio_id() {
        let audio_bytes = b"fake-mp3".to_vec();
        let store = Arc::new(AudioStore::new());
        let handler = VoiceResponseHandler::new(
            Arc::new(FakeTts {
                audio: audio_bytes.clone(),
            }),
            store.clone(),
        );

        let mut params = HashMap::new();
        params.insert("text".to_string(), serde_json::json!("Hello"));

        let ctx = make_ctx();
        let output = handler.run(params, &ctx).await.unwrap();

        assert!(output.success, "expected successful output");
        let data = output.data.expect("expected data payload");
        let audio_id = data["audio_id"]
            .as_str()
            .expect("audio_id must be a string");
        let uuid: Uuid = audio_id.parse().expect("audio_id must be a valid UUID");

        // The audio should be retrievable from the store.
        let stored = store.get(uuid).await;
        assert_eq!(stored, Some(audio_bytes));
        assert_eq!(data["voiced"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn run_returns_error_output_when_text_is_missing() {
        let handler = VoiceResponseHandler::new(
            Arc::new(FakeTts { audio: vec![] }),
            Arc::new(AudioStore::new()),
        );

        let params = HashMap::new(); // no "text" key
        let output = handler.run(params, &make_ctx()).await.unwrap();

        assert!(!output.success, "missing text should yield an error output");
    }

    #[tokio::test]
    async fn run_returns_error_output_when_text_is_empty() {
        let handler = VoiceResponseHandler::new(
            Arc::new(FakeTts { audio: vec![] }),
            Arc::new(AudioStore::new()),
        );

        let mut params = HashMap::new();
        params.insert("text".to_string(), serde_json::json!("   "));
        let output = handler.run(params, &make_ctx()).await.unwrap();

        assert!(!output.success, "blank text should yield an error output");
    }

    #[tokio::test]
    async fn run_returns_error_output_when_tts_fails() {
        let handler = VoiceResponseHandler::new(Arc::new(FailingTts), Arc::new(AudioStore::new()));

        let mut params = HashMap::new();
        params.insert("text".to_string(), serde_json::json!("Hello"));
        let output = handler.run(params, &make_ctx()).await.unwrap();

        assert!(!output.success, "TTS failure should yield an error output");
    }

    #[tokio::test]
    async fn run_passes_custom_voice_to_tts() {
        // We just check it doesn't panic / error when a voice param is supplied.
        let store = Arc::new(AudioStore::new());
        let handler = VoiceResponseHandler::new(
            Arc::new(FakeTts {
                audio: b"ok".to_vec(),
            }),
            store.clone(),
        );

        let mut params = HashMap::new();
        params.insert("text".to_string(), serde_json::json!("Hi"));
        params.insert("voice".to_string(), serde_json::json!("alloy"));

        let output = handler.run(params, &make_ctx()).await.unwrap();
        assert!(output.success);
    }
}
