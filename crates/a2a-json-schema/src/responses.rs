//! A2A protocol response types.
//!
//! These types correspond to the response messages for each RPC in the
//! `A2AService`.

use serde::{Deserialize, Serialize};

use crate::types::{
    Message, Task, TaskArtifactUpdateEvent, TaskPushNotificationConfig, TaskStatusUpdateEvent,
};

/// Response for the `SendMessage` method.
///
/// Contains either a task or a message (oneof payload in the proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    /// The task created or updated by the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,

    /// A message from the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
}

/// A wrapper for streaming operations encapsulating different response types.
///
/// In SSE, each chunk is one `StreamResponse` serialized as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamResponse {
    /// A Task object containing the current state of the task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,

    /// A Message object containing a message from the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,

    /// An event indicating a task status update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_update: Option<TaskStatusUpdateEvent>,

    /// An event indicating a task artifact update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_update: Option<TaskArtifactUpdateEvent>,
}

impl StreamResponse {
    /// Creates a stream response containing a task.
    pub fn from_task(task: Task) -> Self {
        Self {
            task: Some(task),
            message: None,
            status_update: None,
            artifact_update: None,
        }
    }

    /// Creates a stream response containing a message.
    pub fn from_message(message: Message) -> Self {
        Self {
            task: None,
            message: Some(message),
            status_update: None,
            artifact_update: None,
        }
    }

    /// Creates a stream response containing a status update event.
    pub fn from_status_update(event: TaskStatusUpdateEvent) -> Self {
        Self {
            task: None,
            message: None,
            status_update: Some(event),
            artifact_update: None,
        }
    }

    /// Creates a stream response containing an artifact update event.
    pub fn from_artifact_update(event: TaskArtifactUpdateEvent) -> Self {
        Self {
            task: None,
            message: None,
            status_update: None,
            artifact_update: Some(event),
        }
    }
}

/// Response for the `ListTasks` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksResponse {
    /// Tasks matching the specified criteria.
    pub tasks: Vec<Task>,

    /// A token to retrieve the next page of results.
    pub next_page_token: String,

    /// The page size used for this response.
    pub page_size: i32,

    /// Total number of tasks available (before pagination).
    pub total_size: i32,
}

/// Response for the `ListTaskPushNotificationConfigs` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTaskPushNotificationConfigsResponse {
    /// The list of push notification configurations.
    pub configs: Vec<TaskPushNotificationConfig>,

    /// A token to retrieve the next page of results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Part, Role, TaskState, TaskStatus};

    #[test]
    fn test_send_message_response_with_task() {
        let resp = SendMessageResponse {
            task: Some(Task {
                id: "task-001".to_string(),
                context_id: "ctx-001".to_string(),
                status: TaskStatus {
                    state: TaskState::TaskStateSubmitted,
                    message: None,
                    timestamp: None,
                },
                artifacts: vec![],
                history: vec![],
                metadata: None,
            }),
            message: None,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("task-001"));
        assert!(!json.contains("\"message\""));
    }

    #[test]
    fn test_stream_response_constructors() {
        let msg = Message {
            message_id: "msg-001".to_string(),
            context_id: None,
            task_id: None,
            role: Role::RoleAgent,
            parts: vec![Part::text("hello")],
            metadata: None,
            extensions: vec![],
            reference_task_ids: vec![],
        };

        let sr = StreamResponse::from_message(msg);
        assert!(sr.message.is_some());
        assert!(sr.task.is_none());
        assert!(sr.status_update.is_none());
        assert!(sr.artifact_update.is_none());
    }
}
