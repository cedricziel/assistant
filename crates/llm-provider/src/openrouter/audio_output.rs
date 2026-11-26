//! Spoken replies: the `audio` modality, and the PCM it streams back.

use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;
use tracing::warn;

use assistant_audio::{AudioFormat, AudioInput, AudioPipeline};
use assistant_core::types::llm::OpenRouterAudioOutput;
use assistant_core::types::transcription::TtsFormat;
use assistant_core::{GeneratedMedia, MediaKind};

use super::audio_input::is_audio_rejection;
use super::wire::ChatRequest;

pub struct AudioOutput {
    settings: OpenRouterAudioOutput,
    pipeline: AudioPipeline,
    /// Set once the upstream rejected audio output; no audio is asked for
    /// after that.
    rejected: AtomicBool,
}

impl AudioOutput {
    pub fn new(settings: OpenRouterAudioOutput) -> Self {
        Self {
            settings,
            pipeline: AudioPipeline::default(),
            rejected: AtomicBool::new(false),
        }
    }

    /// Whether audio is asked of a model whose catalog lists audio output.
    pub fn enabled(&self, model_speaks: bool) -> bool {
        self.settings.enabled && model_speaks && !self.rejected.load(Ordering::Relaxed)
    }

    /// Stops asking for audio when `error` is the upstream rejecting it, and
    /// says whether it was.
    pub fn reject_if_audio_error(&self, error: &anyhow::Error) -> bool {
        let Some(reason) = is_audio_rejection(error) else {
            return false;
        };
        if !self.rejected.swap(true, Ordering::Relaxed) {
            warn!(error = %reason, "OpenRouter rejected audio output; replying in text from now on");
        }
        true
    }

    /// Asks `request` for a spoken reply, which only a stream carries.
    pub fn request(&self, request: &mut ChatRequest) {
        request.stream = true;
        if request.modalities.is_empty() {
            request.modalities.push("text");
        }
        request.modalities.push("audio");
        request.audio = Some(json!({"voice": self.settings.voice, "format": "pcm16"}));
    }

    /// The reply's PCM16 as a clip in `format`, or `None` when it can't be
    /// transcoded; the answer's text then stands alone.
    pub async fn clip(&self, pcm: Vec<u8>, format: TtsFormat) -> Option<GeneratedMedia> {
        let target = match format {
            TtsFormat::Mp3 => AudioFormat::Mp3,
            TtsFormat::Wav => AudioFormat::Wav,
            TtsFormat::Ogg => AudioFormat::Ogg,
            TtsFormat::Pcm => {
                return Some(media(format, pcm));
            }
        };
        let input = AudioInput::Pcm {
            sample_rate: self.settings.pcm_sample_rate,
        };
        match self.pipeline.transcode(&pcm, input, target).await {
            Ok(data) => Some(media(format, data)),
            Err(e) => {
                warn!(error = %e, "could not transcode OpenRouter's spoken reply");
                None
            }
        }
    }
}

fn media(format: TtsFormat, data: Vec<u8>) -> GeneratedMedia {
    GeneratedMedia {
        kind: MediaKind::Audio,
        mime: format.mime_type().to_owned(),
        data: data.into(),
    }
}
