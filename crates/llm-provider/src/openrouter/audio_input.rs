//! Voice notes sent to the model as `input_audio`.

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use base64::Engine as _;
use tracing::warn;

use assistant_audio::{AudioFormat, AudioPipeline};
use assistant_core::types::llm::OpenRouterAudioInput;
use assistant_core::{ChatHistoryMessage, ContentBlock, VOICE_PLACEHOLDER};

use crate::retry::ProviderHttpError;

/// Labels the transcript that goes with the audio.
const TRANSCRIPT_LABEL: &str = "Transcript (may contain errors): ";

pub struct AudioInput {
    settings: OpenRouterAudioInput,
    pipeline: AudioPipeline,
    /// Set once the upstream rejected audio; no audio is sent after that.
    rejected: AtomicBool,
}

impl AudioInput {
    pub fn new(settings: OpenRouterAudioInput) -> Self {
        Self {
            settings,
            pipeline: AudioPipeline::default(),
            rejected: AtomicBool::new(false),
        }
    }

    /// Whether audio is sent to a model whose catalog lists audio input.
    pub fn enabled(&self, model_hears: bool) -> bool {
        self.settings.enabled && model_hears && !self.rejected.load(Ordering::Relaxed)
    }

    /// Stops sending audio when `error` is the upstream rejecting it, and
    /// says whether it was.
    pub fn reject_if_audio_error(&self, error: &anyhow::Error) -> bool {
        let Some(http) = error.downcast_ref::<ProviderHttpError>() else {
            return false;
        };
        if http.status != 400 || !http.message.to_lowercase().contains("audio") {
            return false;
        }
        if !self.rejected.swap(true, Ordering::Relaxed) {
            warn!(error = %http, "OpenRouter rejected audio input; sending voice notes as text from now on");
        }
        true
    }

    /// `history` with each recording ready to send: in an accepted format,
    /// no longer than `max_secs`, and its text labelled as a transcript.
    /// Recordings that can't be sent are left out.
    pub async fn prepare<'a>(
        &self,
        history: &'a [ChatHistoryMessage],
    ) -> Cow<'a, [ChatHistoryMessage]> {
        if !has_audio(history) {
            return Cow::Borrowed(history);
        }
        let mut prepared = Vec::with_capacity(history.len());
        for message in history {
            prepared.push(match message {
                ChatHistoryMessage::MultimodalUser { content } if blocks_have_audio(content) => {
                    ChatHistoryMessage::MultimodalUser {
                        content: self.prepare_content(content).await,
                    }
                }
                other => other.clone(),
            });
        }
        Cow::Owned(prepared)
    }

    async fn prepare_content(&self, content: &[ContentBlock]) -> Vec<ContentBlock> {
        let mut blocks = Vec::with_capacity(content.len());
        let mut kept = false;
        for block in content {
            match block {
                ContentBlock::Audio { media_type, data } => {
                    if let Some(audio) = self.prepare_audio(media_type, data).await {
                        kept = true;
                        blocks.push(audio);
                    }
                }
                other => blocks.push(other.clone()),
            }
        }
        if kept {
            for block in &mut blocks {
                if let ContentBlock::Text(text) = block
                    && text != VOICE_PLACEHOLDER
                {
                    text.insert_str(0, TRANSCRIPT_LABEL);
                }
            }
        }
        blocks
    }

    /// The recording in an accepted format, or `None` when it is too long
    /// or can't be read.
    async fn prepare_audio(&self, media_type: &str, data: &str) -> Option<ContentBlock> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| warn!(error = %e, "voice note is not valid base64; sending text only"))
            .ok()?;
        let format = AudioFormat::from_mime(media_type);
        let duration = match self.pipeline.probe_duration(&bytes, format).await {
            Ok(Some(duration)) => duration,
            Ok(None) => {
                warn!(
                    media_type,
                    "voice note has no readable duration; sending text only"
                );
                return None;
            }
            Err(e) => {
                warn!(error = %e, "could not probe the voice note; sending text only");
                return None;
            }
        };
        if duration > Duration::from_secs(u64::from(self.settings.max_secs)) {
            return None;
        }
        if self
            .settings
            .formats
            .iter()
            .any(|f| f == format.extension())
        {
            return Some(ContentBlock::Audio {
                media_type: media_type.to_owned(),
                data: data.to_owned(),
            });
        }
        let wav = self
            .pipeline
            .to_wav_16k_mono(&bytes, format)
            .await
            .map_err(|e| warn!(error = %e, "could not convert the voice note; sending text only"))
            .ok()?;
        Some(ContentBlock::Audio {
            media_type: "audio/wav".to_owned(),
            data: base64::engine::general_purpose::STANDARD.encode(wav),
        })
    }
}

/// `history` without recordings; their text stands in.
pub fn strip(history: &[ChatHistoryMessage]) -> Cow<'_, [ChatHistoryMessage]> {
    if !has_audio(history) {
        return Cow::Borrowed(history);
    }
    Cow::Owned(
        history
            .iter()
            .map(|message| match message {
                ChatHistoryMessage::MultimodalUser { content } => {
                    ChatHistoryMessage::MultimodalUser {
                        content: content
                            .iter()
                            .filter(|b| !matches!(b, ContentBlock::Audio { .. }))
                            .cloned()
                            .collect(),
                    }
                }
                other => other.clone(),
            })
            .collect(),
    )
}

pub fn has_audio(history: &[ChatHistoryMessage]) -> bool {
    history.iter().any(|m| {
        matches!(m, ChatHistoryMessage::MultimodalUser { content } if blocks_have_audio(content))
    })
}

fn blocks_have_audio(content: &[ContentBlock]) -> bool {
    content
        .iter()
        .any(|b| matches!(b, ContentBlock::Audio { .. }))
}

/// The `input_audio` format for a recording's MIME type.
pub fn wire_format(media_type: &str) -> &'static str {
    AudioFormat::from_mime(media_type).extension()
}
