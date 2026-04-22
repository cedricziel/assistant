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
    Status {
        /// Human-readable status message.
        message: String,
        /// Provider-assigned tool-call ID (e.g. Anthropic `tool_use_id`).
        /// `None` for status messages that are not tool-call related.
        tool_call_id: Option<String>,
    },

    /// A tool call completed (successfully or with an error).
    ToolResult {
        /// The name of the tool that was called.
        tool_name: String,
        /// `"ok"` on success, `"error"` or `"denied"` otherwise.
        status: String,
        /// The arguments passed to the tool (JSON object).
        arguments: Option<serde_json::Value>,
        /// The tool's output, truncated to a reasonable display length.
        result: Option<String>,
        /// Provider-assigned tool-call ID (e.g. Anthropic `tool_use_id`).
        /// `None` for providers that do not use IDs (Ollama).
        tool_call_id: Option<String>,
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

    /// Internal reasoning / chain-of-thought produced by the LLM.
    ///
    /// Streamed to the client so UIs can render an expandable "Thinking…"
    /// section in the message timeline.
    Thinking(String),

    /// A subagent process was started.
    SubagentStarted {
        /// Identifier of the spawned subagent.
        agent_id: String,
        /// The task description given to the subagent.
        task: String,
    },

    /// A subagent process completed.
    SubagentCompleted {
        /// Identifier of the completed subagent.
        agent_id: String,
        /// `"ok"` on success, `"error"` on failure.
        status: String,
        /// Short summary of the subagent's result.
        summary: String,
    },

    /// An inner event produced by a running subagent.
    ///
    /// Wraps any orchestrator event produced during a subagent's tool loop so
    /// that consumers can render nested timelines (thinking, tool calls, tokens)
    /// scoped to the subagent.
    SubagentEvent {
        /// Identifier of the subagent that produced this event.
        agent_id: String,
        /// The inner event (token, thinking, tool result, etc.).
        inner: Box<OrchestratorEvent>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_event_stores_content() {
        let event = OrchestratorEvent::Thinking("Let me consider...".to_string());
        match event {
            OrchestratorEvent::Thinking(content) => {
                assert_eq!(content, "Let me consider...");
            }
            _ => panic!("expected Thinking variant"),
        }
    }

    #[test]
    fn subagent_started_event_stores_fields() {
        let event = OrchestratorEvent::SubagentStarted {
            agent_id: "research-1".to_string(),
            task: "Find weather data".to_string(),
        };
        match event {
            OrchestratorEvent::SubagentStarted { agent_id, task } => {
                assert_eq!(agent_id, "research-1");
                assert_eq!(task, "Find weather data");
            }
            _ => panic!("expected SubagentStarted variant"),
        }
    }

    #[test]
    fn subagent_completed_event_stores_fields() {
        let event = OrchestratorEvent::SubagentCompleted {
            agent_id: "research-1".to_string(),
            status: "ok".to_string(),
            summary: "Found current conditions".to_string(),
        };
        match event {
            OrchestratorEvent::SubagentCompleted {
                agent_id,
                status,
                summary,
            } => {
                assert_eq!(agent_id, "research-1");
                assert_eq!(status, "ok");
                assert_eq!(summary, "Found current conditions");
            }
            _ => panic!("expected SubagentCompleted variant"),
        }
    }

    #[test]
    fn new_events_are_cloneable_and_debuggable() {
        let events = vec![
            OrchestratorEvent::Thinking("test".to_string()),
            OrchestratorEvent::SubagentStarted {
                agent_id: "a".to_string(),
                task: "t".to_string(),
            },
            OrchestratorEvent::SubagentCompleted {
                agent_id: "a".to_string(),
                status: "ok".to_string(),
                summary: "s".to_string(),
            },
            OrchestratorEvent::SubagentEvent {
                agent_id: "sub-1".to_string(),
                inner: Box::new(OrchestratorEvent::Token("hello".to_string())),
            },
        ];
        for event in &events {
            let cloned = event.clone();
            let debug = format!("{:?}", cloned);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn status_event_carries_tool_call_id() {
        let event = OrchestratorEvent::Status {
            message: "Calling tool: web-search".to_string(),
            tool_call_id: Some("call_abc123".to_string()),
        };
        match event {
            OrchestratorEvent::Status {
                message,
                tool_call_id,
            } => {
                assert_eq!(message, "Calling tool: web-search");
                assert_eq!(tool_call_id.as_deref(), Some("call_abc123"));
            }
            _ => panic!("expected Status variant"),
        }
    }

    #[test]
    fn status_event_without_tool_call_id() {
        let event = OrchestratorEvent::Status {
            message: "Processing…".to_string(),
            tool_call_id: None,
        };
        match event {
            OrchestratorEvent::Status { tool_call_id, .. } => {
                assert!(tool_call_id.is_none());
            }
            _ => panic!("expected Status variant"),
        }
    }

    #[test]
    fn tool_result_carries_tool_call_id() {
        let event = OrchestratorEvent::ToolResult {
            tool_name: "file-read".to_string(),
            status: "ok".to_string(),
            arguments: None,
            result: Some("contents".to_string()),
            tool_call_id: Some("call_xyz789".to_string()),
        };
        match event {
            OrchestratorEvent::ToolResult { tool_call_id, .. } => {
                assert_eq!(tool_call_id.as_deref(), Some("call_xyz789"));
            }
            _ => panic!("expected ToolResult variant"),
        }
    }

    #[test]
    fn subagent_event_wraps_inner_events() {
        let event = OrchestratorEvent::SubagentEvent {
            agent_id: "research-agent".to_string(),
            inner: Box::new(OrchestratorEvent::ToolResult {
                tool_name: "web-search".to_string(),
                status: "ok".to_string(),
                arguments: Some(serde_json::json!({"query": "rust async"})),
                result: Some("Found results".to_string()),
                tool_call_id: Some("call_inner_123".to_string()),
            }),
        };
        match event {
            OrchestratorEvent::SubagentEvent { agent_id, inner } => {
                assert_eq!(agent_id, "research-agent");
                match *inner {
                    OrchestratorEvent::ToolResult {
                        ref tool_name,
                        ref status,
                        ..
                    } => {
                        assert_eq!(tool_name, "web-search");
                        assert_eq!(status, "ok");
                    }
                    _ => panic!("expected ToolResult inner event"),
                }
            }
            _ => panic!("expected SubagentEvent variant"),
        }
    }
}
