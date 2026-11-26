//! Core conversation types: messages, roles, execution context, turn
//! identity, and interface/channel discriminators.

use chrono::{DateTime, Utc};

use crate::clock::{Clock, SystemClock};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Role of a message in the conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

/// A single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    /// For tool messages: which skill produced this result.
    pub skill_name: Option<String>,
    /// For assistant messages that contain tool calls: the serialised
    /// `Vec<ToolCallItem>` JSON.  Populated when the LLM response was a
    /// `ToolCalls` variant; `None` for plain text messages.
    pub tool_calls_json: Option<String>,
    /// For assistant tool-call messages: serialised
    /// [`ProviderState`](crate::ProviderState) to send back on later calls.
    pub provider_state_json: Option<String>,
    pub turn: i64,
    pub created_at: DateTime<Utc>,
    /// The user who sent this message (multi-user scoping).
    /// `None` for assistant/system/tool messages and legacy data.
    pub sender_user_id: Option<String>,
    /// The message this one answers (its predecessor on the conversation
    /// tree). `None` marks a root. Set by the store when the message is saved.
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    /// The turn run that wrote this row: the request id when present,
    /// otherwise the bus message id. `None` for legacy rows.
    #[serde(default)]
    pub run_id: Option<Uuid>,
    /// For tool messages: the provider-supplied id of the call this row
    /// answers, used to join it to its `tool_calls_json` entry.
    #[serde(default)]
    pub tool_call_id: Option<String>,
    /// For tool messages: `ok`, `error` or `denied`. `None` for legacy rows.
    #[serde(default)]
    pub tool_status: Option<String>,
    /// For tool messages: the serialised structured render data (capped by
    /// dispatch). Client-only; never sent to the model.
    #[serde(default)]
    pub tool_data_json: Option<String>,
    /// For tool messages: the serialised `McpApp` the tool declared.
    /// Client-only; never sent to the model.
    #[serde(default)]
    pub tool_app_json: Option<String>,
    /// For final assistant answers: the W3C `traceparent` of the LLM span
    /// that produced the reply. `None` when tracing was off or for legacy rows.
    #[serde(default)]
    pub traceparent: Option<String>,
    /// For final assistant answers: the model that produced the reply.
    /// `None` for user and tool rows and for legacy rows.
    #[serde(default)]
    pub model: Option<String>,
    /// Why this persisted reply ended early. `None` for an ordinary message.
    #[serde(default)]
    pub finish_reason: Option<MessageFinishReason>,
}

/// Why a persisted assistant reply ended, when it did not end normally.
///
/// Not to be confused with the LLM response's `finish_reason`, which says why
/// the model stopped generating. This one describes the stored message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageFinishReason {
    /// The user stopped the turn; the content is the partial reply.
    Cancelled,
}

impl MessageFinishReason {
    /// The value stored in `messages.finish_reason` and returned by the API.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse a stored value. An unknown value reads as `None` and is logged,
    /// so a newer writer's rows do not break an older reader.
    pub fn from_stored(value: &str) -> Option<Self> {
        match value {
            "cancelled" => Some(Self::Cancelled),
            other => {
                tracing::warn!(finish_reason = %other, "unknown message finish_reason, ignoring");
                None
            }
        }
    }
}

impl Message {
    pub fn new(conversation_id: Uuid, role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role,
            content: content.into(),
            skill_name: None,
            tool_calls_json: None,
            provider_state_json: None,
            turn: 0,
            created_at: SystemClock.now(),
            sender_user_id: None,
            parent_id: None,
            run_id: None,
            tool_call_id: None,
            tool_status: None,
            tool_data_json: None,
            tool_app_json: None,
            traceparent: None,
            model: None,
            finish_reason: None,
        }
    }

    pub fn user(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self::new(conversation_id, MessageRole::User, content)
    }

    pub fn assistant(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self::new(conversation_id, MessageRole::Assistant, content)
    }
}

/// Default maximum subagent nesting depth.
pub const DEFAULT_MAX_AGENT_DEPTH: u32 = 5;

/// Runtime context passed to every skill execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub conversation_id: Uuid,
    /// Active assistant agent context ID.
    pub agent_id: String,
    pub turn: i64,
    /// The interface this turn originated from (cli, signal, mcp)
    pub interface: Interface,
    /// Whether a human is waiting on this turn and can answer prompts. Set by
    /// the submitter (`SubmitTurn`), never derived from `interface`.
    pub interactive: bool,
    /// The run this turn belongs to: the bus `batch_id` of its request, which
    /// is also the SSE `run_id` for streaming web turns. `None` for tool
    /// calls outside a bus-submitted turn (scheduler, tests).
    pub run_id: Option<Uuid>,
    /// When `Some`, only tools whose names appear in this list may be executed.
    /// `None` means all registered tools are available (the default).
    pub allowed_tools: Option<Vec<String>>,
    /// Current subagent nesting depth.  The root agent has depth `0`.
    pub depth: u32,

    // -- Identity fields (multi-user) ----------------------------------------
    /// The authenticated user executing this turn. `None` for legacy
    /// single-user or system-originated turns.
    pub user_id: Option<crate::identity::UserId>,
    /// The organization context. `None` for legacy single-user mode.
    pub org_id: Option<crate::identity::OrgId>,
    /// The space context. `None` for legacy single-user mode.
    pub space_id: Option<crate::identity::SpaceId>,
}

