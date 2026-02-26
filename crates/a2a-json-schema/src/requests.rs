//! A2A protocol request types.
//!
//! These types correspond to the request messages for each RPC in the
//! `A2AService`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{Message, PushNotificationConfig, SendMessageConfiguration, TaskState};

/// Request for the `SendMessage` and `SendStreamingMessage` methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    /// Optional tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// The message to send to the agent.
    pub message: Message,

    /// Configuration for the send request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<SendMessageConfiguration>,

    /// Additional context or parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Request for the `GetTask` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskRequest {
    /// Optional tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// The resource ID of the task to retrieve.
    pub id: String,

    /// Max number of recent messages to include in history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
}

/// Request for the `ListTasks` method.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksRequest {
    /// Tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// Filter tasks by context ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,

    /// Filter tasks by current status state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskState>,

    /// Max number of tasks to return (1..=100, default 50).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,

    /// Page token from a previous `ListTasks` call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,

    /// Max number of messages to include in each task's history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,

    /// Filter tasks with status updated after this timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_timestamp_after: Option<DateTime<Utc>>,

    /// Whether to include artifacts in returned tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_artifacts: Option<bool>,
}

/// Request for the `CancelTask` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskRequest {
    /// Optional tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// The resource ID of the task to cancel.
    pub id: String,

    /// Additional context or parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Request for the `SubscribeToTask` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeToTaskRequest {
    /// Optional tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// The resource ID of the task to subscribe to.
    pub id: String,
}

/// Request for the `CreateTaskPushNotificationConfig` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskPushNotificationConfigRequest {
    /// Optional tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// The parent task resource ID.
    pub task_id: String,

    /// The configuration to create.
    pub config: PushNotificationConfig,
}

/// Request for the `GetTaskPushNotificationConfig` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskPushNotificationConfigRequest {
    /// Optional tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// The parent task resource ID.
    pub task_id: String,

    /// The resource ID of the configuration to retrieve.
    pub id: String,
}

/// Request for the `DeleteTaskPushNotificationConfig` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTaskPushNotificationConfigRequest {
    /// Optional tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// The parent task resource ID.
    pub task_id: String,

    /// The resource ID of the configuration to delete.
    pub id: String,
}

/// Request for the `ListTaskPushNotificationConfigs` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTaskPushNotificationConfigsRequest {
    /// Optional tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// The parent task resource ID.
    pub task_id: String,

    /// Max number of configurations to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,

    /// Page token from a previous call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

/// Request for the `GetExtendedAgentCard` method.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExtendedAgentCardRequest {
    /// Optional tenant ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}
