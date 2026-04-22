//! Shared LLM types used across the workspace.
//!
//! These types form the contract between the LLM provider implementations,
//! the orchestrator, and other consumers.  They live in `core` so that any
//! crate in the workspace can reference them without depending on a specific
//! provider crate.

use serde::{Deserialize, Serialize};

// ── Public types ──────────────────────────────────────────────────────────────

/// A single content block in a multimodal message.
///
/// Used by [`ChatHistoryMessage::MultimodalUser`] to carry a mix of text and
/// inline images.
#[derive(Debug, Clone)]
pub enum ContentBlock {
    /// A text segment.
    Text(String),
    /// A base64-encoded image.
    Image {
        /// MIME type, e.g. `"image/png"`, `"image/jpeg"`.
        media_type: String,
        /// Base64-encoded image data (no data-URI prefix).
        data: String,
    },
    /// A base64-encoded document (e.g. PDF).
    ///
    /// Providers with native document support (Anthropic) serialize this as a
    /// `"document"` content block.  Others should receive a text-extracted
    /// fallback via [`ContentBlock::Text`] instead — the orchestrator handles
    /// the conversion before reaching the provider.
    Document {
        /// MIME type, e.g. `"application/pdf"`.
        media_type: String,
        /// Base64-encoded document data.
        data: String,
    },
}

/// A single message in the chat history as tracked by the caller.
///
/// The enum reflects the structurally distinct message kinds in the
/// Ollama (and OpenAI-compatible) multi-turn tool-calling format:
///
/// * `Text` — a plain user, assistant, or system message.
/// * `MultimodalUser` — a user message with mixed text and image content.
/// * `AssistantToolCalls` — the assistant's decision to invoke one or more
///   tools.  Serialises to `{"role":"assistant","content":"","tool_calls":[…]}`.
/// * `ToolResult` — the result returned for a single tool invocation.
///   Serialises to `{"role":"tool","name":"…","content":"…"}`.
#[derive(Debug, Clone)]
pub enum ChatHistoryMessage {
    /// A plain text message (user / assistant / system).
    Text { role: ChatRole, content: String },
    /// A user message with mixed text and image content blocks.
    ///
    /// Providers that do not support vision should extract only the
    /// [`ContentBlock::Text`] parts and discard images.
    MultimodalUser { content: Vec<ContentBlock> },
    /// The assistant requested one or more tool calls in a single turn.
    AssistantToolCalls(Vec<ToolCallItem>),
    /// The result of a single tool invocation.
    ToolResult { name: String, content: String },
}

/// Chat participant role.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallItem {
    pub name: String,
    pub params: serde_json::Value,
    /// Provider-assigned call ID (e.g. Anthropic `tool_use_id`).
    /// `None` for providers that do not use IDs (Ollama).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Metadata returned alongside the model's response.
///
/// Fields are best-effort: providers populate what they can, leaving the rest
/// as `None` / `0`.  The struct is intentionally cheap to clone.
#[derive(Debug, Clone, Default)]
pub struct LlmResponseMeta {
    /// Model identifier echoed by the provider (e.g. `"qwen2.5:7b"`).
    pub model: Option<String>,
    /// Number of tokens in the prompt (input).
    pub input_tokens: Option<u64>,
    /// Number of tokens in the completion (output).
    pub output_tokens: Option<u64>,
    /// Provider-specific finish/stop reason (e.g. `"stop"`, `"tool_calls"`).
    pub finish_reason: Option<String>,
    /// Provider-assigned response/message ID.
    pub response_id: Option<String>,
}

/// Structured payload for a tool-call response from the LLM.
///
/// Carries the tool calls themselves, response metadata, and any thinking
/// that was produced alongside the tool calls (which would otherwise be
/// discarded by providers that treat response types as mutually exclusive).
#[derive(Debug, Clone)]
pub struct ToolCallResponse {
    /// The tool invocations requested by the model.
    pub items: Vec<ToolCallItem>,
    /// Response-level metadata (token counts, model, etc.).
    pub meta: LlmResponseMeta,
    /// Thinking/reasoning that preceded the tool calls.
    /// `None` if no thinking was produced, or if thinking was already
    /// streamed via `StreamChunk::Thinking` during the call.
    pub thinking: Option<String>,
}

/// The outcome of a single LLM chat invocation.
#[derive(Debug, Clone)]
pub enum LlmResponse {
    /// The model wants to call one or more tools.
    ToolCalls(ToolCallResponse),
    /// The model has a definitive answer for the user.
    FinalAnswer(String, LlmResponseMeta),
    /// The model emitted only a reasoning step (no action yet).
    Thinking(String, LlmResponseMeta),
}

impl LlmResponse {
    /// Access the response metadata regardless of variant.
    pub fn meta(&self) -> &LlmResponseMeta {
        match self {
            LlmResponse::ToolCalls(resp) => &resp.meta,
            LlmResponse::FinalAnswer(_, m) => m,
            LlmResponse::Thinking(_, m) => m,
        }
    }
}
