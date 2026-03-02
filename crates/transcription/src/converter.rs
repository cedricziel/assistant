//! Audio format conversion for transcription providers.
//!
//! Some transcription providers (e.g., Deepgram) don't support certain audio
//! formats like M4A/MP4. This module provides FFmpeg-based conversion to
//! supported formats (WAV/MP3).

use std::process::Stdio;

use anyhow::{bail, Context};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Audio formats that may need conversion before sending to transcription providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// M4A/AAC format (MPEG-4 audio)
    M4a,
    /// MP4 container with audio
    Mp4,
    /// Raw AAC audio
    Aac,
    /// MP3/MPEG audio (usually supported natively)
    Mp3,
    /// WAV audio (usually supported natively)
    Wav,
    /// OGG/Opus audio (usually supported natively)
    Ogg,
    /// FLAC audio (usually supported natively)
    Flac,
    /// WebM audio (usually supported natively)
    Webm,
    /// Unknown/other format
    Unknown,
}

impl AudioFormat {
    /// Detect audio format from MIME type.
    pub fn from_mime(mime: &str) -> Self {
        match mime.to_lowercase().as_str() {
            "audio/mp4" | "audio/m4a" | "audio/x-m4a" => Self::M4a,
            "video/mp4" => Self::Mp4,
            "audio/aac" => Self::Aac,
            "audio/mpeg" | "audio/mp3" => Self::Mp3,
            "audio/wav" | "audio/x-wav" => Self::Wav,
            "audio/ogg" | "audio/opus" => Self::Ogg,
            "audio/flac" | "audio/x-flac" => Self::Flac,
            "audio/webm" => Self::Webm,
            _ => Self::Unknown,
        }
    }

    /// Returns true if this format typically needs conversion for Deepgram.
    pub fn needs_conversion_for_deepgram(&self) -> bool {
        matches!(self, Self::M4a | Self::Mp4 | Self::Aac)
    }

    /// Returns true if this format is known to be supported by Deepgram natively.
    pub fn is_supported_by_deepgram(&self) -> bool {
        matches!(
            self,
            Self::Mp3 | Self::Wav | Self::Ogg | Self::Flac | Self::Webm
        )
    }

    /// Get the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::M4a => "m4a",
            Self::Mp4 => "mp4",
            Self::Aac => "aac",
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Ogg => "ogg",
            Self::Flac => "flac",
            Self::Webm => "webm",
            Self::Unknown => "bin",
        }
    }
}

/// Result of a conversion operation.
#[derive(Debug, Clone)]
pub struct ConversionResult {
    /// The converted audio data.
    pub data: Vec<u8>,
    /// The new MIME type.
    pub mime_type: String,
    /// The original format.
    pub original_format: AudioFormat,
    /// The target format.
    pub target_format: AudioFormat,
}

/// Audio converter using FFmpeg.
#[derive(Debug, Clone)]
pub struct AudioConverter {
    /// Path to ffmpeg binary (default: "ffmpeg").
    ffmpeg_path: String,
    /// Target format for conversion (default: WAV).
    target_format: AudioFormat,
    /// Target sample rate in Hz (default: 16000 for speech recognition).
    sample_rate: u32,
    /// Target number of channels (default: 1 for mono).
    channels: u8,
}

impl Default for AudioConverter {
    fn default() -> Self {
        Self {
            ffmpeg_path: "ffmpeg".to_string(),
            target_format: AudioFormat::Wav,
            sample_rate: 16000,
            channels: 1,
        }
    }
}

impl AudioConverter {
    /// Create a new audio converter with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a custom path to the ffmpeg binary.
    pub fn with_ffmpeg_path(mut self, path: impl Into<String>) -> Self {
        self.ffmpeg_path = path.into();
        self
    }

    /// Set the target format for conversion.
    pub fn with_target_format(mut self, format: AudioFormat) -> Self {
        self.target_format = format;
        self
    }

    /// Set the target sample rate (default: 16000 Hz).
    pub fn with_sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Set the target number of channels (default: 1 for mono).
    pub fn with_channels(mut self, channels: u8) -> Self {
        self.channels = channels;
        self
    }

    /// Check if FFmpeg is available and working.
    pub async fn check_ffmpeg(&self) -> anyhow::Result<()> {
        let output = Command::new(&self.ffmpeg_path)
            .arg("-version")
            .output()
            .await
            .context("Failed to run FFmpeg. Is FFmpeg installed and in PATH?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("FFmpeg check failed: {}", stderr);
        }

        debug!(
            ffmpeg_path = %self.ffmpeg_path,
            version = %String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("unknown"),
            "FFmpeg is available"
        );

        Ok(())
    }

