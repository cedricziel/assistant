//! Dedicated embedding provider abstraction.
//!
//! Allows decoupling the embedding backend from the main LLM provider.
//! For example, an Anthropic chat provider can use Ollama or Voyage AI
//! for embeddings via [`WithEmbeddingOverride`].

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::provider::{Capabilities, LlmProvider};
use crate::tool_spec::ToolSpec;
use crate::{ChatHistoryMessage, LlmResponse};

// ── EmbeddingProvider trait ──────────────────────────────────────────────────

/// Minimal trait for providers that can compute text embeddings.
///
/// Unlike [`LlmProvider`], implementors only need to support `embed()`.
/// This allows lightweight embedding-only clients (e.g. Voyage AI) without
/// stubbing out chat methods.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Compute a dense vector embedding for `text`.
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>>;
}

// ── LlmEmbedder adapter ─────────────────────────────────────────────────────

/// Adapts any [`LlmProvider`] to the [`EmbeddingProvider`] trait by
/// delegating to its `embed()` method.
///
/// Use this to wrap an existing Ollama or OpenAI provider as an
/// embedding-only backend.
pub struct LlmEmbedder(pub Arc<dyn LlmProvider>);

#[async_trait]
impl EmbeddingProvider for LlmEmbedder {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        self.0.embed(text).await
    }
}

// ── WithEmbeddingOverride ────────────────────────────────────────────────────

/// Wraps a primary [`LlmProvider`] and overrides its `embed()` with a
/// dedicated [`EmbeddingProvider`].
///
/// Chat methods are forwarded to the inner provider unchanged.  This is
/// the composition point used at startup to combine e.g. an Anthropic
/// chat provider with an Ollama or Voyage embedding provider.
pub struct WithEmbeddingOverride {
    inner: Arc<dyn LlmProvider>,
    embedder: Arc<dyn EmbeddingProvider>,
}

impl WithEmbeddingOverride {
    /// Create a new composite provider.
    ///
    /// * `inner` — the primary provider for chat / streaming / capabilities.
    /// * `embedder` — the dedicated embedding backend.
    pub fn new(inner: Arc<dyn LlmProvider>, embedder: Arc<dyn EmbeddingProvider>) -> Self {
        Self { inner, embedder }
    }
}

