//! Voice turns ask the provider for a spoken reply, and fall back to the
//! TTS provider when the answer comes back without one.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc;
use uuid::Uuid;

use assistant_core::types::conversation::{Interface, TurnIdentity};
use assistant_core::types::transcription::{TtsConfig, TtsFormat};
use assistant_core::{
    Capabilities, ChatHistoryMessage, ChatOptions, ChatOptionsSupport, ContentBlock,
    GeneratedMedia, LlmProvider, LlmResponse, LlmResponseMeta, MediaKind, StreamChunk, ToolSpec,
    ToolSupport,
};
use assistant_transcription::{TtsProvider, TtsRequest, TtsResult};

use super::*;

/// Answers every call with `answer` (and `media`), and records the options.
struct Speaker {
    audio_output: bool,
    media: Vec<GeneratedMedia>,
    seen: Mutex<Vec<ChatOptions>>,
}

impl Speaker {
    fn new(audio_output: bool, media: Vec<GeneratedMedia>) -> Arc<Self> {
        Arc::new(Self {
            audio_output,
            media,
            seen: Mutex::default(),
        })
    }

    fn answer(&self) -> LlmResponse {
        LlmResponse::FinalAnswer(
            "Sure, it is sunny.".into(),
            LlmResponseMeta {
                media: self.media.clone(),
                ..LlmResponseMeta::default()
            },
        )
    }
}

#[async_trait]
impl LlmProvider for Speaker {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: ToolSupport::Native,
            streaming: true,
            vision: false,
            hosted_tools: vec![],
            context_window_tokens: None,
        }
    }

    fn chat_options_support(&self) -> ChatOptionsSupport {
        ChatOptionsSupport {
            audio_output: self.audio_output,
            ..Default::default()
        }
    }

    async fn chat(
        &self,
        _: &str,
        _: &[ChatHistoryMessage],
        _: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        Ok(self.answer())
    }

    async fn chat_streaming(
        &self,
        _: &str,
        _: &[ChatHistoryMessage],
        _: &[ToolSpec],
        _: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        Ok(self.answer())
    }

    async fn chat_with(
        &self,
        _: &str,
        _: &[ChatHistoryMessage],
        _: &[ToolSpec],
        options: &ChatOptions,
        _: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        self.seen.lock().unwrap().push(options.clone());
        Ok(self.answer())
    }
}

/// Speaks every text as `b"tts:<text>"`.
#[derive(Default)]
struct StubTts {
    spoken: Mutex<Vec<String>>,
}

#[async_trait]
impl TtsProvider for StubTts {
    fn name(&self) -> &str {
        "stub-tts"
    }

    async fn synthesize(&self, request: TtsRequest) -> anyhow::Result<TtsResult> {
        self.spoken.lock().unwrap().push(request.text.clone());
        Ok(TtsResult {
            audio_data: format!("tts:{}", request.text).into_bytes(),
            mime_type: "audio/ogg".into(),
        })
    }
}

fn config(format: Option<TtsFormat>) -> OrgConfig {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.tts = Some(TtsConfig {
        provider: assistant_core::types::transcription::TtsProviderKind::OpenRouter,
        model: None,
        voice: None,
        base_url: None,
        api_key: None,
        voices: Default::default(),
        format,
        speed: None,
        instructions: None,
        pcm_sample_rate: None,
    });
    config
}

fn voice_note() -> ContentBlock {
    ContentBlock::Audio {
        media_type: "audio/ogg".into(),
        data: "T2dnUw==".into(),
    }
}

