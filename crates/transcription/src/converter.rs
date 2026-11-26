//! Audio formats recognized for transcription.

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
    /// AMR narrow- or wide-band voice notes
    Amr,
    /// 3GPP / 3GPP2 container
    ThreeGpp,
    /// Apple Core Audio Format
    Caf,
    /// AIFF audio
    Aiff,
    /// QuickTime video container
    Mov,
    /// Unknown/other format
    Unknown,
}

impl AudioFormat {
    /// Detect audio format from MIME type.
    pub fn from_mime(mime: &str) -> Self {
        match crate::essence(mime).as_str() {
            "audio/mp4" | "audio/m4a" | "audio/x-m4a" => Self::M4a,
            "video/mp4" => Self::Mp4,
            "audio/aac" => Self::Aac,
            "audio/mpeg" | "audio/mp3" => Self::Mp3,
            "audio/wav" | "audio/x-wav" | "audio/wave" => Self::Wav,
            "audio/ogg" | "audio/opus" => Self::Ogg,
            "audio/flac" | "audio/x-flac" => Self::Flac,
            "audio/webm" | "video/webm" => Self::Webm,
            "audio/amr" | "audio/amr-wb" => Self::Amr,
            "audio/3gpp" | "audio/3gpp2" => Self::ThreeGpp,
            "audio/x-caf" => Self::Caf,
            "audio/aiff" | "audio/x-aiff" => Self::Aiff,
            "video/quicktime" => Self::Mov,
            _ => Self::Unknown,
        }
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
            Self::Amr => "amr",
            Self::ThreeGpp => "3gp",
            Self::Caf => "caf",
            Self::Aiff => "aiff",
            Self::Mov => "mov",
            Self::Unknown => "bin",
        }
    }
}

/// Formats OpenAI's `/audio/transcriptions` API accepts, which Whisper and
/// Ollama servers share.
pub const OPENAI_COMPATIBLE_FORMATS: &[AudioFormat] = &[
    AudioFormat::Flac,
    AudioFormat::M4a,
    AudioFormat::Mp3,
    AudioFormat::Mp4,
    AudioFormat::Ogg,
    AudioFormat::Wav,
    AudioFormat::Webm,
];

#[cfg(test)]
mod tests {
    use super::AudioFormat;

    #[test]
    fn from_mime_ignores_parameters_and_knows_new_formats() {
        assert_eq!(
            AudioFormat::from_mime("audio/ogg; codecs=opus"),
            AudioFormat::Ogg
        );
        assert_eq!(AudioFormat::from_mime("AUDIO/MPEG"), AudioFormat::Mp3);
        assert_eq!(AudioFormat::from_mime("audio/amr"), AudioFormat::Amr);
        assert_eq!(AudioFormat::from_mime("audio/amr-wb"), AudioFormat::Amr);
        assert_eq!(AudioFormat::from_mime("audio/3gpp"), AudioFormat::ThreeGpp);
        assert_eq!(AudioFormat::from_mime("audio/x-caf"), AudioFormat::Caf);
        assert_eq!(AudioFormat::from_mime("audio/x-aiff"), AudioFormat::Aiff);
        assert_eq!(AudioFormat::from_mime("audio/wave"), AudioFormat::Wav);
        assert_eq!(AudioFormat::from_mime("video/quicktime"), AudioFormat::Mov);
        assert_eq!(AudioFormat::from_mime("video/webm"), AudioFormat::Webm);
    }
}
