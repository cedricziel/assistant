//! `LlmProvider` trait — the single abstraction point for all LLM backends.
//!
//! Implement this trait to plug in a new provider (Ollama, OpenAI, Anthropic, …).
//! All orchestration and skill-execution code works against `Arc<dyn LlmProvider>`
//! so no provider-specific code leaks into the core runtime.

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::stream_chunk::StreamChunk;
use super::tool_spec::ToolSpec;
use super::types::{ChatHistoryMessage, LlmResponse};

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
    /// Hosted tools supplied directly by the provider (e.g. Anthropic web search).
    pub hosted_tools: Vec<HostedTool>,
    /// The model's context window size in tokens, if known.
    ///
    /// When set, the orchestrator uses this to override the default compaction
    /// `context_window_tokens` so that history compaction triggers at the right
    /// threshold for the actual model being used.
    pub context_window_tokens: Option<u64>,
}

/// Provider-managed tools that should suppress local equivalents.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostedTool {
    /// Anthropic-managed `web_search` tool.
    WebSearch,
    /// Anthropic-managed `web_fetch` tool.
    WebFetch,
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

    /// Like [`chat`] but streams typed chunks through `chunk_sink` as they are
    /// generated.
    ///
    /// [`StreamChunk::Text`] carries final-answer tokens visible to the user.
    /// [`StreamChunk::Thinking`] carries internal reasoning tokens (when the
    /// provider supports extended thinking).
    ///
    /// The method still returns the complete [`LlmResponse`] once generation
    /// is finished.
    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse>;

    /// Compute a dense vector embedding for `text`.
    ///
    /// Returns an error if the provider does not support embeddings.
    /// The default implementation always returns an error so that existing
    /// providers do not need to be updated until they are ready.
    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!("Embedding not supported by this provider"))
    }

    // ── Provider identity (for OTel GenAI semantic conventions) ───────────

    /// Short, stable identifier for the provider system (e.g. `"ollama"`,
    /// `"anthropic"`).  Maps to the OTel `gen_ai.provider.name` attribute.
    fn provider_name(&self) -> &str {
        "unknown"
    }

    /// The model name configured for this provider instance (e.g.
    /// `"qwen2.5:7b"`, `"claude-opus-4-6"`).
    /// Maps to the OTel `gen_ai.request.model` attribute.
    fn model_name(&self) -> &str {
        "unknown"
    }

    /// Base URL / address of the inference server (e.g.
    /// `"http://localhost:11434"`, `"https://api.anthropic.com"`).
    /// Maps to the OTel `server.address` attribute.
    fn server_address(&self) -> &str {
        ""
    }
}
