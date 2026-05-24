//! A2A wire projector.
//!
//! Maps the canonical [`OrchestratorEvent`] stream onto A2A
//! [`StreamResponse`](assistant_a2a_json_schema::responses::StreamResponse)
//! frames for a single task. Implements the shared
//! [`StreamProjector`](assistant_runtime::projection::StreamProjector) trait, so
//! A2A reuses the Phase 0 projection seam without `assistant-runtime` taking a
//! dependency on the A2A wire types.

use assistant_a2a_json_schema::responses::StreamResponse;
use assistant_a2a_json_schema::types::{
    Message, Part, Role, TaskState, TaskStatus, TaskStatusUpdateEvent,
};
use assistant_runtime::OrchestratorEvent;
use assistant_runtime::projection::StreamProjector;
use uuid::Uuid;

/// Projects orchestrator events onto the A2A streaming wire for one task.
///
/// Constructed per task so it can stamp the `task_id` / `context_id` onto each
/// emitted frame.
#[derive(Debug, Clone)]
pub struct A2aProjector {
    task_id: String,
    context_id: String,
}

impl A2aProjector {
    /// Create a projector bound to a task and its context.
    pub fn new(task_id: impl Into<String>, context_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            context_id: context_id.into(),
        }
    }

    /// Build an agent-authored message carrying `text`, scoped to this task.
    fn agent_message(&self, text: impl Into<String>) -> Message {
        Message {
            message_id: Uuid::new_v4().to_string(),
            context_id: Some(self.context_id.clone()),
            task_id: Some(self.task_id.clone()),
            role: Role::RoleAgent,
            parts: vec![Part::text(text)],
            metadata: None,
            extensions: vec![],
            reference_task_ids: vec![],
        }
    }

    /// Build a status-update frame. Timestamps are left to the durable store /
    /// handler (which hold a clock); the projector stays pure and clock-free.
    fn status(&self, state: TaskState, message: Option<Message>) -> StreamResponse {
        StreamResponse::from_status_update(TaskStatusUpdateEvent {
            task_id: self.task_id.clone(),
            context_id: self.context_id.clone(),
            status: TaskStatus {
                state,
                message,
                timestamp: None,
            },
            metadata: None,
        })
    }
}

impl StreamProjector for A2aProjector {
    type Frame = StreamResponse;

    fn project(&self, event: &OrchestratorEvent) -> Vec<StreamResponse> {
        // No catch-all `_` arm: a new OrchestratorEvent variant must fail to
        // compile here until it is explicitly projected onto the A2A wire.
        let frame = match event {
            OrchestratorEvent::Token(token) => {
                StreamResponse::from_message(self.agent_message(token.clone()))
            }
            OrchestratorEvent::Status { message, .. } => self.status(
                TaskState::TaskStateWorking,
                Some(self.agent_message(message.clone())),
            ),
            OrchestratorEvent::ToolResult {
                tool_name, status, ..
            } => self.status(
                TaskState::TaskStateWorking,
                Some(self.agent_message(format!("Tool {tool_name}: {status}"))),
            ),
            OrchestratorEvent::SkillComplete {
                skill_name,
                success,
                summary,
            } => {
                let verb = if *success { "complete" } else { "failed" };
                self.status(
                    TaskState::TaskStateWorking,
                    Some(self.agent_message(format!("Skill {skill_name} {verb}: {summary}"))),
                )
            }
            OrchestratorEvent::AgentError { message } => self.status(
                TaskState::TaskStateFailed,
                Some(self.agent_message(message.clone())),
            ),
            OrchestratorEvent::Thinking(content) => self.status(
                TaskState::TaskStateWorking,
                Some(self.agent_message(content.clone())),
            ),
            OrchestratorEvent::SubagentStarted { agent_id, task } => self.status(
                TaskState::TaskStateWorking,
                Some(self.agent_message(format!("Subagent {agent_id} started: {task}"))),
            ),
            OrchestratorEvent::SubagentCompleted {
                agent_id,
                status,
                summary,
            } => self.status(
                TaskState::TaskStateWorking,
                Some(self.agent_message(format!("Subagent {agent_id} {status}: {summary}"))),
            ),
            // Surface inner subagent events on the same task wire (recurses,
            // so nested SubagentEvents are handled).
            OrchestratorEvent::SubagentEvent { inner, .. } => return self.project(inner),
            OrchestratorEvent::AudioReady { audio_id } => self.status(
                TaskState::TaskStateWorking,
                Some(self.agent_message(format!("Audio ready: {audio_id}"))),
            ),
        };
        vec![frame]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn all_variants() -> Vec<OrchestratorEvent> {
        vec![
            OrchestratorEvent::Token("hi".into()),
            OrchestratorEvent::Status {
                message: "Calling tool".into(),
                tool_call_id: None,
            },
            OrchestratorEvent::ToolResult {
                tool_name: "file-read".into(),
                status: "ok".into(),
                arguments: Some(json!({"path": "/x"})),
                result: Some("contents".into()),
                tool_call_id: None,
            },
            OrchestratorEvent::SkillComplete {
                skill_name: "tdd".into(),
                success: true,
                summary: "done".into(),
            },
            OrchestratorEvent::AgentError {
                message: "boom".into(),
            },
            OrchestratorEvent::Thinking("pondering".into()),
            OrchestratorEvent::SubagentStarted {
                agent_id: "a1".into(),
                task: "research".into(),
            },
            OrchestratorEvent::SubagentCompleted {
                agent_id: "a1".into(),
                status: "ok".into(),
                summary: "found".into(),
            },
            OrchestratorEvent::SubagentEvent {
                agent_id: "a1".into(),
                inner: Box::new(OrchestratorEvent::Token("inner".into())),
            },
            OrchestratorEvent::AudioReady {
                audio_id: "audio-1".into(),
            },
        ]
    }

    #[test]
    fn projection_is_total() {
        let p = A2aProjector::new("task-1", "ctx-1");
        for event in all_variants() {
            assert!(
                !p.project(&event).is_empty(),
                "A2aProjector produced no frame for {event:?}"
            );
        }
    }

    #[test]
    fn token_projects_to_agent_message_scoped_to_task() {
        let p = A2aProjector::new("task-1", "ctx-1");
        let frames = p.project(&OrchestratorEvent::Token("hello".into()));
        let frame = frames.into_iter().next().expect("one frame");
        let msg = frame.message.expect("token projects to a message");
        assert_eq!(msg.role, Role::RoleAgent);
        assert_eq!(msg.task_id.as_deref(), Some("task-1"));
        assert_eq!(msg.context_id.as_deref(), Some("ctx-1"));
        assert_eq!(msg.parts[0].text.as_deref(), Some("hello"));
    }

    #[test]
    fn agent_error_projects_failed_status() {
        let p = A2aProjector::new("task-1", "ctx-1");
        let frames = p.project(&OrchestratorEvent::AgentError {
            message: "boom".into(),
        });
        let update = frames
            .into_iter()
            .next()
            .expect("one frame")
            .status_update
            .expect("agent_error projects to a status update");
        assert!(matches!(update.status.state, TaskState::TaskStateFailed));
    }
}
