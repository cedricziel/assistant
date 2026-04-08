//! Structured events emitted by the orchestrator during a streaming turn.
//!
//! [`OrchestratorEvent`] is the richer replacement for raw `String` tokens.
//! Consumers receive the full semantic context of what the orchestrator is
//! doing — not just the text being produced — which enables UIs to render
//! live tool-call indicators, status messages, and completion badges.

/// A single event emitted by the orchestrator during a streaming turn.
///
/// Events flow through an `mpsc::Sender<OrchestratorEvent>` registered via
/// [`Orchestrator::register_token_sink`].  The sender is consumed (removed)
/// by the worker once the turn is complete; callers should drain their
/// receiver until it is closed to know the turn is finished.
#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    /// An incremental text token produced by the LLM.
    Token(String),

    /// A human-readable status update while the turn is in progress.
    ///
    /// Examples: `"Calling tool: web-search"`, `"Processing results…"`.
    Status(String),

    /// A tool call completed (successfully or with an error).
    ToolResult {
        /// The name of the tool that was called.
        tool_name: String,
        /// `"ok"` on success, `"error"` or `"denied"` otherwise.
        status: String,
    },
}
