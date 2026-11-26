//! Audio formats and the ffmpeg-based pipeline shared by transcription,
//! speech and chat providers.

mod format;
mod pipeline;

pub use format::AudioFormat;
pub use pipeline::{AudioInput, AudioPipeline};

/// `mime` lowercased, without parameters such as `; codecs=opus`.
pub fn essence(mime: &str) -> String {
    mime.split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase()
}
