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
    pub const TURN_RESULT: &str = "turn.result";
    pub const TURN_STATUS: &str = "turn.status";
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
    /// File attachments collected from tool outputs during the turn.
    #[serde(default)]
    pub attachments: Vec<crate::Attachment>,
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
        };
        let json = serde_json::to_value(&req).unwrap();
        let back: TurnRequest = serde_json::from_value(json).unwrap();
        assert_eq!(back.prompt, req.prompt);
        assert_eq!(back.conversation_id, req.conversation_id);
        assert_eq!(back.extension_tools, req.extension_tools);
    }

    #[test]
    fn turn_request_extension_tools_default_empty() {
        let json = serde_json::json!({
            "prompt": "hi",
            "conversation_id": Uuid::new_v4().to_string(),
        });
        let req: TurnRequest = serde_json::from_value(json).unwrap();
        assert!(req.extension_tools.is_empty());
    }

    #[test]
    fn turn_result_roundtrips_json() {
        let res = TurnResult {
            conversation_id: Uuid::new_v4(),
            content: "the answer is 42".into(),
            turn: 3,
            attachments: vec![],
        };
        let json = serde_json::to_value(&res).unwrap();
        let back: TurnResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.content, res.content);
        assert_eq!(back.turn, res.turn);
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
        };
        let json = serde_json::to_value(&spawn).unwrap();
        let back: AgentSpawn = serde_json::from_value(json).unwrap();
        assert_eq!(back.agent_id, "research-1");
        assert_eq!(back.allowed_tools.len(), 2);
        assert!(back.system_prompt.is_some());
        assert!(back.model.is_none());
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
