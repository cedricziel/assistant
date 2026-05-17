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
    pub turn: i64,
    pub created_at: DateTime<Utc>,
    /// The user who sent this message (multi-user scoping).
    /// `None` for assistant/system/tool messages and legacy data.
    pub sender_user_id: Option<String>,
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
            turn: 0,
            created_at: SystemClock.now(),
            sender_user_id: None,
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
    /// Whether the skill can prompt the user for confirmation
    pub interactive: bool,
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
/// orchestrator turn pipeline.
///
/// Callers that have an authenticated user populate this from
/// [`AuthContext`](crate::auth::AuthContext); legacy / system turns use
/// [`Default`].
#[derive(Debug, Clone, Default)]
pub struct TurnIdentity {
    pub user_id: Option<crate::identity::UserId>,
    pub org_id: Option<crate::identity::OrgId>,
    pub space_id: Option<crate::identity::SpaceId>,
}

impl TurnIdentity {
    /// Build a `TurnIdentity` from an [`AuthContext`](crate::auth::AuthContext).
    pub fn from_auth(auth: &crate::auth::AuthContext) -> Self {
        Self {
            user_id: Some(auth.user_id.clone()),
            org_id: Some(auth.org_id.clone()),
            space_id: None,
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
