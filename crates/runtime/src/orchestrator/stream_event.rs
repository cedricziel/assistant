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

    /// A skill run completed (success or failure).
    ///
    /// Emitted after a skill tool invocation finishes so that notification
    /// handlers can fire a "Skill complete" push without needing to know
    /// which tool names map to skills.
    SkillComplete {
        /// The skill name (matches the skill's `name` field).
        skill_name: String,
        /// Whether the skill succeeded (`true`) or failed (`false`).
        success: bool,
        /// Short human-readable description of the outcome (first 120 chars).
        summary: String,
    },

    /// The orchestrator encountered a critical error that prevented it from
    /// producing a response (e.g. LLM timeout, safety gate block).
    AgentError {
        /// Human-readable description of the error.
        message: String,
    },

    /// A `voice-response` tool call produced synthesised audio that clients
    /// should auto-play.
    ///
    /// Emitted immediately after the tool result is finalised so that
    /// voice-enabled web clients can retrieve and play the audio without
    /// waiting for the full turn to complete.
    AudioReady {
        /// The UUID of the audio blob in the server's [`AudioStore`].
        /// Retrieve via `GET /api/audio/{audio_id}`.
        audio_id: String,
    },
}
