//! Per-call options for [`LlmProvider::chat_with`](crate::LlmProvider::chat_with).

use serde_json::Value;

use crate::types::transcription::TtsFormat;

/// Which tools the model may or must call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ToolChoice {
    /// The model decides.
    #[default]
    Auto,
    /// No tool may be called.
    None,
    /// At least one tool must be called.
    Required,
    /// This tool must be called.
    Tool(String),
}

/// The shape of the final answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ResponseFormat {
    #[default]
    Text,
    /// JSON matching `schema`. Only providers that report
    /// [`ChatOptionsSupport::json_schema`] send it; others ignore it.
    JsonSchema {
        name: String,
        schema: Value,
        strict: bool,
    },
}

/// Options for one chat call. The default is today's behavior.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatOptions {
    pub tool_choice: ToolChoice,
    /// `None` keeps the provider's default.
    pub parallel_tool_calls: Option<bool>,
    pub response_format: ResponseFormat,
    /// Speak the answer too, encoded as this format. Only providers that
    /// report [`ChatOptionsSupport::audio_output`] send it; others ignore it.
    pub audio_reply: Option<TtsFormat>,
}

/// Which [`ChatOptions`] a provider sends natively. Unsupported options are
/// emulated by the default `chat_with` or ignored, as documented there.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChatOptionsSupport {
    pub tool_choice: bool,
    pub parallel_tool_calls: bool,
    pub json_schema: bool,
    /// The model hears [`ContentBlock::Audio`](crate::ContentBlock::Audio).
    pub audio_input: bool,
    /// The model can speak its answer ([`ChatOptions::audio_reply`]).
    pub audio_output: bool,
}
