//! `ScriptedLlmProvider` — replays a queue of canned [`LlmResponse`] values.
//!
//! Use this for:
//! - Unit tests of upstream consumers (orchestrator, tool-executor) that
//!   don't need a real LLM backend.
//! - Offline / demo mode where the responses are baked in.
//! - Contract tests that need deterministic streaming behaviour.
//!
//! The provider does NOT require a network connection; it satisfies the
//! [`LlmProvider`] trait by popping responses from an internal queue. It
//! also records every call so tests can assert what the orchestrator
//! actually sent.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use assistant_core::{
    ChatHistoryMessage, FakeClock, LlmProvider, LlmResponse, LlmResponseMeta, StreamChunk,
    ToolSpec,
    llm::provider::{Capabilities, ToolSupport},
};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// A single recorded `chat` or `chat_streaming` call.
#[derive(Debug, Clone)]
pub struct RecordedCall {
    pub system_prompt: String,
    pub history: Vec<ChatHistoryMessage>,
    pub tools: Vec<ToolSpec>,
    /// `true` for `chat_streaming`, `false` for `chat`.
    pub streaming: bool,
}

/// One scripted `chat_streaming` call for
/// [`ScriptedLlmProvider::with_timed_streaming_calls`].
///
/// Each step advances the shared [`FakeClock`] by its duration, then sends
/// its chunk. After the last step the clock advances by
/// `advance_before_return` before the call returns, so tests can place the
/// end of the call at an exact offset from its start.
#[derive(Debug, Clone, Default)]
pub struct TimedCall {
    /// `(advance_before, chunk)` pairs, applied in order.
    pub steps: Vec<(Duration, StreamChunk)>,
    /// How far the clock moves after the last chunk, before the call returns.
    pub advance_before_return: Duration,
}

/// Internal state — wrapped in `Mutex` so the provider can be shared
/// across async tasks while still being mutable.
#[derive(Default)]
struct ScriptedData {
    responses: Vec<LlmResponse>,
    stream_chunks: Vec<StreamChunk>,
    embeddings: Vec<Vec<f32>>,
    recorded_calls: Vec<RecordedCall>,
    timed_calls: VecDeque<TimedCall>,
}

/// LLM provider that replays a fixed queue of canned responses.
///
/// Construct with [`ScriptedLlmProvider::new`], then optionally chain
/// [`with_canned_responses`](Self::with_canned_responses),
/// [`with_streaming_script`](Self::with_streaming_script), and
/// [`with_embeddings`](Self::with_embeddings).
///
/// ## Default behaviour
///
/// - When the response queue is empty, `chat` / `chat_streaming` return
///   a benign `FinalAnswer("(scripted: no more responses)", ...)`.
/// - When the streaming chunk queue is empty, `chat_streaming` emits a
///   single `StreamChunk::Text` with the final answer before returning.
/// - `embed` and `embed_batch` return queued vectors in order; an empty
///   queue yields a zero-length vector (so callers can detect "not
///   configured" without an explicit error).
pub struct ScriptedLlmProvider {
    state: Arc<Mutex<ScriptedData>>,
    clock: Option<FakeClock>,
    capabilities: Capabilities,
    provider_name: String,
    model_name: String,
}