    /// Returns true if this source format requires a seekable input (temp file)
    /// rather than a pipe.
    ///
    /// MP4/M4A containers store the `moov` atom (metadata) at the end of the
    /// file, so FFmpeg must seek backwards to parse them -- something that
    /// `pipe:0` (stdin) does not support.
    fn needs_seekable_input(format: AudioFormat) -> bool {
        matches!(format, AudioFormat::M4a | AudioFormat::Mp4)
    }

    /// Convert audio data to the target format.
    ///
    /// For most formats, conversion is done entirely in-memory via stdin/stdout
    /// pipes.  MP4/M4A containers, however, require seekable input (the `moov`
    /// atom may sit at the end of the file) so a temporary file is used instead.
    pub async fn convert(
        &self,
        audio_data: &[u8],
        source_format: AudioFormat,
    ) -> anyhow::Result<ConversionResult> {
        if audio_data.is_empty() {
            bail!("Cannot convert empty audio data");
        }

        // If the format is already the target, just return as-is
        if source_format == self.target_format {
            return Ok(ConversionResult {
                data: audio_data.to_vec(),
                mime_type: format!("audio/{}", self.target_format.extension()),
                original_format: source_format,
                target_format: self.target_format,
            });
        }

        debug!(
            source_format = ?source_format,
            target_format = ?self.target_format,
            input_size = audio_data.len(),
            "Converting audio format"
        );

        if Self::needs_seekable_input(source_format) {
            self.convert_via_tempfile(audio_data, source_format).await
        } else {
            self.convert_via_pipe(audio_data, source_format).await
        }
    }

    /// Target format as FFmpeg output format name and MIME type.
    fn output_format_info(&self) -> anyhow::Result<(&'static str, &'static str)> {
        match self.target_format {
            AudioFormat::Wav => Ok(("wav", "audio/wav")),
            AudioFormat::Mp3 => Ok(("mp3", "audio/mpeg")),
            AudioFormat::Ogg => Ok(("ogg", "audio/ogg")),
            AudioFormat::Flac => Ok(("flac", "audio/flac")),
            _ => bail!("Unsupported target format: {:?}", self.target_format),
        }
    }