async fn turn(
    llm: Arc<Speaker>,
    config: OrgConfig,
    tts: Option<Arc<StubTts>>,
    voice: bool,
) -> crate::TurnResult {
    let (orch, _) = build_with_llm(llm, config, |o| match tts {
        Some(tts) => o.with_tts(tts),
        None => o,
    })
    .await;
    orch.run_turn_with_tools(
        "[Voice message]: weather?",
        Uuid::new_v4(),
        Interface::Web,
        vec![],
        None,
        if voice { vec![voice_note()] } else { vec![] },
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap()
}

fn spoken_reply() -> GeneratedMedia {
    GeneratedMedia {
        kind: MediaKind::Audio,
        mime: "audio/ogg".into(),
        data: bytes::Bytes::from_static(b"native"),
    }
}

#[tokio::test]
async fn voice_turn_asks_for_a_spoken_reply_in_the_tts_format() {
    let llm = Speaker::new(true, vec![]);
    turn(llm.clone(), config(Some(TtsFormat::Ogg)), None, true).await;

    let seen = llm.seen.lock().unwrap();
    assert_eq!(seen[0].audio_reply, Some(TtsFormat::Ogg));
}

#[tokio::test]
async fn typed_turns_and_providers_without_audio_output_ask_for_text() {
    for (audio_output, voice) in [(true, false), (false, true)] {
        let llm = Speaker::new(audio_output, vec![]);
        turn(llm.clone(), config(None), None, voice).await;
        assert_eq!(
            llm.seen.lock().unwrap()[0].audio_reply,
            None,
            "audio_output {audio_output}, voice {voice}"
        );
    }
}

#[tokio::test]
async fn reply_without_audio_is_spoken_by_the_tts_provider() {
    let tts = Arc::new(StubTts::default());
    let result = turn(
        Speaker::new(false, vec![]),
        config(None),
        Some(tts.clone()),
        true,
    )
    .await;

    assert_eq!(*tts.spoken.lock().unwrap(), ["Sure, it is sunny."]);
    let audio: Vec<_> = result
        .attachments
        .iter()
        .filter(|a| a.mime_type == "audio/ogg")
        .collect();
    assert_eq!(audio.len(), 1, "{:?}", result.attachments);
    assert_eq!(audio[0].data, b"tts:Sure, it is sunny.");
}

#[tokio::test]
async fn native_audio_needs_no_tts_and_typed_turns_get_none() {
    let tts = Arc::new(StubTts::default());
    let native = turn(
        Speaker::new(true, vec![spoken_reply()]),
        config(None),
        Some(tts.clone()),
        true,
    )
    .await;
    assert_eq!(native.attachments.len(), 1, "only the native clip");
    assert_eq!(native.attachments[0].data, b"native");

    let typed = turn(
        Speaker::new(false, vec![]),
        config(None),
        Some(tts.clone()),
        false,
    )
    .await;
    assert!(typed.attachments.is_empty(), "{:?}", typed.attachments);
    assert!(tts.spoken.lock().unwrap().is_empty(), "nothing synthesized");
}

#[tokio::test]
async fn without_tts_a_voice_turn_answers_in_text() {
    let result = turn(Speaker::new(false, vec![]), config(None), None, true).await;
    assert!(result.attachments.is_empty(), "{:?}", result.attachments);
}

/// Returns one spoken clip, like `voice-response`.
struct SpeakTool;

#[async_trait]
impl assistant_core::ToolHandler for SpeakTool {
    fn name(&self) -> &str {
        "speak"
    }

    fn description(&self) -> &str {
        "speaks"
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    async fn run(
        &self,
        _params: std::collections::HashMap<String, serde_json::Value>,
        _ctx: &assistant_core::types::conversation::ExecutionContext,
    ) -> anyhow::Result<assistant_core::ToolOutput> {
        Ok(
            assistant_core::ToolOutput::success("spoken").with_attachments(vec![
                assistant_core::Attachment::new("voice.ogg", "audio/ogg", b"tool".to_vec()),
            ]),
        )
    }
}

#[tokio::test]
async fn a_clip_from_a_tool_needs_no_tts() {
    let llm =
        assistant_llm_provider::scripted::ScriptedLlmProvider::new().with_canned_responses(vec![
            LlmResponse::ToolCalls(assistant_core::ToolCallResponse {
                items: vec![assistant_core::ToolCallItem {
                    name: "speak".into(),
                    params: serde_json::json!({}),
                    id: Some("call-1".into()),
                }],
                meta: LlmResponseMeta::default(),
                thinking: None,
                provider_state: None,
            }),
            LlmResponse::FinalAnswer("Said it.".into(), LlmResponseMeta::default()),
        ]);
    let mut config = config(None);
    allow_all_tools(&mut config);
    let tts = Arc::new(StubTts::default());
    let tts_for_build = tts.clone();
    let (orch, _) = build_with_llm(Arc::new(llm), config, |o| o.with_tts(tts_for_build)).await;

    let result = orch
        .run_turn_with_tools(
            "[Voice message]: say it",
            Uuid::new_v4(),
            Interface::Web,
            vec![Arc::new(SpeakTool) as Arc<dyn assistant_core::ToolHandler>],
            None,
            vec![voice_note()],
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert!(tts.spoken.lock().unwrap().is_empty(), "nothing synthesized");
    assert_eq!(result.attachments.len(), 1, "only the tool's clip");
}
