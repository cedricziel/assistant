//! ffmpeg-based audio pipeline shared by transcription and speech providers:
//! duration probing, conversion to 16 kHz mono WAV, splitting and
//! transcoding.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use tokio::process::Command;

use crate::converter::AudioFormat;

/// Audio handed to [`AudioPipeline::transcode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioInput {
    /// A container or codec ffmpeg can detect.
    Encoded(AudioFormat),
    /// Raw signed 16-bit little-endian mono PCM at `sample_rate` Hz.
    Pcm { sample_rate: u32 },
}

/// Runs ffmpeg and ffprobe. Inputs go through temp files, because some
/// containers (MP4, MOV) need seekable input.
#[derive(Debug, Clone)]
pub struct AudioPipeline {
    ffmpeg: String,
    ffprobe: String,
}

impl Default for AudioPipeline {
    fn default() -> Self {
        Self {
            ffmpeg: "ffmpeg".to_owned(),
            ffprobe: "ffprobe".to_owned(),
        }
    }
}

impl AudioPipeline {
    pub fn with_ffmpeg_path(mut self, path: impl Into<String>) -> Self {
        self.ffmpeg = path.into();
        self
    }

    pub fn with_ffprobe_path(mut self, path: impl Into<String>) -> Self {
        self.ffprobe = path.into();
        self
    }

    /// Fails unless ffmpeg runs, naming `format` as the reason it is needed.
    pub async fn require(&self, format: AudioFormat) -> anyhow::Result<()> {
        let ran = Command::new(&self.ffmpeg)
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .is_ok_and(|s| s.success());
        if !ran {
            anyhow::bail!(
                "ffmpeg is needed to convert {} audio, but `{}` could not be run",
                format.extension(),
                self.ffmpeg
            );
        }
        Ok(())
    }

    /// The duration of `data`, or `None` when ffprobe can't tell.
    pub async fn probe_duration(
        &self,
        data: &[u8],
        format: AudioFormat,
    ) -> anyhow::Result<Option<Duration>> {
        let dir = tempfile::tempdir().context("Failed to create a temp dir")?;
        let input = write_input(dir.path(), data, format.extension()).await?;
        let output = Command::new(&self.ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "csv=p=0",
            ])
            .arg(&input)
            .output()
            .await
            .with_context(|| format!("Failed to run ffprobe at {}", self.ffprobe))?;
        if !output.status.success() {
            anyhow::bail!(
                "ffprobe failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|secs| secs.is_finite() && *secs >= 0.0)
            .map(Duration::from_secs_f64))
    }

    /// `data` as 16 kHz mono WAV, taking only the audio stream of a video.
    pub async fn to_wav_16k_mono(
        &self,
        data: &[u8],
        format: AudioFormat,
    ) -> anyhow::Result<Vec<u8>> {
        self.require(format).await?;
        let dir = tempfile::tempdir().context("Failed to create a temp dir")?;
        let input = write_input(dir.path(), data, format.extension()).await?;
        let output = dir.path().join("out.wav");
        self.ffmpeg(&[
            "-i".as_ref(),
            input.as_os_str(),
            "-vn".as_ref(),
            "-ar".as_ref(),
            "16000".as_ref(),
            "-ac".as_ref(),
            "1".as_ref(),
            "-f".as_ref(),
            "wav".as_ref(),
            output.as_os_str(),
        ])
        .await?;
        read_output(&output).await
    }

