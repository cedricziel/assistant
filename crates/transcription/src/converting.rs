//! Converts audio to a format the wrapped transcription provider accepts.

use async_trait::async_trait;
use tracing::debug;

use crate::provider::{TranscriptionProvider, TranscriptionRequest, TranscriptionResult};
use assistant_audio::AudioFormat;
use assistant_audio::AudioPipeline;

/// Wraps a provider that declares
/// [`accepted_formats`](TranscriptionProvider::accepted_formats). Audio in
/// an accepted format, or in a format the system doesn't recognize, is sent
/// unchanged; other recognized formats become 16 kHz mono WAV first.
pub struct ConvertingTranscriber<P> {
    inner: P,
    pipeline: AudioPipeline,
}

impl<P: TranscriptionProvider> ConvertingTranscriber<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            pipeline: AudioPipeline::default(),
        }
    }

    pub fn with_pipeline(mut self, pipeline: AudioPipeline) -> Self {
        self.pipeline = pipeline;
        self
    }
}

#[async_trait]
impl<P: TranscriptionProvider> TranscriptionProvider for ConvertingTranscriber<P> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn accepted_formats(&self) -> Option<&[AudioFormat]> {
        self.inner.accepted_formats()
    }

    async fn transcribe(
        &self,
        mut request: TranscriptionRequest,
    ) -> anyhow::Result<TranscriptionResult> {
        let format = AudioFormat::from_mime(&request.mime_type);
        let accepted = self
            .inner
            .accepted_formats()
            .is_none_or(|formats| formats.contains(&format));
        if !accepted && format != AudioFormat::Unknown {
            debug!(
                ?format,
                provider = self.inner.name(),
                "Converting audio to WAV for transcription"
            );
            request.audio_data = self
                .pipeline
                .to_wav_16k_mono(&request.audio_data, format)
                .await?;
            request.mime_type = "audio/wav".to_owned();
            request.filename = request.filename.map(|name| match name.rsplit_once('.') {
                Some((stem, _)) => format!("{stem}.wav"),
                None => format!("{name}.wav"),
            });
        }
        self.inner.transcribe(request).await
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::ConvertingTranscriber;
    use crate::provider::{TranscriptionProvider, TranscriptionRequest, TranscriptionResult};
    use assistant_audio::AudioFormat;
    use assistant_audio::AudioPipeline;

    /// Accepts Ogg and WAV, and records what it was sent.
    #[derive(Default)]
    struct Recorder {
        sent: Mutex<Vec<(String, Vec<u8>)>>,
    }

    #[async_trait]
    impl TranscriptionProvider for Recorder {
        fn name(&self) -> &str {
            "recorder"
        }

        fn accepted_formats(&self) -> Option<&[AudioFormat]> {
            Some(&[AudioFormat::Ogg, AudioFormat::Wav])
        }

        async fn transcribe(
            &self,
            request: TranscriptionRequest,
        ) -> anyhow::Result<TranscriptionResult> {
            self.sent
                .lock()
                .unwrap()
                .push((request.mime_type, request.audio_data));
            Ok(TranscriptionResult {
                text: "ok".into(),
                language: None,
                duration_secs: None,
            })
        }
    }

    fn request(mime: &str, data: Vec<u8>) -> TranscriptionRequest {
        TranscriptionRequest {
            audio_data: data,
            mime_type: mime.into(),
            filename: None,
            language: None,
        }
    }

    fn without_ffmpeg() -> AudioPipeline {
        AudioPipeline::default().with_ffmpeg_path("/nonexistent/ffmpeg")
    }

    fn caf_tone() -> Option<Vec<u8>> {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("tone.caf");
        let ok = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=duration=1",
            ])
            .arg(&out)
            .status()
            .is_ok_and(|s| s.success());
        ok.then(|| std::fs::read(out).unwrap())
    }

    #[tokio::test]
    async fn accepted_format_passes_through_without_ffmpeg() {
        let converting =
            ConvertingTranscriber::new(Recorder::default()).with_pipeline(without_ffmpeg());

        converting
            .transcribe(request("audio/ogg; codecs=opus", b"OggS-bytes".to_vec()))
            .await
            .unwrap();

        let sent = converting.inner.sent.lock().unwrap().clone();
        assert_eq!(
            sent,
            [("audio/ogg; codecs=opus".to_string(), b"OggS-bytes".to_vec())]
        );
    }

    #[tokio::test]
    async fn other_formats_are_converted_to_wav() {
        let Some(caf) = caf_tone() else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };
        let converting = ConvertingTranscriber::new(Recorder::default());

        converting
            .transcribe(request("audio/x-caf", caf))
            .await
            .unwrap();

        let sent = converting.inner.sent.lock().unwrap().clone();
        assert_eq!(sent[0].0, "audio/wav");
        assert_eq!(&sent[0].1[..4], b"RIFF");
    }

    #[tokio::test]
    async fn missing_ffmpeg_fails_with_a_clear_error() {
        let converting =
            ConvertingTranscriber::new(Recorder::default()).with_pipeline(without_ffmpeg());

        let err = converting
            .transcribe(request("audio/amr", b"#!AMR".to_vec()))
            .await
            .unwrap_err()
            .to_string();

        assert!(err.contains("ffmpeg") && err.contains("amr"), "{err}");
        assert!(converting.inner.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unrecognized_formats_pass_through() {
        let converting =
            ConvertingTranscriber::new(Recorder::default()).with_pipeline(without_ffmpeg());

        converting
            .transcribe(request("application/octet-stream", vec![1, 2, 3]))
            .await
            .unwrap();

        assert_eq!(converting.inner.sent.lock().unwrap().len(), 1);
    }
}