#[async_trait]
impl LlmProvider for WithEmbeddingOverride {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        self.inner.chat(system_prompt, history, tools).await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse> {
        self.inner
            .chat_streaming(system_prompt, history, tools, token_sink)
            .await
    }

    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        self.embedder.embed(text).await
    }

    fn provider_name(&self) -> &str {
        self.inner.provider_name()
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn server_address(&self) -> &str {
        self.inner.server_address()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LlmResponseMeta;

    /// Stub provider that always returns a fixed answer.
    struct StubChat;

    #[async_trait]
    impl LlmProvider for StubChat {
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                tools: crate::ToolSupport::None,
                streaming: false,
                vision: false,
                hosted_tools: Vec::new(),
            }
        }

        async fn chat(
            &self,
            _system_prompt: &str,
            _history: &[ChatHistoryMessage],
            _tools: &[ToolSpec],
        ) -> anyhow::Result<LlmResponse> {
            Ok(LlmResponse::FinalAnswer(
                "stub".to_string(),
                LlmResponseMeta::default(),
            ))
        }

        async fn chat_streaming(
            &self,
            _system_prompt: &str,
            _history: &[ChatHistoryMessage],
            _tools: &[ToolSpec],
            _token_sink: Option<mpsc::Sender<String>>,
        ) -> anyhow::Result<LlmResponse> {
            Ok(LlmResponse::FinalAnswer(
                "stub".to_string(),
                LlmResponseMeta::default(),
            ))
        }

        async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
            anyhow::bail!("StubChat does not support embeddings")
        }

        fn provider_name(&self) -> &str {
            "stub"
        }
    }

    /// Stub embedder that returns a fixed vector.
    struct StubEmbedder;

    #[async_trait]
    impl EmbeddingProvider for StubEmbedder {
        async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
            Ok(vec![0.1, 0.2, 0.3])
        }
    }

    /// Stub provider that actually supports embeddings (like Ollama/OpenAI).
    struct StubChatWithEmbedding;

    #[async_trait]
    impl LlmProvider for StubChatWithEmbedding {
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                tools: crate::ToolSupport::None,
                streaming: false,
                vision: false,
                hosted_tools: Vec::new(),
            }
        }

        async fn chat(
            &self,
            _system_prompt: &str,
            _history: &[ChatHistoryMessage],
            _tools: &[ToolSpec],
        ) -> anyhow::Result<LlmResponse> {
            Ok(LlmResponse::FinalAnswer(
                "chat-with-embed".to_string(),
                LlmResponseMeta::default(),
            ))
        }

        async fn chat_streaming(
            &self,
            _system_prompt: &str,
            _history: &[ChatHistoryMessage],
            _tools: &[ToolSpec],
            _token_sink: Option<mpsc::Sender<String>>,
        ) -> anyhow::Result<LlmResponse> {
            Ok(LlmResponse::FinalAnswer(
                "chat-with-embed".to_string(),
                LlmResponseMeta::default(),
            ))
        }

        async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
            Ok(vec![1.0, 2.0, 3.0, 4.0])
        }

        fn provider_name(&self) -> &str {
            "stub-with-embed"
        }

        fn model_name(&self) -> &str {
            "embed-model"
        }

        fn server_address(&self) -> &str {
            "http://localhost:9999"
        }
    }

    /// Stub embedder that always fails (simulates misconfigured provider).
    struct FailingEmbedder;

    #[async_trait]
    impl EmbeddingProvider for FailingEmbedder {
        async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
            anyhow::bail!("embedding service unavailable")
        }
    }

    // -- WithEmbeddingOverride tests -----------------------------------------

    #[tokio::test]
    async fn override_delegates_embed_to_embedder() {
        let chat = Arc::new(StubChat);
        let embedder = Arc::new(StubEmbedder);
        let composite = WithEmbeddingOverride::new(chat, embedder);

        // embed() should use the dedicated embedder, not the inner provider.
        let result = composite.embed("hello").await.unwrap();
        assert_eq!(result, vec![0.1, 0.2, 0.3]);
    }

    #[tokio::test]
    async fn override_delegates_chat_to_inner() {
        let chat = Arc::new(StubChat);
        let embedder = Arc::new(StubEmbedder);
        let composite = WithEmbeddingOverride::new(chat, embedder);

        let response = composite.chat("sys", &[], &[]).await.unwrap();
        match response {
            LlmResponse::FinalAnswer(text, _) => assert_eq!(text, "stub"),
            other => panic!("Expected FinalAnswer, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn override_delegates_chat_streaming_to_inner() {
        let chat = Arc::new(StubChat);
        let embedder = Arc::new(StubEmbedder);
        let composite = WithEmbeddingOverride::new(chat, embedder);

        let response = composite
            .chat_streaming("sys", &[], &[], None)
            .await
            .unwrap();
        match response {
            LlmResponse::FinalAnswer(text, _) => assert_eq!(text, "stub"),
            other => panic!("Expected FinalAnswer, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn override_preserves_inner_identity() {
        let chat: Arc<dyn LlmProvider> = Arc::new(StubChatWithEmbedding);
        let embedder: Arc<dyn EmbeddingProvider> = Arc::new(StubEmbedder);
        let composite = WithEmbeddingOverride::new(chat, embedder);

        assert_eq!(composite.provider_name(), "stub-with-embed");
        assert_eq!(composite.model_name(), "embed-model");
        assert_eq!(composite.server_address(), "http://localhost:9999");
    }

    #[tokio::test]
    async fn override_embed_error_propagates_from_embedder() {
        let chat = Arc::new(StubChatWithEmbedding);
        let embedder: Arc<dyn EmbeddingProvider> = Arc::new(FailingEmbedder);
        let composite = WithEmbeddingOverride::new(chat, embedder);

        // The inner provider supports embeddings, but the override should use
        // the dedicated (failing) embedder instead.
        let result = composite.embed("hello").await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("unavailable"),
            "Error should come from the FailingEmbedder"
        );
    }

    // -- Fallback: no override means main provider is used -------------------

    #[tokio::test]
    async fn no_override_uses_main_provider_embed() {
        // When no WithEmbeddingOverride is applied, the provider's own
        // embed() is called.  This simulates the Ollama/OpenAI path where
        // [llm.embeddings] is not configured.
        let provider: Arc<dyn LlmProvider> = Arc::new(StubChatWithEmbedding);
        let result = provider.embed("test").await.unwrap();
        assert_eq!(
            result,
            vec![1.0, 2.0, 3.0, 4.0],
            "Should get the main provider's embedding"
        );
    }

    #[tokio::test]
    async fn no_override_error_when_main_provider_lacks_embeddings() {
        // When no WithEmbeddingOverride is applied and the main provider
        // does not support embeddings (like Anthropic), embed() returns
        // an error.  This simulates the case where the user forgot to
        // configure [llm.embeddings].
        let provider: Arc<dyn LlmProvider> = Arc::new(StubChat);
        let result = provider.embed("test").await;
        assert!(
            result.is_err(),
            "Provider without embedding support should error"
        );
    }

    // -- LlmEmbedder adapter ------------------------------------------------

    #[tokio::test]
    async fn llm_embedder_adapter_delegates_error() {
        // Verify LlmEmbedder wraps an LlmProvider for use as EmbeddingProvider.
        // StubChat.embed() returns an error, so LlmEmbedder should propagate it.
        let provider: Arc<dyn LlmProvider> = Arc::new(StubChat);
        let adapter = LlmEmbedder(provider);
        let result = adapter.embed("hello").await;
        assert!(result.is_err(), "StubChat embed should error");
    }

    #[tokio::test]
    async fn llm_embedder_adapter_delegates_success() {
        // When wrapping a provider that supports embeddings, the adapter
        // should forward the result.
        let provider: Arc<dyn LlmProvider> = Arc::new(StubChatWithEmbedding);
        let adapter = LlmEmbedder(provider);
        let result = adapter.embed("hello").await.unwrap();
        assert_eq!(
            result,
            vec![1.0, 2.0, 3.0, 4.0],
            "Adapter should forward embedding from inner provider"
        );
    }

    // -- Composite: override replaces a working provider's embed -------------

    #[tokio::test]
    async fn override_replaces_working_embed_with_dedicated() {
        // Even if the inner provider supports embeddings natively,
        // the override should take precedence.
        let chat: Arc<dyn LlmProvider> = Arc::new(StubChatWithEmbedding);
        let embedder: Arc<dyn EmbeddingProvider> = Arc::new(StubEmbedder);
        let composite = WithEmbeddingOverride::new(chat, embedder);

        let result = composite.embed("hello").await.unwrap();
        assert_eq!(
            result,
            vec![0.1, 0.2, 0.3],
            "Override embedder should win over inner provider's embed()"
        );
    }
}