impl ScriptedLlmProvider {
    /// Create an empty scripted provider with default capabilities.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ScriptedData::default())),
            clock: None,
            capabilities: Capabilities {
                tools: ToolSupport::Native,
                streaming: true,
                vision: false,
                hosted_tools: Vec::new(),
                context_window_tokens: None,
            },
            provider_name: "scripted".to_string(),
            model_name: "scripted-model".to_string(),
        }
    }

    /// Replace the queued chat responses. Calls consume from the front;
    /// when empty, a benign fallback is returned (see struct docs).
    pub fn with_canned_responses(self, responses: Vec<LlmResponse>) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.responses = responses;
        }
        self
    }

    /// Replace the queued streaming chunks. Each `chat_streaming` call
    /// consumes the entire queue before returning.
    pub fn with_streaming_script(self, chunks: Vec<StreamChunk>) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.stream_chunks = chunks;
        }
        self
    }

    /// Script `chat_streaming` calls with virtual timing.
    ///
    /// Each call consumes one [`TimedCall`] from the front, so successive
    /// calls play separate scripts. Once the queue is empty, calls fall back
    /// to [`with_streaming_script`](Self::with_streaming_script) behaviour.
    /// A timed call sends exactly its scripted chunks; it does not append the
    /// final answer text. The return value still comes from
    /// [`with_canned_responses`](Self::with_canned_responses).
    ///
    /// After sending each chunk the provider waits until the receiver has
    /// drained the channel before it advances `clock` again. A consumer that
    /// reads `clock.now_instant()` right after `recv()` therefore sees the
    /// scripted offsets exactly, provided the test runs on the current-thread
    /// runtime (the `#[tokio::test]` default).
    pub fn with_timed_streaming_calls(mut self, clock: FakeClock, calls: Vec<TimedCall>) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.timed_calls = calls.into();
        }
        self.clock = Some(clock);
        self
    }

    /// Replace the queued embedding vectors. `embed` consumes one per call.
    pub fn with_embeddings(self, embeddings: Vec<Vec<f32>>) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.embeddings = embeddings;
        }
        self
    }

    /// Override the reported provider name (default `"scripted"`).
    pub fn with_provider_name(mut self, name: impl Into<String>) -> Self {
        self.provider_name = name.into();
        self
    }

    /// Override the reported model name (default `"scripted-model"`).
    pub fn with_model_name(mut self, name: impl Into<String>) -> Self {
        self.model_name = name.into();
        self
    }

    /// Override the capability flags (defaults: tools=Native, streaming=true).
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Snapshot of every chat / chat_streaming call this provider has
    /// received, in order.
    pub fn recorded_calls(&self) -> Vec<RecordedCall> {
        match self.state.lock() {
            Ok(s) => s.recorded_calls.clone(),
            Err(p) => p.into_inner().recorded_calls.clone(),
        }
    }

    /// Number of recorded calls so far. Cheap accessor for assertions.
    pub fn call_count(&self) -> usize {
        match self.state.lock() {
            Ok(s) => s.recorded_calls.len(),
            Err(p) => p.into_inner().recorded_calls.len(),
        }
    }

    /// Pop the next timed script, paired with the clock it runs on.
    fn next_timed_call(&self) -> Option<(FakeClock, TimedCall)> {
        let clock = self.clock.clone()?;
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        state.timed_calls.pop_front().map(|call| (clock, call))
    }

    fn record_and_pop_response(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        streaming: bool,
    ) -> LlmResponse {
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        state.recorded_calls.push(RecordedCall {
            system_prompt: system_prompt.to_string(),
            history: history.to_vec(),
            tools: tools.to_vec(),
            streaming,
        });
        if state.responses.is_empty() {
            return LlmResponse::FinalAnswer(
                "(scripted: no more responses)".to_string(),
                LlmResponseMeta::default(),
            );
        }
        state.responses.remove(0)
    }
}

/// Play one [`TimedCall`]: advance the clock, send the chunk, and wait for
/// the receiver to drain the channel before the next advance.
async fn play_timed_call(
    clock: &FakeClock,
    call: TimedCall,
    chunk_sink: Option<mpsc::Sender<StreamChunk>>,
) {
    for (advance_before, chunk) in call.steps {
        clock.advance(advance_before);
        if let Some(sink) = &chunk_sink
            && sink.send(chunk).await.is_ok()
        {
            while !sink.is_closed() && sink.capacity() != sink.max_capacity() {
                tokio::task::yield_now().await;
            }
        }
    }
    clock.advance(call.advance_before_return);
}