    /// Splits WAV `data` into consecutive WAV pieces of at most `max_secs`.
    pub async fn split(&self, data: &[u8], max_secs: u32) -> anyhow::Result<Vec<Vec<u8>>> {
        self.require(AudioFormat::Wav).await?;
        let dir = tempfile::tempdir().context("Failed to create a temp dir")?;
        let input = write_input(dir.path(), data, "wav").await?;
        let pattern = dir.path().join("piece-%05d.wav");
        let secs = max_secs.max(1).to_string();
        self.ffmpeg(&[
            "-i".as_ref(),
            input.as_os_str(),
            "-f".as_ref(),
            "segment".as_ref(),
            "-segment_time".as_ref(),
            secs.as_ref(),
            "-c".as_ref(),
            "copy".as_ref(),
            pattern.as_os_str(),
        ])
        .await?;
        let mut pieces: Vec<PathBuf> = std::fs::read_dir(dir.path())
            .context("Failed to list split pieces")?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("piece-"))
            })
            .collect();
        pieces.sort();
        let mut out = Vec::with_capacity(pieces.len());
        for piece in pieces {
            out.push(read_output(&piece).await?);
        }
        Ok(out)
    }

    /// `data` re-encoded as `target`: MP3, Ogg/Opus (48 kHz mono 32 kbps,
    /// as messenger voice notes use) or WAV.
    pub async fn transcode(
        &self,
        data: &[u8],
        input: AudioInput,
        target: AudioFormat,
    ) -> anyhow::Result<Vec<u8>> {
        let source_format = match input {
            AudioInput::Encoded(format) => format,
            AudioInput::Pcm { .. } => AudioFormat::Wav,
        };
        self.require(source_format).await?;
        let dir = tempfile::tempdir().context("Failed to create a temp dir")?;
        let input_path = write_input(
            dir.path(),
            data,
            match input {
                AudioInput::Encoded(format) => format.extension(),
                AudioInput::Pcm { .. } => "pcm",
            },
        )
        .await?;
        let output = dir.path().join(format!("out.{}", target.extension()));

        let mut args: Vec<std::ffi::OsString> = Vec::new();
        if let AudioInput::Pcm { sample_rate } = input {
            args.extend(["-f", "s16le", "-ar"].map(Into::into));
            args.push(sample_rate.to_string().into());
            args.extend(["-ac", "1"].map(Into::into));
        }
        args.push("-i".into());
        args.push(input_path.into_os_string());
        args.push("-vn".into());
        match target {
            AudioFormat::Mp3 => args.extend(["-c:a", "libmp3lame", "-f", "mp3"].map(Into::into)),
            AudioFormat::Ogg => args.extend(
                [
                    "-c:a", "libopus", "-ar", "48000", "-ac", "1", "-b:a", "32k", "-f", "ogg",
                ]
                .map(Into::into),
            ),
            AudioFormat::Wav => args.extend(["-f", "wav"].map(Into::into)),
            other => anyhow::bail!("Cannot transcode to {}", other.extension()),
        }
        args.push(output.clone().into_os_string());
        let refs: Vec<&std::ffi::OsStr> = args.iter().map(|a| a.as_os_str()).collect();
        self.ffmpeg(&refs).await?;
        read_output(&output).await
    }

    /// `data` as raw signed 16-bit little-endian mono PCM at `sample_rate`.
    pub async fn to_pcm_s16le(
        &self,
        data: &[u8],
        input: AudioInput,
        sample_rate: u32,
    ) -> anyhow::Result<Vec<u8>> {
        let (format, extension) = match input {
            AudioInput::Encoded(format) => (format, format.extension()),
            AudioInput::Pcm { .. } => (AudioFormat::Wav, "pcm"),
        };
        self.require(format).await?;
        let dir = tempfile::tempdir().context("Failed to create a temp dir")?;
        let input_path = write_input(dir.path(), data, extension).await?;
        let output = dir.path().join("out.pcm");
        let rate = sample_rate.to_string();
        let mut args: Vec<&std::ffi::OsStr> = Vec::new();
        let source_rate;
        if let AudioInput::Pcm { sample_rate } = input {
            source_rate = sample_rate.to_string();
            args.extend(
                ["-f", "s16le", "-ar", source_rate.as_str(), "-ac", "1"].map(std::ffi::OsStr::new),
            );
        }
        args.extend([std::ffi::OsStr::new("-i"), input_path.as_os_str()]);
        args.extend(
            [
                "-vn",
                "-f",
                "s16le",
                "-acodec",
                "pcm_s16le",
                "-ac",
                "1",
                "-ar",
                rate.as_str(),
            ]
            .map(std::ffi::OsStr::new),
        );
        args.push(output.as_os_str());
        self.ffmpeg(&args).await?;
        read_output(&output).await
    }

    async fn ffmpeg(&self, args: &[&std::ffi::OsStr]) -> anyhow::Result<()> {
        let output = Command::new(&self.ffmpeg)
            .args(["-loglevel", "error", "-y"])
            .args(args)
            .stdin(Stdio::null())
            .output()
            .await
            .with_context(|| format!("Failed to run ffmpeg at {}", self.ffmpeg))?;
        if !output.status.success() {
            anyhow::bail!(
                "ffmpeg failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }
}

async fn write_input(dir: &Path, data: &[u8], extension: &str) -> anyhow::Result<PathBuf> {
    if data.is_empty() {
        anyhow::bail!("Cannot process empty audio");
    }
    let path = dir.join(format!("in.{extension}"));
    tokio::fs::write(&path, data)
        .await
        .context("Failed to write audio to a temp file")?;
    Ok(path)
}

async fn read_output(path: &Path) -> anyhow::Result<Vec<u8>> {
    tokio::fs::read(path)
        .await
        .with_context(|| format!("Failed to read ffmpeg output {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::Duration;

    use super::{AudioInput, AudioPipeline};
    use crate::converter::AudioFormat;

    /// Media made by ffmpeg's lavfi sources, or `None` without ffmpeg.
    fn generate(args: &[&str], format: &str) -> Option<Vec<u8>> {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join(format!("out.{format}"));
        let status = Command::new("ffmpeg")
            .args(["-loglevel", "error", "-y"])
            .args(args)
            .arg(&out)
            .status()
            .ok()?;
        status.success().then(|| std::fs::read(out).unwrap())
    }

    fn tone(secs: u32, format: &str) -> Option<Vec<u8>> {
        let source = format!("sine=frequency=440:duration={secs}");
        generate(&["-f", "lavfi", "-i", &source], format)
    }

    fn wav_header(wav: &[u8]) -> (u16, u32) {
        assert_eq!(&wav[..4], b"RIFF", "a WAV file");
        let channels = u16::from_le_bytes([wav[22], wav[23]]);
        let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        (channels, rate)
    }

    macro_rules! or_skip {
        ($media:expr) => {
            match $media {
                Some(media) => media,
                None => {
                    eprintln!("skipped: ffmpeg is not installed");
                    return;
                }
            }
        };
    }

    #[tokio::test]
    async fn probes_the_duration() {
        let wav = or_skip!(tone(3, "wav"));
        let duration = AudioPipeline::default()
            .probe_duration(&wav, AudioFormat::Wav)
            .await
            .unwrap()
            .unwrap();
        assert!((duration.as_secs_f64() - 3.0).abs() < 0.1, "{duration:?}");
    }

    #[tokio::test]
    async fn converts_mp3_to_16k_mono_wav() {
        let mp3 = or_skip!(generate(
            &[
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-ac",
                "2",
                "-ar",
                "44100"
            ],
            "mp3"
        ));
        let wav = AudioPipeline::default()
            .to_wav_16k_mono(&mp3, AudioFormat::Mp3)
            .await
            .unwrap();
        assert_eq!(wav_header(&wav), (1, 16_000));
    }

    #[tokio::test]
    async fn extracts_the_audio_of_a_video() {
        let mp4 = or_skip!(generate(
            &[
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=1:size=64x64:rate=10",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-shortest",
                "-pix_fmt",
                "yuv420p",
            ],
            "mp4"
        ));
        let wav = AudioPipeline::default()
            .to_wav_16k_mono(&mp4, AudioFormat::Mp4)
            .await
            .unwrap();
        assert_eq!(wav_header(&wav), (1, 16_000));
    }

    #[tokio::test]
    async fn splits_into_ordered_pieces() {
        let wav = or_skip!(tone(7, "wav"));
        let pipeline = AudioPipeline::default();
        let pieces = pipeline.split(&wav, 3).await.unwrap();
        assert_eq!(pieces.len(), 3);
        let mut total = Duration::ZERO;
        for piece in &pieces {
            total += pipeline
                .probe_duration(piece, AudioFormat::Wav)
                .await
                .unwrap()
                .unwrap();
        }
        assert!((total.as_secs_f64() - 7.0).abs() < 0.2, "{total:?}");
    }

    #[tokio::test]
    async fn transcodes_pcm_to_mp3() {
        let pcm = or_skip!(generate(
            &[
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-ar",
                "24000",
                "-ac",
                "1",
                "-f",
                "s16le"
            ],
            "pcm"
        ));
        let mp3 = AudioPipeline::default()
            .transcode(
                &pcm,
                AudioInput::Pcm {
                    sample_rate: 24_000,
                },
                AudioFormat::Mp3,
            )
            .await
            .unwrap();
        assert!(
            mp3.starts_with(b"ID3") || (mp3[0] == 0xFF && mp3[1] & 0xE0 == 0xE0),
            "an MP3 stream"
        );
    }

    #[tokio::test]
    async fn transcodes_wav_to_ogg_opus() {
        let wav = or_skip!(tone(1, "wav"));
        let ogg = AudioPipeline::default()
            .transcode(
                &wav,
                AudioInput::Encoded(AudioFormat::Wav),
                AudioFormat::Ogg,
            )
            .await
            .unwrap();
        assert_eq!(&ogg[..4], b"OggS");
        assert!(
            ogg.windows(8).any(|w| w == b"OpusHead"),
            "the Ogg stream carries Opus"
        );
    }

    #[tokio::test]
    async fn transcodes_mp3_to_wav() {
        let mp3 = or_skip!(tone(1, "mp3"));
        let wav = AudioPipeline::default()
            .transcode(
                &mp3,
                AudioInput::Encoded(AudioFormat::Mp3),
                AudioFormat::Wav,
            )
            .await
            .unwrap();
        assert_eq!(&wav[..4], b"RIFF");
    }

    #[tokio::test]
    async fn missing_ffmpeg_names_ffmpeg_and_the_format() {
        let err = AudioPipeline::default()
            .with_ffmpeg_path("/nonexistent/ffmpeg")
            .require(AudioFormat::Caf)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("ffmpeg") && err.contains("caf"), "{err}");
    }
}
