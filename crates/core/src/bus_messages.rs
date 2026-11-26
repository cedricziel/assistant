//! Typed message envelopes for the bus.
//!
//! Each struct maps to a specific topic and provides compile-time guarantees
//! on payload shape.  Envelopes are serialised to/from `serde_json::Value`
//! for transport through [`PublishRequest`](crate::PublishRequest) payloads.
//!
//! # Topic → Envelope mapping
//!
//! | Topic             | Envelope            |
//! |-------------------|---------------------|
//! | `turn.request`    | [`TurnRequest`]     |
//! | `turn.result`     | [`TurnResult`]      |
//! | `turn.status`     | [`TurnStatus`]      |
//! | `tool.execute`    | [`ToolExecute`]     |
//! | `tool.result`     | [`ToolResult`]      |
//! | `agent.spawn`     | [`AgentSpawn`]      |
//! | `agent.report`    | [`AgentReport`]     |

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// -- Topics (constants) -----------------------------------------------------

/// Well-known topic names.
///
/// Using constants avoids typos and makes it easy to grep for all producers
/// and consumers of a given topic.
pub mod topic {
    pub const TURN_REQUEST: &str = "turn.request";
    /// The reply to one `turn.request`, consumed by whoever submitted it.
    pub const TURN_RESULT: &str = "turn.result";
    /// Announces a successful turn to background consumers (titling).
    ///
    /// Separate from [`TURN_RESULT`] because a SQLite bus claim is exclusive:
    /// a background consumer of `turn.result` takes the reply its submitter
    /// is waiting for. Payload: [`TurnResult`](super::TurnResult).
    pub const TURN_COMPLETED: &str = "turn.completed";
    pub const TURN_STATUS: &str = "turn.status";
    /// One `OrchestratorEvent` of a running turn, relayed from the process
    /// that runs the turn to the process that streams it. `batch_id` is the
    /// run id. Payload: a serialized `OrchestratorEvent`.
    pub const TURN_EVENT: &str = "turn.event";
    pub const TOOL_EXECUTE: &str = "tool.execute";
    pub const TOOL_RESULT: &str = "tool.result";
    pub const AGENT_SPAWN: &str = "agent.spawn";
    pub const AGENT_REPORT: &str = "agent.report";
    pub const AGENT_TERMINATE: &str = "agent.terminate";
    pub const SCHEDULE_TRIGGER: &str = "schedule.trigger";
}

// -- Turn envelopes ---------------------------------------------------------

/// A request for an agent to run a turn.
///
/// Published by interfaces (CLI, Slack, …) or by parent agents delegating
/// sub-turns.
///
/// Topic: [`topic::TURN_REQUEST`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRequest {
    /// The user's prompt text.
    pub prompt: String,
    /// Conversation to continue (or start).
    pub conversation_id: Uuid,
    /// Extension tool names available for this turn (e.g. `"reply"`, `"react"`).
    /// These are interface-specific tools that the interface provides.
    #[serde(default)]
    pub extension_tools: Vec<String>,
    /// When the message was received by the interface.
    ///
    /// Used to inject a `[YYYY-MM-DD HH:MM:SS TZ]` prefix into the prompt so
    /// the agent knows exactly when each message arrived.
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
    /// W3C Trace Context `traceparent` for cross-queue trace propagation.
    ///
    /// Producers can set this so workers reconstruct parent context before
    /// running the turn, preserving a single distributed trace across
    /// publish/claim async boundaries.
    #[serde(default)]
    pub traceparent: Option<String>,
    /// IDs of user-provided attachments (images, etc.) for this turn.
    ///
    /// Lightweight references — the orchestrator loads actual bytes from
    /// [`AttachmentStore`] at LLM-call time.
    #[serde(default)]
    pub attachment_ids: Vec<Uuid>,
    /// Whether a human is waiting on this turn and can answer prompts. The
    /// submitter decides; the worker copies it to `ExecutionContext`.
    #[serde(default)]
    pub interactive: bool,
    /// The submitter streams this turn from another process. When no local
    /// event sink is registered, the worker relays each event on
    /// [`topic::TURN_EVENT`] with `batch_id` set to the run id.
    #[serde(default)]
    pub relay_events: bool,
    /// Where in the conversation tree the turn's rows attach. Carried, not yet
    /// acted on by the worker.
    #[serde(default)]
    pub anchor: TurnAnchor,
    /// The id the run will persist its user row under, when the producer
    /// pre-assigns one. Carried, not yet acted on by the worker.
    #[serde(default)]
    pub user_message_id: Option<Uuid>,
    /// Model for this turn only, overriding the conversation and org defaults.
    /// Carried, not yet acted on by the worker.
    #[serde(default)]
    pub model_override: Option<String>,

    // -- Identity (multi-user) ------------------------------------------------
    /// The authenticated user who initiated this turn.
    #[serde(default)]
    pub user_id: Option<String>,
    /// The organisation context for this turn.
    #[serde(default)]
    pub org_id: Option<String>,
    /// The space context for this turn.
    #[serde(default)]
    pub space_id: Option<String>,
}