impl Default for ScriptedLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for ScriptedLlmProvider {
    fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> Result<LlmResponse> {
        let response = self.record_and_pop_response(system_prompt, history, tools, false);
        // A non-streaming call plays its timed script too, without a sink,
        // so turns without a token sink can be timed on the fake clock.
        if let Some((clock, call)) = self.next_timed_call() {
            play_timed_call(&clock, call, None).await;
        }
        Ok(response)
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> Result<LlmResponse> {
        let response = self.record_and_pop_response(system_prompt, history, tools, true);

        if let Some((clock, call)) = self.next_timed_call() {
            play_timed_call(&clock, call, chunk_sink).await;
            return Ok(response);
        }

        // Drain the streaming script into the sink (best-effort — channel
        // send errors are silently ignored to mirror the production
        // provider behaviour when the downstream stream closes early).
        let chunks: Vec<StreamChunk> = {
            let mut state = match self.state.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            std::mem::take(&mut state.stream_chunks)
        };
        if let Some(sink) = chunk_sink {
            for chunk in chunks {
                let _ = sink.send(chunk).await;
            }
            // If no scripted chunks, emit the final answer text as a
            // single chunk so consumers that count chunks see at least
            // one event.
            if let LlmResponse::FinalAnswer(text, _) = &response {
                let _ = sink.send(StreamChunk::Text(text.clone())).await;
            }
        }
        Ok(response)
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if state.embeddings.is_empty() {
            return Ok(Vec::new());
        }
        Ok(state.embeddings.remove(0))
    }

    fn provider_name(&self) -> &str {
        &self.provider_name
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn server_address(&self) -> &str {
        "scripted://local"
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::Clock;

    use super::*;

    #[tokio::test]
    async fn empty_queue_returns_benign_final_answer() {
        let p = ScriptedLlmProvider::new();
        let resp = p.chat("sys", &[], &[]).await.unwrap();
        match resp {
            LlmResponse::FinalAnswer(text, _) => {
                assert!(text.contains("scripted"));
            }
            other => panic!("expected FinalAnswer, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn canned_responses_consumed_in_order() {
        let p = ScriptedLlmProvider::new().with_canned_responses(vec![
            LlmResponse::FinalAnswer("first".into(), LlmResponseMeta::default()),
            LlmResponse::FinalAnswer("second".into(), LlmResponseMeta::default()),
        ]);
        let r1 = p.chat("sys", &[], &[]).await.unwrap();
        let r2 = p.chat("sys", &[], &[]).await.unwrap();
        match (r1, r2) {
            (LlmResponse::FinalAnswer(a, _), LlmResponse::FinalAnswer(b, _)) => {
                assert_eq!(a, "first");
                assert_eq!(b, "second");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn recorded_calls_captures_system_prompt_and_history() {
        let p = ScriptedLlmProvider::new();
        let _ = p.chat("you are helpful", &[], &[]).await.unwrap();
        let calls = p.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].system_prompt, "you are helpful");
        assert!(!calls[0].streaming);
    }

    #[tokio::test]
    async fn streaming_emits_default_chunk_when_script_is_empty() {
        let p = ScriptedLlmProvider::new().with_canned_responses(vec![LlmResponse::FinalAnswer(
            "hello".into(),
            LlmResponseMeta::default(),
        )]);
        let (tx, mut rx) = mpsc::channel::<StreamChunk>(8);
        let _ = p.chat_streaming("sys", &[], &[], Some(tx)).await.unwrap();
        let chunk = rx.recv().await.expect("at least one chunk");
        match chunk {
            StreamChunk::Text(t) => assert_eq!(t, "hello"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn streaming_script_drains_into_sink() {
        let p = ScriptedLlmProvider::new()
            .with_canned_responses(vec![LlmResponse::FinalAnswer(
                "done".into(),
                LlmResponseMeta::default(),
            )])
            .with_streaming_script(vec![
                StreamChunk::Text("a".into()),
                StreamChunk::Text("b".into()),
            ]);
        let (tx, mut rx) = mpsc::channel::<StreamChunk>(8);
        let _ = p.chat_streaming("sys", &[], &[], Some(tx)).await.unwrap();
        let mut texts = Vec::new();
        while let Some(chunk) = rx.recv().await {
            if let StreamChunk::Text(t) = chunk {
                texts.push(t);
            }
        }
        assert_eq!(texts, vec!["a", "b", "done"]);
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn text_step(advance: u64, text: &str) -> (Duration, StreamChunk) {
        (ms(advance), StreamChunk::Text(text.into()))
    }

    fn final_answer(text: &str) -> LlmResponse {
        LlmResponse::FinalAnswer(text.into(), LlmResponseMeta::default())
    }

    /// Spawns a consumer that stamps `clock.now_instant()` right after each `recv()`.
    fn spawn_stamping_consumer(
        clock: FakeClock,
        mut rx: mpsc::Receiver<StreamChunk>,
    ) -> tokio::task::JoinHandle<Vec<(std::time::Instant, String)>> {
        tokio::spawn(async move {
            let mut seen = Vec::new();
            while let Some(chunk) = rx.recv().await {
                let at = clock.now_instant();
                if let StreamChunk::Text(t) = chunk {
                    seen.push((at, t));
                }
            }
            seen
        })
    }

    #[tokio::test]
    async fn timed_call_chunks_arrive_at_scripted_offsets() {
        let clock = FakeClock::new_at_seed();
        let p = ScriptedLlmProvider::new()
            .with_canned_responses(vec![final_answer("ab")])
            .with_timed_streaming_calls(
                clock.clone(),
                vec![TimedCall {
                    steps: vec![text_step(200, "a"), text_step(1300, "b")],
                    advance_before_return: ms(500),
                }],
            );
        let (tx, rx) = mpsc::channel::<StreamChunk>(64);
        let consumer = spawn_stamping_consumer(clock.clone(), rx);

        let start = clock.now_instant();
        let resp = p.chat_streaming("sys", &[], &[], Some(tx)).await.unwrap();
        let end = clock.now_instant();
        let seen = consumer.await.unwrap();

        let offsets: Vec<Duration> = seen.iter().map(|(at, _)| *at - start).collect();
        assert_eq!(
            offsets,
            vec![ms(200), ms(1500)],
            "each chunk is observed at its scripted offset from the call start"
        );
        assert_eq!(
            seen.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"],
            "a timed call sends only its scripted chunks"
        );
        assert_eq!(
            end - start,
            ms(2000),
            "the call returns after advance_before_return"
        );
        assert!(
            matches!(resp, LlmResponse::FinalAnswer(ref t, _) if t == "ab"),
            "the return value comes from the canned responses"
        );
    }

    #[tokio::test]
    async fn timed_calls_consume_separate_scripts_in_order() {
        let clock = FakeClock::new_at_seed();
        let p = ScriptedLlmProvider::new()
            .with_canned_responses(vec![final_answer("one"), final_answer("two")])
            .with_timed_streaming_calls(
                clock.clone(),
                vec![
                    TimedCall {
                        steps: vec![text_step(100, "first")],
                        advance_before_return: ms(10),
                    },
                    TimedCall {
                        steps: vec![text_step(300, "second")],
                        advance_before_return: ms(20),
                    },
                ],
            );

        let mut per_call = Vec::new();
        for _ in 0..2 {
            let (tx, rx) = mpsc::channel::<StreamChunk>(64);
            let consumer = spawn_stamping_consumer(clock.clone(), rx);
            let start = clock.now_instant();
            let _ = p.chat_streaming("sys", &[], &[], Some(tx)).await.unwrap();
            let seen = consumer.await.unwrap();
            per_call.push((
                seen[0].0 - start,
                seen[0].1.clone(),
                clock.now_instant() - start,
            ));
        }

        assert_eq!(
            per_call,
            vec![
                (ms(100), "first".to_string(), ms(110)),
                (ms(300), "second".to_string(), ms(320)),
            ],
            "the second call plays the second script, not the drained first one"
        );
    }

    #[tokio::test]
    async fn timed_call_without_sink_still_advances_the_clock() {
        let clock = FakeClock::new_at_seed();
        let p = ScriptedLlmProvider::new().with_timed_streaming_calls(
            clock.clone(),
            vec![TimedCall {
                steps: vec![text_step(250, "x")],
                advance_before_return: ms(50),
            }],
        );
        let start = clock.now_instant();
        let _ = p.chat_streaming("sys", &[], &[], None).await.unwrap();
        assert_eq!(clock.now_instant() - start, ms(300));
    }

    #[tokio::test]
    async fn non_streaming_chat_plays_the_timed_script() {
        let clock = FakeClock::new_at_seed();
        let p = ScriptedLlmProvider::new()
            .with_canned_responses(vec![final_answer("hello")])
            .with_timed_streaming_calls(
                clock.clone(),
                vec![TimedCall {
                    steps: vec![text_step(250, "x")],
                    advance_before_return: ms(50),
                }],
            );
        let start = clock.now_instant();
        let response = p.chat("sys", &[], &[]).await.unwrap();
        assert!(matches!(response, LlmResponse::FinalAnswer(t, _) if t == "hello"));
        assert_eq!(clock.now_instant() - start, ms(300));
    }

    #[tokio::test]
    async fn timed_call_survives_a_dropped_receiver() {
        let clock = FakeClock::new_at_seed();
        let p = ScriptedLlmProvider::new().with_timed_streaming_calls(
            clock.clone(),
            vec![TimedCall {
                steps: vec![text_step(100, "x"), text_step(100, "y")],
                advance_before_return: ms(0),
            }],
        );
        let (tx, rx) = mpsc::channel::<StreamChunk>(64);
        drop(rx);
        let start = clock.now_instant();
        let _ = p.chat_streaming("sys", &[], &[], Some(tx)).await.unwrap();
        assert_eq!(clock.now_instant() - start, ms(200));
    }

    #[tokio::test]
    async fn exhausted_timed_calls_fall_back_to_the_untimed_path() {
        let clock = FakeClock::new_at_seed();
        let p = ScriptedLlmProvider::new()
            .with_canned_responses(vec![final_answer("hello")])
            .with_timed_streaming_calls(clock, Vec::new());
        let (tx, mut rx) = mpsc::channel::<StreamChunk>(8);
        let _ = p.chat_streaming("sys", &[], &[], Some(tx)).await.unwrap();
        assert!(matches!(rx.recv().await, Some(StreamChunk::Text(t)) if t == "hello"));
    }

    #[tokio::test]
    async fn embeddings_consumed_in_order() {
        let p = ScriptedLlmProvider::new().with_embeddings(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(p.embed("x").await.unwrap(), vec![1.0, 2.0]);
        assert_eq!(p.embed("y").await.unwrap(), vec![3.0, 4.0]);
        assert!(p.embed("z").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn provider_name_and_model_name_overrides() {
        let p = ScriptedLlmProvider::new()
            .with_provider_name("test-suite")
            .with_model_name("fake-model");
        assert_eq!(p.provider_name(), "test-suite");
        assert_eq!(p.model_name(), "fake-model");
    }
}