    /// Convert audio by piping data through FFmpeg's stdin/stdout.
    async fn convert_via_pipe(
        &self,
        audio_data: &[u8],
        source_format: AudioFormat,
    ) -> anyhow::Result<ConversionResult> {
        let (output_format, mime_type) = self.output_format_info()?;

        let mut cmd = Command::new(&self.ffmpeg_path);
        cmd.arg("-i")
            .arg("pipe:0")
            .arg("-f")
            .arg(output_format)
            .arg("-ar")
            .arg(self.sample_rate.to_string())
            .arg("-ac")
            .arg(self.channels.to_string())
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("pipe:1");

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn FFmpeg at {}", self.ffmpeg_path))?;

        let stdin = child
            .stdin
            .take()
            .context("Failed to get FFmpeg stdin handle")?;

        let input_data = audio_data.to_vec();
        let write_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let mut stdin = stdin;
            if let Err(e) = stdin.write_all(&input_data).await {
                warn!(error = %e, "Failed to write to FFmpeg stdin");
            }
            let _ = stdin.shutdown().await;
        });

        let stdout = child
            .stdout
            .take()
            .context("Failed to get FFmpeg stdout handle")?;

        let stderr = child
            .stderr
            .take()
            .context("Failed to get FFmpeg stderr handle")?;

        let read_task = tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut stdout = stdout;
            let mut buffer = Vec::new();
            if let Err(e) = stdout.read_to_end(&mut buffer).await {
                warn!(error = %e, "Failed to read from FFmpeg stdout");
            }
            buffer
        });

        let stderr_task = tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut stderr = stderr;
            let mut buffer = Vec::new();
            let _ = stderr.read_to_end(&mut buffer).await;
            String::from_utf8_lossy(&buffer).to_string()
        });

        let (write_result, output_data, stderr_output, status) =
            tokio::join!(write_task, read_task, stderr_task, child.wait(),);

        if let Err(e) = write_result {
            bail!("FFmpeg stdin write task failed: {}", e);
        }

        let output_data =
            output_data.map_err(|e| anyhow::anyhow!("FFmpeg stdout read task failed: {}", e))?;

        let stderr_text = stderr_output.unwrap_or_default();

        let status = status.context("Failed to wait for FFmpeg process")?;

        if !status.success() {
            bail!(
                "FFmpeg conversion failed (exit code {:?}): {}",
                status.code(),
                stderr_text.trim(),
            );
        }

        if output_data.is_empty() {
            bail!("FFmpeg produced empty output");
        }

        self.log_success(audio_data.len(), output_data.len());

        Ok(ConversionResult {
            data: output_data,
            mime_type: mime_type.to_string(),
            original_format: source_format,
            target_format: self.target_format,
        })
    }

    /// Convert audio via a temporary file (required for MP4/M4A whose `moov`
    /// atom prevents pure-pipe processing).
    async fn convert_via_tempfile(
        &self,
        audio_data: &[u8],
        source_format: AudioFormat,
    ) -> anyhow::Result<ConversionResult> {
        let (output_format, mime_type) = self.output_format_info()?;

        // Write input to a temp file so FFmpeg can seek
        let tmp_dir = std::env::temp_dir();
        let input_path = tmp_dir.join(format!(
            "assistant-audio-in-{}.{}",
            std::process::id(),
            source_format.extension()
        ));
        let output_path = tmp_dir.join(format!(
            "assistant-audio-out-{}.{}",
            std::process::id(),
            self.target_format.extension()
        ));

        tokio::fs::write(&input_path, audio_data)
            .await
            .context("Failed to write audio to temp file")?;

        info!(
            input_path = %input_path.display(),
            input_size = audio_data.len(),
            source_format = ?source_format,
            "Using temp file for seekable MP4/M4A input"
        );

        let output = Command::new(&self.ffmpeg_path)
            .arg("-i")
            .arg(&input_path)
            .arg("-f")
            .arg(output_format)
            .arg("-ar")
            .arg(self.sample_rate.to_string())
            .arg("-ac")
            .arg(self.channels.to_string())
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg(&output_path)
            .output()
            .await
            .with_context(|| format!("Failed to spawn FFmpeg at {}", self.ffmpeg_path))?;

        // Always clean up the input file
        let _ = tokio::fs::remove_file(&input_path).await;

        if !output.status.success() {
            let _ = tokio::fs::remove_file(&output_path).await;
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "FFmpeg conversion failed (exit code {:?}): {}",
                output.status.code(),
                stderr.trim(),
            );
        }

        let output_data = tokio::fs::read(&output_path)
            .await
            .context("Failed to read FFmpeg output file")?;
        let _ = tokio::fs::remove_file(&output_path).await;

        if output_data.is_empty() {
            bail!("FFmpeg produced empty output");
        }

        self.log_success(audio_data.len(), output_data.len());

        Ok(ConversionResult {
            data: output_data,
            mime_type: mime_type.to_string(),
            original_format: source_format,
            target_format: self.target_format,
        })
    }

    fn log_success(&self, input_size: usize, output_size: usize) {
        debug!(input_size, output_size, "Audio conversion successful");
    }

    /// Convenience method to convert audio if needed for Deepgram.
    ///
    /// Returns the original data if no conversion is needed, otherwise converts.
    pub async fn convert_for_deepgram_if_needed(
        &self,
        audio_data: &[u8],
        mime_type: &str,
    ) -> anyhow::Result<(Vec<u8>, String)> {
        let format = AudioFormat::from_mime(mime_type);

        if !format.needs_conversion_for_deepgram() {
            debug!(
                format = ?format,
                mime_type = %mime_type,
                "No conversion needed for Deepgram"
            );
            return Ok((audio_data.to_vec(), mime_type.to_string()));
        }

        let result = self.convert(audio_data, format).await?;
        Ok((result.data, result.mime_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_from_mime() {
        assert_eq!(AudioFormat::from_mime("audio/mp4"), AudioFormat::M4a);
        assert_eq!(AudioFormat::from_mime("audio/m4a"), AudioFormat::M4a);
        assert_eq!(AudioFormat::from_mime("audio/x-m4a"), AudioFormat::M4a);
        assert_eq!(AudioFormat::from_mime("video/mp4"), AudioFormat::Mp4);
        assert_eq!(AudioFormat::from_mime("audio/aac"), AudioFormat::Aac);
        assert_eq!(AudioFormat::from_mime("audio/mpeg"), AudioFormat::Mp3);
        assert_eq!(AudioFormat::from_mime("audio/mp3"), AudioFormat::Mp3);
        assert_eq!(AudioFormat::from_mime("audio/wav"), AudioFormat::Wav);
        assert_eq!(AudioFormat::from_mime("audio/x-wav"), AudioFormat::Wav);
        assert_eq!(AudioFormat::from_mime("audio/ogg"), AudioFormat::Ogg);
        assert_eq!(AudioFormat::from_mime("audio/opus"), AudioFormat::Ogg);
        assert_eq!(AudioFormat::from_mime("audio/flac"), AudioFormat::Flac);
        assert_eq!(AudioFormat::from_mime("audio/webm"), AudioFormat::Webm);
        assert_eq!(
            AudioFormat::from_mime("audio/unknown"),
            AudioFormat::Unknown
        );
    }

    #[test]
    fn test_needs_conversion_for_deepgram() {
        assert!(AudioFormat::M4a.needs_conversion_for_deepgram());
        assert!(AudioFormat::Mp4.needs_conversion_for_deepgram());
        assert!(AudioFormat::Aac.needs_conversion_for_deepgram());
        assert!(!AudioFormat::Mp3.needs_conversion_for_deepgram());
        assert!(!AudioFormat::Wav.needs_conversion_for_deepgram());
        assert!(!AudioFormat::Ogg.needs_conversion_for_deepgram());
        assert!(!AudioFormat::Flac.needs_conversion_for_deepgram());
        assert!(!AudioFormat::Webm.needs_conversion_for_deepgram());
        assert!(!AudioFormat::Unknown.needs_conversion_for_deepgram());
    }

    #[test]
    fn test_is_supported_by_deepgram() {
        assert!(!AudioFormat::M4a.is_supported_by_deepgram());
        assert!(!AudioFormat::Mp4.is_supported_by_deepgram());
        assert!(!AudioFormat::Aac.is_supported_by_deepgram());
        assert!(AudioFormat::Mp3.is_supported_by_deepgram());
        assert!(AudioFormat::Wav.is_supported_by_deepgram());
        assert!(AudioFormat::Ogg.is_supported_by_deepgram());
        assert!(AudioFormat::Flac.is_supported_by_deepgram());
        assert!(AudioFormat::Webm.is_supported_by_deepgram());
        assert!(!AudioFormat::Unknown.is_supported_by_deepgram());
    }

    #[test]
    fn test_audio_format_extensions() {
        assert_eq!(AudioFormat::M4a.extension(), "m4a");
        assert_eq!(AudioFormat::Mp4.extension(), "mp4");
        assert_eq!(AudioFormat::Aac.extension(), "aac");
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Wav.extension(), "wav");
        assert_eq!(AudioFormat::Ogg.extension(), "ogg");
        assert_eq!(AudioFormat::Flac.extension(), "flac");
        assert_eq!(AudioFormat::Webm.extension(), "webm");
        assert_eq!(AudioFormat::Unknown.extension(), "bin");
    }

    #[tokio::test]
    async fn test_converter_default_settings() {
        let converter = AudioConverter::new();
        assert_eq!(converter.sample_rate, 16000);
        assert_eq!(converter.channels, 1);
        assert_eq!(converter.target_format, AudioFormat::Wav);
        assert_eq!(converter.ffmpeg_path, "ffmpeg");
    }

    #[tokio::test]
    async fn test_converter_builder_methods() {
        let converter = AudioConverter::new()
            .with_ffmpeg_path("/usr/bin/ffmpeg")
            .with_target_format(AudioFormat::Mp3)
            .with_sample_rate(44100)
            .with_channels(2);

        assert_eq!(converter.ffmpeg_path, "/usr/bin/ffmpeg");
        assert_eq!(converter.target_format, AudioFormat::Mp3);
        assert_eq!(converter.sample_rate, 44100);
        assert_eq!(converter.channels, 2);
    }

    #[tokio::test]
    async fn test_convert_empty_data() {
        let converter = AudioConverter::new();
        let result = converter.convert(&[], AudioFormat::M4a).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[tokio::test]
    async fn test_convert_same_format() {
        // When source and target are the same, data should pass through unchanged
        let converter = AudioConverter::new().with_target_format(AudioFormat::Wav);
        let test_data = vec![1, 2, 3, 4, 5];

        let result = converter
            .convert(&test_data, AudioFormat::Wav)
            .await
            .unwrap();

        assert_eq!(result.data, test_data);
        assert_eq!(result.mime_type, "audio/wav");
        assert_eq!(result.original_format, AudioFormat::Wav);
        assert_eq!(result.target_format, AudioFormat::Wav);
    }
}