/// Where a turn attaches in the conversation's message tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnAnchor {
    /// Append the prompt after the conversation's current leaf.
    #[default]
    Append,
    /// Replace the earlier user message `edited_message_id`: the prompt
    /// becomes a sibling branch under `parent_message_id` (`None` for a first
    /// message), and the attachments of the edited message carry forward.
    Edit {
        parent_message_id: Option<Uuid>,
        edited_message_id: Uuid,
    },
    /// Re-run the reply to an existing user message without a new user row.
    Regenerate { user_message_id: Uuid },
}

/// The final result of a completed turn.
///
/// Published by the orchestrator when the agent produces a final answer.
///
/// Topic: [`topic::TURN_RESULT`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnResult {
    /// The conversation this result belongs to.
    pub conversation_id: Uuid,
    /// The agent's final answer text.
    pub content: String,
    /// Turn number within the conversation.
    pub turn: i64,
    /// IDs of attachments produced by tool outputs during the turn.
    ///
    /// Lightweight references — interfaces load actual bytes from
    /// [`AttachmentStore`] when they need to deliver files.
    #[serde(default)]
    pub attachment_ids: Vec<Uuid>,
    /// The UUID of the persisted assistant message in the database.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<Uuid>,
    /// Whether any tool dispatched during the turn produced an error.
    ///
    /// Gates downstream signals like skill-learner evaluation.
    #[serde(default)]
    pub had_errors: bool,
    /// How long the turn took. `None` when the turn was not measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<crate::TurnTiming>,
}

/// A status update emitted during turn processing.
///
/// Published by the orchestrator to inform interfaces about progress.
///
/// Topic: [`topic::TURN_STATUS`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnStatus {
    /// The conversation this status belongs to.
    pub conversation_id: Uuid,
    /// Current phase.
    pub phase: TurnPhase,
    /// Optional detail (e.g. which tool is being called).
    #[serde(default)]
    pub detail: Option<String>,
}

/// Phase of turn processing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnPhase {
    /// Agent is reasoning / generating.
    Thinking,
    /// Agent is executing tool calls.
    CallingTools,
    /// Agent is producing the final answer.
    Responding,
}

// -- Tool envelopes ---------------------------------------------------------

/// A request to execute a single tool call.
///
/// Published by the orchestrator. Tool calls within a turn are published
/// sequentially (in the order the LLM emitted them) and processed one
/// at a time per conversation.
///
/// Topic: [`topic::TOOL_EXECUTE`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecute {
    /// Name of the tool to invoke (kebab-case, e.g. `"file-read"`).
    pub tool_name: String,
    /// Tool call ID from the LLM response (for correlating results).
    pub call_id: String,
    /// Parameters to pass to the tool handler.
    pub params: HashMap<String, Value>,
    /// The conversation context.
    pub conversation_id: Uuid,
    /// Turn number within the conversation.
    pub turn: i64,
}

