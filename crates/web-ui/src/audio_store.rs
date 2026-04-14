//! Re-export [`AudioStore`] from `assistant-transcription` for use within
//! the web-ui crate.
//!
//! The canonical implementation lives in `assistant_transcription::AudioStore`
//! so that `assistant-tool-executor` can also depend on it without creating a
//! circular dependency on `assistant-web-ui`.

pub use assistant_transcription::AudioStore;
