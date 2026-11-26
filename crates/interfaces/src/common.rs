//! Shared helpers used across multiple interface adapters.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use assistant_core::clock::{Clock, SystemClock};
use assistant_core::{ChannelContent, VOICE_PLACEHOLDER};
use assistant_transcription::{TranscriptionProvider, TranscriptionRequest};
use lru::LruCache;
use tokio::sync::watch;
use tracing::{error, warn};

/// Minimum reconnect delay.
pub const BACKOFF_MIN: Duration = Duration::from_secs(1);
/// Maximum reconnect delay.
pub const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Maximum entries in the per-adapter maps that track in-flight state
/// (pending reactions, seeded threads). Entries that never see their matching
/// `on_turn_start` are evicted least-recently-used first.
pub const PENDING_CAPACITY: usize = 1024;

const PENDING_CAPACITY_NZ: NonZeroUsize = match NonZeroUsize::new(PENDING_CAPACITY) {
    Some(n) => n,
    None => unreachable!(),
};

/// A string-keyed LRU cache sized to [`PENDING_CAPACITY`].
pub fn pending_cache<V>() -> LruCache<String, V> {
    LruCache::new(PENDING_CAPACITY_NZ)
}

/// Collects the recordings of one inbound message and the text that stands
/// for them.
///
/// The first recording is passed into the turn. Every recording is
/// transcribed when a provider is configured. A missing provider, a failed
/// transcription and an empty one all leave the recording in place and
/// [`VOICE_PLACEHOLDER`] as its text.
pub(crate) struct VoiceCapture<'a> {
    provider: Option<&'a dyn TranscriptionProvider>,
    language: Option<&'a str>,
    platform: &'static str,
    label: &'a str,
    recording: Option<(Vec<u8>, String)>,
    transcripts: Vec<String>,
}

impl<'a> VoiceCapture<'a> {
    pub(crate) fn new(
        provider: Option<&'a dyn TranscriptionProvider>,
        language: Option<&'a str>,
        platform: &'static str,
    ) -> Self {
        Self {
            provider,
            language,
            platform,
            label: VOICE_PLACEHOLDER,
            recording: None,
            transcripts: Vec::new(),
        }
    }

    /// Prefix the transcript with `label` instead of [`VOICE_PLACEHOLDER`].
    pub(crate) fn with_label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    /// Add one recording that already passed the adapter's size limit.
    pub(crate) async fn add(&mut self, data: Vec<u8>, mime_type: &str, filename: Option<&str>) {
        if let Some(provider) = self.provider {
            let request = TranscriptionRequest {
                audio_data: data.clone(),
                mime_type: mime_type.to_string(),
                filename: filename.map(str::to_string),
                language: self.language.map(str::to_string),
            };
            match provider.transcribe(request).await {
                Ok(result) if !result.text.trim().is_empty() => {
                    self.transcripts.push(result.text);
                }
                Ok(_) => warn!(platform = self.platform, "audio transcription was empty"),
                Err(e) => warn!(platform = self.platform, error = %e, "audio transcription failed"),
            }
        }
        if self.recording.is_none() {
            self.recording = Some((data, mime_type.to_string()));
        }
    }

    /// The voice message, or `None` when no recording was added.
    ///
    /// The text is `[Voice message]: transcript` (see [`with_label`](Self::with_label)), or [`VOICE_PLACEHOLDER`] without a
    /// transcript, followed by the sender's own `typed` text if any.
    pub(crate) fn into_content(self, typed: &str) -> Option<ChannelContent> {
        let (data, mime_type) = self.recording?;
        let spoken = if self.transcripts.is_empty() {
            VOICE_PLACEHOLDER.to_string()
        } else {
            format!("{}: {}", self.label, self.transcripts.join(" "))
        };
        let text = if typed.is_empty() {
            spoken
        } else {
            format!("{spoken}\n{typed}")
        };
        Some(ChannelContent::Voice {
            data,
            mime_type,
            text,
        })
    }
}

