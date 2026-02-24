//! `LlmProvider` trait — the single abstraction point for all LLM backends.
//!
//! Implement this trait to plug in a new provider (Ollama, OpenAI, Anthropic, …).
//! All orchestration and skill-execution code works against `Arc<dyn LlmProvider>`
//! so no provider-specific code leaks into the core runtime.

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::tool_spec::ToolSpec;
use crate::{ChatHistoryMessage, LlmResponse};

// ── Capabilities ─────────────────────────────────────────────────────────────

/// Level of tool-calling support offered by a provider.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolSupport {
    /// Provider understands the `tools` / `tool_calls` wire protocol natively.
    Native,
    /// Provider has no structured tool-calling support.
    None,
}

/// Static metadata describing what a provider can do.
#[derive(Debug, Clone)]
pub struct Capabilities {
    /// Whether and how the provider supports tool / function calling.
    pub tools: ToolSupport,
    /// Whether the provider supports streaming token output.
    pub streaming: bool,
    /// Whether the provider accepts image inputs.
    pub vision: bool,
}

// ── LlmProvider trait ─────────────────────────────────────────────────────────

/// Common interface for LLM backends.
///
/// All internal orchestration code works against `Arc<dyn LlmProvider>` so the
/// concrete provider (Ollama, OpenAI, Anthropic, …) is swapped without touching
/// the runtime or tool-executor.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Return static metadata about this provider's capabilities.
    fn capabilities(&self) -> Capabilities;

    /// Send a chat turn and return the model's response.
    ///
    /// # Parameters
    /// * `system_prompt` – base system instructions
    /// * `history` – previous messages in the conversation
    /// * `tools` – tools available for this turn (passed as native tool specs)
    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse>;

    /// Like [`chat`] but streams final-answer tokens through `token_sink` as
    /// they are generated.
    ///
    /// Tool-call steps are never streamed — only the tokens that form part of
    /// a `FinalAnswer` are forwarded.  The method still returns the complete
    /// [`LlmResponse`] once generation is finished.
    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        token_sink: Option<mpsc::Sender<String>>,
    ) -> anyhow::Result<LlmResponse>;

    /// Compute a dense vector embedding for `text`.
    ///
    /// Returns an error if the provider does not support embeddings.
    /// The default implementation always returns an error so that existing
    /// providers do not need to be updated until they are ready.
    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!("Embedding not supported by this provider"))
    }
}