impl ExecutionContext {
    /// Populate the identity fields from an [`AuthContext`](crate::auth::AuthContext).
    ///
    /// Copies `user_id` and `org_id` directly; leaves `space_id` as-is since
    /// it depends on the routing context rather than the token.
    pub fn with_auth(mut self, auth: &crate::auth::AuthContext) -> Self {
        self.user_id = Some(auth.user_id.clone());
        self.org_id = Some(auth.org_id.clone());
        self
    }

    /// Populate the identity fields from a [`TurnIdentity`].
    pub fn with_identity(mut self, id: &TurnIdentity) -> Self {
        self.user_id = id.user_id.clone();
        self.org_id = id.org_id.clone();
        self.space_id = id.space_id.clone();
        self
    }
}

/// Lightweight bundle of optional identity fields threaded through the
/// orchestrator turn pipeline, plus the per-turn settings the worker takes
/// from the bus request (`interactive`, `run_id`, `anchor`, `user_message_id`).
///
/// Callers that have an authenticated user populate this from
/// [`AuthContext`](crate::auth::AuthContext); legacy / system turns use
/// [`Default`].
#[derive(Debug, Clone, Default)]
pub struct TurnIdentity {
    pub user_id: Option<crate::identity::UserId>,
    pub org_id: Option<crate::identity::OrgId>,
    pub space_id: Option<crate::identity::SpaceId>,
    /// Whether a human is waiting on the turn (see [`ExecutionContext::interactive`]).
    pub interactive: bool,
    /// The run id (see [`ExecutionContext::run_id`]).
    pub run_id: Option<Uuid>,
    /// Where the turn's rows attach in the message tree.
    pub anchor: crate::bus_messages::TurnAnchor,
    /// The id the turn persists its user row under. `None` lets the store pick.
    pub user_message_id: Option<Uuid>,
    /// Model for this turn only. Beats the conversation's `/model` override
    /// and the default, and is never stored on the conversation.
    pub model_override: Option<String>,
}

impl TurnIdentity {
    /// Build a `TurnIdentity` from an [`AuthContext`](crate::auth::AuthContext).
    pub fn from_auth(auth: &crate::auth::AuthContext) -> Self {
        Self {
            user_id: Some(auth.user_id.clone()),
            org_id: Some(auth.org_id.clone()),
            ..Self::default()
        }
    }
}

/// Which interface originated the request
#[derive(Debug, Clone, PartialEq)]
pub enum Interface {
    Cli,
    Signal,
    Mcp,
    Slack,
    Mattermost,
    /// Nextcloud Talk webhook-based bot interface.
    Nextcloud,
    /// Web UI chat interface.
    Web,
    /// Background scheduled tasks and heartbeats — non-interactive.
    Scheduler,
    /// Matrix messaging interface.
    Matrix,
    /// Agent-to-Agent (A2A) protocol interface.
    A2a,
}

/// Identifies the messaging platform for a `ChannelAdapter`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    Slack,
    Mattermost,
    Matrix,
    Nextcloud,
    Signal,
    /// Catch-all for future or custom platforms.
    Custom(String),
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Slack => write!(f, "slack"),
            ChannelType::Mattermost => write!(f, "mattermost"),
            ChannelType::Matrix => write!(f, "matrix"),
            ChannelType::Nextcloud => write!(f, "nextcloud"),
            ChannelType::Signal => write!(f, "signal"),
            ChannelType::Custom(name) => write!(f, "{name}"),
        }
    }
}

#[cfg(test)]
mod finish_reason_tests {
    use super::*;

    #[test]
    fn stored_value_round_trips() {
        let stored = MessageFinishReason::Cancelled.as_str();
        assert_eq!(stored, "cancelled");
        assert_eq!(
            MessageFinishReason::from_stored(stored),
            Some(MessageFinishReason::Cancelled)
        );
    }

    #[test]
    fn unknown_stored_value_reads_as_none() {
        assert_eq!(MessageFinishReason::from_stored("something_new"), None);
    }

    #[test]
    fn new_message_has_no_finish_reason() {
        let m = Message::assistant(Uuid::new_v4(), "hi");
        assert_eq!(m.finish_reason, None);
    }
}