/// Waits out one backoff interval. Abstracted so tests can record delays
/// instead of sleeping.
#[async_trait::async_trait]
pub(crate) trait Pause: Send + Sync {
    /// Wait roughly `base`, returning early if `stop_rx` fires.
    async fn pause(&self, base: Duration, stop_rx: &mut watch::Receiver<bool>);
}

/// Production [`Pause`]: sleeps `base` ±10 % jitter on the tokio timer.
pub(crate) struct JitteredSleep;

#[async_trait::async_trait]
impl Pause for JitteredSleep {
    async fn pause(&self, base: Duration, stop_rx: &mut watch::Receiver<bool>) {
        tokio::select! {
            _ = tokio::time::sleep(jittered(base)) => {}
            _ = stop_rx.changed() => {}
        }
    }
}

/// `base` scaled by a random factor in `[0.9, 1.1]`, never below 100 ms.
fn jittered(base: Duration) -> Duration {
    let jitter_pct = (rand_jitter() - 0.5) * 0.2;
    Duration::from_secs_f64((base.as_secs_f64() * (1.0 + jitter_pct)).max(0.1))
}

/// Reconnect policy shared by every long-lived adapter connection.
///
/// Exponential backoff from [`BACKOFF_MIN`] to [`BACKOFF_MAX`] with jitter,
/// reset by [`connected`](Self::connected), cancelled by the stop signal.
/// The adapter keeps its own session loop and consults this at the three
/// decision points: before an attempt, after a failed connect, and after a
/// session ends.
pub(crate) struct Reconnect {
    backoff: Duration,
    stop_rx: watch::Receiver<bool>,
    pause: Arc<dyn Pause>,
}

impl Reconnect {
    pub(crate) fn new(stop_rx: watch::Receiver<bool>) -> Self {
        Self::with_pause(stop_rx, Arc::new(JitteredSleep))
    }

    fn with_pause(stop_rx: watch::Receiver<bool>, pause: Arc<dyn Pause>) -> Self {
        Self {
            backoff: BACKOFF_MIN,
            stop_rx,
            pause,
        }
    }

    /// `true` once shutdown has been requested.
    pub(crate) fn should_stop(&self) -> bool {
        *self.stop_rx.borrow()
    }

    /// A connection (or sync) succeeded: the next failure starts from
    /// [`BACKOFF_MIN`] again.
    pub(crate) fn connected(&mut self) {
        self.backoff = BACKOFF_MIN;
    }

    /// Wait out the current backoff, then double it (capped at
    /// [`BACKOFF_MAX`]). Returns early if shutdown is requested.
    pub(crate) async fn backoff(&mut self) {
        self.pause.pause(self.backoff, &mut self.stop_rx).await;
        self.backoff = (self.backoff * 2).min(BACKOFF_MAX);
    }

    /// Resolve a connect attempt: on success reset the backoff and return the
    /// connection; on failure log `what`, back off, and return `None` so the
    /// caller can `continue`.
    pub(crate) async fn connect_or_backoff<T, E: std::fmt::Display>(
        &mut self,
        result: Result<T, E>,
        what: &str,
    ) -> Option<T> {
        match result {
            Ok(conn) => {
                self.connected();
                Some(conn)
            }
            Err(e) => {
                error!(error = %e, "{what}; retrying");
                self.backoff().await;
                None
            }
        }
    }
}