/// The result of a tool execution.
///
/// Published by the tool executor after running a tool.
///
/// Topic: [`topic::TOOL_RESULT`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Name of the tool that was executed.
    pub tool_name: String,
    /// Tool call ID (matches [`ToolExecute::call_id`]).
    pub call_id: String,
    /// Text content of the tool output.
    pub content: String,
    /// Whether the tool completed successfully.
    pub success: bool,
    /// Optional structured data from the tool.
    #[serde(default)]
    pub data: Option<Value>,
}

// -- Agent lifecycle envelopes ----------------------------------------------

/// A request to spawn a sub-agent for a delegated task.
///
/// Published by a parent agent when it decides to delegate work.
///
/// Topic: [`topic::AGENT_SPAWN`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawn {
    /// Identifier for the new agent.
    pub agent_id: String,
    /// Human-readable description of what this agent should do.
    pub task: String,
    /// Optional system prompt override for the sub-agent.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Optional model override (e.g. use a smaller model for simple tasks).
    #[serde(default)]
    pub model: Option<String>,
    /// Tool names the sub-agent is allowed to use.
    /// Empty means all tools.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Whether this subagent should run bound to `agent_id` as a Persona.
    #[serde(default)]
    pub persona_bound: bool,
    /// Parent conversation ID that initiated this delegated task.
    #[serde(default)]
    pub parent_conversation_id: Option<Uuid>,
    /// Parent agent (Persona) ID that initiated this delegated task.
    #[serde(default)]
    pub parent_agent_id: Option<String>,
}

/// A report from a sub-agent back to its parent.
///
/// Published by the sub-agent when it has completed (or failed) its task.
///
/// Topic: [`topic::AGENT_REPORT`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReport {
    /// Status of the sub-agent's work.
    pub status: AgentReportStatus,
    /// The sub-agent's output / answer.
    pub content: String,
    /// Optional structured data.
    #[serde(default)]
    pub data: Option<Value>,
}

/// Outcome of a sub-agent's work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentReportStatus {
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was cancelled by the parent.
    Cancelled,
}

