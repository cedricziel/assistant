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

use std::sync::{Arc, Mutex};

use anyhow::Result;
use assistant_core::{
    ChatHistoryMessage, LlmProvider, LlmResponse, LlmResponseMeta, StreamChunk, ToolSpec,
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

/// Internal state — wrapped in `Mutex` so the provider can be shared
/// across async tasks while still being mutable.
#[derive(Default)]
struct ScriptedState {
    responses: Vec<LlmResponse>,
    stream_chunks: Vec<StreamChunk>,
    embeddings: Vec<Vec<f32>>,
    recorded_calls: Vec<RecordedCall>,
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
    state: Arc<Mutex<ScriptedState>>,
    capabilities: Capabilities,
    provider_name: String,
    model_name: String,
}

impl ScriptedLlmProvider {
    /// Create an empty scripted provider with default capabilities.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ScriptedState::default())),
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
        Ok(self.record_and_pop_response(system_prompt, history, tools, false))
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> Result<LlmResponse> {
        let response = self.record_and_pop_response(system_prompt, history, tools, true);

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