/// Simple LCG-based float in `[0, 1)` without pulling in `rand`.
pub fn rand_jitter() -> f64 {
    // Use wall-clock subsec-nanos as the seed source. This is sufficient
    // randomness for jitter; security-sensitive randomness should use
    // `getrandom` instead.
    let seed = SystemClock.now().timestamp_subsec_nanos() as u64;
    let v = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (v >> 32) as f64 / ((1u64 << 32) as f64)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex as StdMutex};

    use super::*;
    use crate::common::test_support::StubTranscriber;

    /// Records the base delay of every pause instead of sleeping.
    #[derive(Default)]
    struct RecordingPause(StdMutex<Vec<Duration>>);

    #[async_trait::async_trait]
    impl Pause for RecordingPause {
        async fn pause(&self, base: Duration, _stop_rx: &mut watch::Receiver<bool>) {
            self.0.lock().unwrap_or_else(|e| e.into_inner()).push(base);
        }
    }

    impl RecordingPause {
        fn delays(&self) -> Vec<Duration> {
            self.0.lock().unwrap().clone()
        }
    }

    fn reconnect() -> (Reconnect, Arc<RecordingPause>, watch::Sender<bool>) {
        let (stop_tx, stop_rx) = watch::channel(false);
        let pause = Arc::new(RecordingPause::default());
        (
            Reconnect::with_pause(stop_rx, pause.clone()),
            pause,
            stop_tx,
        )
    }

    #[test]
    fn pending_cache_evicts_least_recently_used_beyond_capacity() {
        let mut cache = pending_cache::<usize>();
        for i in 0..PENDING_CAPACITY + 5 {
            cache.put(format!("k{i}"), i);
        }
        assert_eq!(cache.len(), PENDING_CAPACITY, "never exceeds capacity");
        assert!(cache.peek("k0").is_none(), "oldest entry evicted");
        assert!(
            cache.peek(&format!("k{}", PENDING_CAPACITY + 4)).is_some(),
            "newest entry kept"
        );
    }

    #[tokio::test]
    async fn backoff_doubles_each_failure_up_to_the_cap() {
        let (mut reconnect, pause, _stop) = reconnect();
        for _ in 0..8 {
            reconnect.backoff().await;
        }
        let secs: Vec<u64> = pause.delays().iter().map(Duration::as_secs).collect();
        assert_eq!(secs, vec![1, 2, 4, 8, 16, 32, 60, 60]);
    }

    #[tokio::test]
    async fn connected_resets_the_backoff() {
        let (mut reconnect, pause, _stop) = reconnect();
        reconnect.backoff().await;
        reconnect.backoff().await;
        reconnect.connected();
        reconnect.backoff().await;
        let secs: Vec<u64> = pause.delays().iter().map(Duration::as_secs).collect();
        assert_eq!(secs, vec![1, 2, 1]);
    }

    #[tokio::test]
    async fn connect_or_backoff_retries_on_error_and_resets_on_success() {
        let (mut reconnect, pause, _stop) = reconnect();

        let failed: Option<u8> = reconnect
            .connect_or_backoff(Err::<u8, _>("refused"), "test connect")
            .await;
        assert_eq!(failed, None, "an error yields no connection");
        assert_eq!(pause.delays().len(), 1, "an error backs off once");

        let ok = reconnect.connect_or_backoff(Ok::<_, &str>(7), "test").await;
        assert_eq!(ok, Some(7));
        assert_eq!(pause.delays().len(), 1, "success does not back off");

        reconnect.backoff().await;
        assert_eq!(
            pause.delays().last(),
            Some(&BACKOFF_MIN),
            "success reset the delay"
        );
    }

    #[tokio::test]
    async fn should_stop_follows_the_stop_signal() {
        let (reconnect, _pause, stop_tx) = reconnect();
        assert!(!reconnect.should_stop());
        stop_tx.send(true).unwrap();
        assert!(reconnect.should_stop());
    }

    #[test]
    fn jittered_stays_within_ten_percent() {
        for _ in 0..50 {
            let d = jittered(Duration::from_secs(10)).as_secs_f64();
            assert!((9.0..=11.0).contains(&d), "{d} outside ±10%");
        }
    }

    #[tokio::test]
    async fn real_pause_returns_early_when_stop_fires() {
        let (stop_tx, mut stop_rx) = watch::channel(false);
        stop_tx.send(true).unwrap();
        let start = std::time::Instant::now();
        JitteredSleep.pause(BACKOFF_MAX, &mut stop_rx).await;
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "expected early cancel"
        );
    }

    // -- VoiceCapture --------------------------------------------------------

    fn ogg() -> (Vec<u8>, &'static str) {
        (b"OggS".to_vec(), "audio/ogg")
    }

    #[tokio::test]
    async fn capture_with_transcript_keeps_recording_and_text() {
        let provider = StubTranscriber::saying("hello");
        let mut capture = VoiceCapture::new(Some(&provider), Some("en"), "test");
        let (data, mime) = ogg();
        capture.add(data, mime, Some("voice.ogg")).await;
        match capture.into_content("").unwrap() {
            ChannelContent::Voice {
                data,
                mime_type,
                text,
            } => {
                assert_eq!(data, b"OggS");
                assert_eq!(mime_type, "audio/ogg");
                assert_eq!(text, "[Voice message]: hello");
            }
            other => panic!("expected Voice, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn capture_without_provider_uses_the_placeholder() {
        let mut capture = VoiceCapture::new(None, None, "test");
        let (data, mime) = ogg();
        capture.add(data, mime, None).await;
        match capture.into_content("").unwrap() {
            ChannelContent::Voice { data, text, .. } => {
                assert_eq!(data, b"OggS");
                assert_eq!(text, "[Voice message]");
            }
            other => panic!("expected Voice, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn capture_falls_back_to_placeholder_when_transcription_fails_or_is_empty() {
        for provider in [StubTranscriber::failing(), StubTranscriber::saying("  ")] {
            let mut capture = VoiceCapture::new(Some(&provider), None, "test");
            let (data, mime) = ogg();
            capture.add(data, mime, None).await;
            match capture.into_content("").unwrap() {
                ChannelContent::Voice { text, .. } => assert_eq!(text, "[Voice message]"),
                other => panic!("expected Voice, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn capture_puts_typed_text_after_the_voice_text() {
        let provider = StubTranscriber::saying("hello");
        let mut capture = VoiceCapture::new(Some(&provider), None, "test");
        let (data, mime) = ogg();
        capture.add(data, mime, None).await;
        match capture.into_content("see attached").unwrap() {
            ChannelContent::Voice { text, .. } => {
                assert_eq!(text, "[Voice message]: hello\nsee attached")
            }
            other => panic!("expected Voice, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn capture_keeps_the_first_recording_and_every_transcript() {
        let provider = StubTranscriber::saying("hi");
        let mut capture = VoiceCapture::new(Some(&provider), None, "test");
        capture.add(b"one".to_vec(), "audio/ogg", None).await;
        capture.add(b"two".to_vec(), "audio/mpeg", None).await;
        match capture.into_content("").unwrap() {
            ChannelContent::Voice {
                data,
                mime_type,
                text,
            } => {
                assert_eq!(data, b"one");
                assert_eq!(mime_type, "audio/ogg");
                assert_eq!(text, "[Voice message]: hi hi");
            }
            other => panic!("expected Voice, got {other:?}"),
        }
    }

    #[test]
    fn capture_without_a_recording_yields_nothing() {
        let capture = VoiceCapture::new(None, None, "test");
        assert!(capture.into_content("typed").is_none());
    }

    #[test]
    fn rand_jitter_in_unit_interval() {
        for _ in 0..50 {
            let v = rand_jitter();
            assert!((0.0..1.0).contains(&v), "{v} not in [0,1)");
        }
    }
}

/// Transcription stubs shared by the adapter tests.
#[cfg(test)]
pub(crate) mod test_support {
    use assistant_transcription::{
        TranscriptionProvider, TranscriptionRequest, TranscriptionResult,
    };
    use async_trait::async_trait;

    /// Answers every request with a fixed transcript, or fails when it has none.
    pub(crate) struct StubTranscriber(Option<&'static str>);

    impl StubTranscriber {
        pub(crate) fn saying(text: &'static str) -> Self {
            Self(Some(text))
        }

        pub(crate) fn failing() -> Self {
            Self(None)
        }
    }

    #[async_trait]
    impl TranscriptionProvider for StubTranscriber {
        fn name(&self) -> &str {
            "stub"
        }

        async fn transcribe(
            &self,
            _request: TranscriptionRequest,
        ) -> anyhow::Result<TranscriptionResult> {
            match self.0 {
                Some(text) => Ok(TranscriptionResult {
                    text: text.to_string(),
                    language: None,
                    duration_secs: None,
                }),
                None => anyhow::bail!("stub transcription failure"),
            }
        }
    }
}