// -- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_request_roundtrips_json() {
        let req = TurnRequest {
            prompt: "hello world".into(),
            conversation_id: Uuid::new_v4(),
            extension_tools: vec!["reply".into(), "react".into()],
            timestamp: None,
            traceparent: Some(
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
            ),
            attachment_ids: vec![],
            interactive: true,
            relay_events: false,
            anchor: TurnAnchor::Append,
            user_message_id: None,
            model_override: None,
            user_id: None,
            org_id: None,
            space_id: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        let back: TurnRequest = serde_json::from_value(json).unwrap();
        assert!(back.interactive, "the interactive flag survives the bus");
        assert_eq!(back.prompt, req.prompt);
        assert_eq!(back.conversation_id, req.conversation_id);
        assert_eq!(back.extension_tools, req.extension_tools);
        assert_eq!(back.traceparent, req.traceparent);
    }

    fn bus_turn_result(timing: Option<crate::TurnTiming>) -> TurnResult {
        TurnResult {
            conversation_id: Uuid::new_v4(),
            content: "done".into(),
            turn: 0,
            attachment_ids: vec![],
            message_id: None,
            had_errors: false,
            timing,
        }
    }

    #[test]
    fn turn_result_without_timing_omits_the_key() {
        let json = serde_json::to_value(bus_turn_result(None)).unwrap();
        assert!(
            json.get("timing").is_none(),
            "timing: None must not serialise a key"
        );
    }

    #[test]
    fn turn_result_payload_without_timing_deserialises() {
        let json = serde_json::json!({
            "conversation_id": Uuid::new_v4(),
            "content": "done",
            "turn": 0,
        });
        let back: TurnResult = serde_json::from_value(json).unwrap();
        assert_eq!(
            back.timing, None,
            "a payload without timing defaults to None"
        );
    }

    #[test]
    fn turn_result_timing_roundtrips_json() {
        let timing = crate::TurnTiming {
            time_to_first_token_ms: Some(800),
            total_ms: 3000,
            output_tokens: Some(120),
            decode_ms: Some(2200),
        };
        let json = serde_json::to_value(bus_turn_result(Some(timing.clone()))).unwrap();
        let back: TurnResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.timing, Some(timing));
    }

    #[test]
    fn turn_request_extension_tools_default_empty() {
        let json = serde_json::json!({
            "prompt": "hi",
            "conversation_id": Uuid::new_v4().to_string(),
        });
        let req: TurnRequest = serde_json::from_value(json).unwrap();
        assert!(req.extension_tools.is_empty());
        assert!(
            !req.interactive,
            "a request without the flag is not interactive"
        );
    }

    #[test]
    fn turn_request_without_branch_fields_appends() {
        let json = serde_json::json!({
            "prompt": "hi",
            "conversation_id": Uuid::new_v4().to_string(),
        });
        let req: TurnRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.anchor, TurnAnchor::Append, "default anchor appends");
        assert_eq!(req.user_message_id, None);
        assert_eq!(req.model_override, None);
    }

    #[test]
    fn turn_request_branch_fields_roundtrip_json() {
        let parent = Uuid::new_v4();
        let user_message_id = Uuid::new_v4();
        for anchor in [
            TurnAnchor::Append,
            TurnAnchor::Edit {
                parent_message_id: Some(parent),
                edited_message_id: user_message_id,
            },
            TurnAnchor::Edit {
                parent_message_id: None,
                edited_message_id: user_message_id,
            },
            TurnAnchor::Regenerate {
                user_message_id: parent,
            },
        ] {
            let req = TurnRequest {
                prompt: "hi".into(),
                conversation_id: Uuid::new_v4(),
                extension_tools: vec![],
                timestamp: None,
                traceparent: None,
                attachment_ids: vec![],
                interactive: false,
                relay_events: false,
                anchor: anchor.clone(),
                user_message_id: Some(user_message_id),
                model_override: Some("gpt-x".into()),
                user_id: None,
                org_id: None,
                space_id: None,
            };
            let back: TurnRequest =
                serde_json::from_value(serde_json::to_value(&req).unwrap()).unwrap();
            assert_eq!(back.anchor, anchor);
            assert_eq!(back.user_message_id, Some(user_message_id));
            assert_eq!(back.model_override.as_deref(), Some("gpt-x"));
        }
    }

    #[test]
    fn turn_result_roundtrips_json() {
        let res = TurnResult {
            conversation_id: Uuid::new_v4(),
            content: "the answer is 42".into(),
            turn: 3,
            attachment_ids: vec![],
            message_id: None,
            had_errors: false,
            timing: None,
        };
        let json = serde_json::to_value(&res).unwrap();
        let back: TurnResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.content, res.content);
        assert_eq!(back.turn, res.turn);
        assert!(back.message_id.is_none());
    }

    #[test]
    fn turn_result_message_id_roundtrips_json() {
        let mid = Uuid::new_v4();
        let res = TurnResult {
            conversation_id: Uuid::new_v4(),
            content: "with id".into(),
            turn: 1,
            attachment_ids: vec![],
            message_id: Some(mid),
            had_errors: false,
            timing: None,
        };
        let json = serde_json::to_value(&res).unwrap();
        // message_id should be present in the serialised form.
        assert_eq!(
            json["message_id"].as_str(),
            Some(mid.to_string().as_str()),
            "message_id should serialise to its UUID string"
        );
        let back: TurnResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.message_id, Some(mid));
    }

    #[test]
    fn turn_result_message_id_absent_deserialises_as_none() {
        // A payload from an older server that omits message_id should still parse.
        let json = serde_json::json!({
            "conversation_id": Uuid::new_v4().to_string(),
            "content": "old server",
            "turn": 0,
        });
        let res: TurnResult = serde_json::from_value(json).unwrap();
        assert!(res.message_id.is_none());
    }

    #[test]
    fn turn_result_had_errors_absent_deserialises_as_false() {
        // Pre-Slice-C payloads omit had_errors; serde(default) must keep
        // that contract so skill-learner gating doesn't see a deserialise
        // error against an older worker.
        let json = serde_json::json!({
            "conversation_id": Uuid::new_v4().to_string(),
            "content": "old server",
            "turn": 0,
        });
        let res: TurnResult = serde_json::from_value(json).unwrap();
        assert!(
            !res.had_errors,
            "had_errors should default to false when absent from JSON"
        );
    }

    #[test]
    fn turn_status_phase_serializes_snake_case() {
        let status = TurnStatus {
            conversation_id: Uuid::new_v4(),
            phase: TurnPhase::CallingTools,
            detail: Some("file-read".into()),
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["phase"], "calling_tools");
        assert_eq!(json["detail"], "file-read");
    }

    #[test]
    fn tool_execute_roundtrips_json() {
        let mut params = HashMap::new();
        params.insert("path".into(), Value::String("/tmp/test".into()));

        let exec = ToolExecute {
            tool_name: "file-read".into(),
            call_id: "call_abc123".into(),
            params,
            conversation_id: Uuid::new_v4(),
            turn: 1,
        };
        let json = serde_json::to_value(&exec).unwrap();
        let back: ToolExecute = serde_json::from_value(json).unwrap();
        assert_eq!(back.tool_name, exec.tool_name);
        assert_eq!(back.call_id, exec.call_id);
        assert_eq!(back.params["path"], "/tmp/test");
    }

    #[test]
    fn tool_result_roundtrips_json() {
        let res = ToolResult {
            tool_name: "bash".into(),
            call_id: "call_xyz".into(),
            content: "command output".into(),
            success: true,
            data: Some(serde_json::json!({"exit_code": 0})),
        };
        let json = serde_json::to_value(&res).unwrap();
        let back: ToolResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.tool_name, "bash");
        assert!(back.success);
        assert_eq!(back.data.unwrap()["exit_code"], 0);
    }

    #[test]
    fn tool_result_data_defaults_none() {
        let json = serde_json::json!({
            "tool_name": "bash",
            "call_id": "c1",
            "content": "ok",
            "success": true,
        });
        let res: ToolResult = serde_json::from_value(json).unwrap();
        assert!(res.data.is_none());
    }

    #[test]
    fn agent_spawn_roundtrips_json() {
        let spawn = AgentSpawn {
            agent_id: "research-1".into(),
            task: "Find recent papers on RLHF".into(),
            system_prompt: Some("You are a research agent.".into()),
            model: None,
            allowed_tools: vec!["web-fetch".into(), "file-write".into()],
            persona_bound: true,
            parent_conversation_id: Some(Uuid::new_v4()),
            parent_agent_id: Some("default".into()),
        };
        let json = serde_json::to_value(&spawn).unwrap();
        let back: AgentSpawn = serde_json::from_value(json).unwrap();
        assert_eq!(back.agent_id, "research-1");
        assert_eq!(back.allowed_tools.len(), 2);
        assert!(back.system_prompt.is_some());
        assert!(back.model.is_none());
        assert!(back.persona_bound);
        assert!(back.parent_conversation_id.is_some());
        assert_eq!(back.parent_agent_id.as_deref(), Some("default"));
    }

    #[test]
    fn agent_spawn_optionals_default_none() {
        let json = serde_json::json!({
            "agent_id": "a1",
            "task": "do something",
        });
        let spawn: AgentSpawn = serde_json::from_value(json).unwrap();
        assert!(spawn.system_prompt.is_none());
        assert!(spawn.model.is_none());
        assert!(spawn.allowed_tools.is_empty());
        assert!(!spawn.persona_bound);
        assert!(spawn.parent_conversation_id.is_none());
        assert!(spawn.parent_agent_id.is_none());
    }

    #[test]
    fn agent_report_status_serializes_snake_case() {
        let report = AgentReport {
            status: AgentReportStatus::Completed,
            content: "done".into(),
            data: None,
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["status"], "completed");
    }

    #[test]
    fn agent_report_roundtrips_json() {
        let report = AgentReport {
            status: AgentReportStatus::Failed,
            content: "connection timed out".into(),
            data: Some(serde_json::json!({"retries": 3})),
        };
        let json = serde_json::to_value(&report).unwrap();
        let back: AgentReport = serde_json::from_value(json).unwrap();
        assert_eq!(back.status, AgentReportStatus::Failed);
        assert_eq!(back.content, "connection timed out");
        assert_eq!(back.data.unwrap()["retries"], 3);
    }
}
