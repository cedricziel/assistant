//! Core message bus abstraction.
//!
//! Defines the [`MessageBus`] trait and supporting types for a durable,
//! topic-based message queue.  The trait is backend-agnostic — the default
//! implementation uses SQLite (in `assistant-storage`), but can be swapped
//! for NATS, Redis Streams, or any other broker by providing an alternative
//! implementation.
//!
//! Messages carry identity, routing, and correlation metadata to support
//! multi-agent, multi-user, multi-interface architectures.  Topics represent
//! *message types* (`turn.request`, `tool.execute`, …); routing is handled
//! via metadata fields and [`ClaimFilter`].

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// -- Message Status ---------------------------------------------------------

/// Lifecycle state of a bus message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    /// Waiting to be claimed by a worker.
    Pending,
    /// Claimed by a worker but not yet acknowledged.
    Claimed,
    /// Successfully processed.
    Done,
    /// Permanently failed (will not be retried).
    Failed,
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageStatus::Pending => write!(f, "pending"),
            MessageStatus::Claimed => write!(f, "claimed"),
            MessageStatus::Done => write!(f, "done"),
            MessageStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for MessageStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "claimed" => Ok(Self::Claimed),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            other => anyhow::bail!("unknown message status: {other}"),
        }
    }
}

impl MessageStatus {
    /// Parse a status string (case-insensitive).
    pub fn parse(s: &str) -> Result<Self> {
        s.parse()
    }
}

// -- Bus Message ------------------------------------------------------------

/// A single message on the bus.
///
/// Carries identity, routing, and correlation metadata alongside the payload
/// so that consumers can filter by agent, user, conversation, or batch
/// without inspecting the payload body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusMessage {
    pub id: Uuid,
    pub topic: String,
    pub payload: Value,
    pub status: MessageStatus,

    // -- identity --
    /// Which user initiated the chain of work.
    pub user_id: Option<String>,
    /// Which agent produced or should consume this message.
    pub agent_id: Option<String>,

    // -- routing --
    /// Conversation thread this message belongs to.
    pub conversation_id: Option<Uuid>,
    /// Originating interface (`cli`, `slack`, `mattermost`, `signal`, `mcp`).
    pub interface: Option<String>,
    /// Topic the consumer should publish its response to.
    pub reply_to: Option<String>,

    // -- correlation --
    /// Traces the entire request chain from the initial user action.
    pub correlation_id: Option<Uuid>,
    /// Links to the specific message that caused this one.
    pub causation_id: Option<Uuid>,
    /// Groups parallel fan-out messages (e.g. N tool calls in one iteration).
    pub batch_id: Option<Uuid>,

    // -- lifecycle --
    pub created_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub claimed_by: Option<String>,
}

// -- Publish Request (builder) ----------------------------------------------

/// Builder for publishing a message to the bus.
///
/// # Examples
///
/// ```rust,ignore
/// let id = bus.publish(
///     PublishRequest::new("turn.request", json!({"prompt": "hello"}))
///         .with_user_id("U123")
///         .with_agent_id("main")
///         .with_conversation_id(conv_id)
///         .with_interface("slack")
///         .with_reply_to("turn.result")
///         .with_correlation_id(corr_id),
/// ).await?;
/// ```
#[derive(Debug, Clone)]
pub struct PublishRequest {
    pub topic: String,
    pub payload: Value,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub conversation_id: Option<Uuid>,
    pub interface: Option<String>,
    pub reply_to: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
}

impl PublishRequest {
    /// Create a new publish request for the given topic and payload.
    pub fn new(topic: impl Into<String>, payload: Value) -> Self {
        Self {
            topic: topic.into(),
            payload,
            user_id: None,
            agent_id: None,
            conversation_id: None,
            interface: None,
            reply_to: None,
            correlation_id: None,
            causation_id: None,
            batch_id: None,
        }
    }

    pub fn with_user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }

    pub fn with_agent_id(mut self, id: impl Into<String>) -> Self {
        self.agent_id = Some(id.into());
        self
    }

    pub fn with_conversation_id(mut self, id: Uuid) -> Self {
        self.conversation_id = Some(id);
        self
    }

    pub fn with_interface(mut self, iface: impl Into<String>) -> Self {
        self.interface = Some(iface.into());
        self
    }

    pub fn with_reply_to(mut self, topic: impl Into<String>) -> Self {
        self.reply_to = Some(topic.into());
        self
    }

    pub fn with_correlation_id(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }

    pub fn with_causation_id(mut self, id: Uuid) -> Self {
        self.causation_id = Some(id);
        self
    }

    pub fn with_batch_id(mut self, id: Uuid) -> Self {
        self.batch_id = Some(id);
        self
    }
}

// -- Claim Filter -----------------------------------------------------------

/// Filter criteria for selective message claiming.
///
/// All fields are optional; only set fields are applied as `AND` conditions.
/// An empty filter matches any pending message on the topic.
#[derive(Debug, Clone, Default)]
pub struct ClaimFilter {
    /// Only claim messages targeted at this agent.
    pub agent_id: Option<String>,
    /// Only claim messages from this user.
    pub user_id: Option<String>,
    /// Only claim messages in this conversation.
    pub conversation_id: Option<Uuid>,
    /// Only claim messages in this batch.
    pub batch_id: Option<Uuid>,
    /// Only claim messages for this interface (e.g. "Slack", "Web").
    pub interface: Option<String>,
}

impl ClaimFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_agent_id(mut self, id: impl Into<String>) -> Self {
        self.agent_id = Some(id.into());
        self
    }

    pub fn with_user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }

    pub fn with_conversation_id(mut self, id: Uuid) -> Self {
        self.conversation_id = Some(id);
        self
    }

    pub fn with_batch_id(mut self, id: Uuid) -> Self {
        self.batch_id = Some(id);
        self
    }

    pub fn with_interface(mut self, iface: impl Into<String>) -> Self {
        self.interface = Some(iface.into());
        self
    }

    /// Returns `true` if no filter criteria are set.
    pub fn is_empty(&self) -> bool {
        self.agent_id.is_none()
            && self.user_id.is_none()
            && self.conversation_id.is_none()
            && self.batch_id.is_none()
            && self.interface.is_none()
    }
}

// -- MessageBus Trait -------------------------------------------------------

/// A durable, topic-based message bus.
///
/// Implementations must be `Send + Sync` so they can be shared via
/// `Arc<dyn MessageBus>` across async tasks.
///
/// # Delivery semantics
///
/// The trait targets **at-least-once** delivery: a claimed message that is
/// not acknowledged within a reasonable timeout should be reclaimed via
/// [`reap_stale`](MessageBus::reap_stale).
///
/// # Routing model
///
/// Topics represent *message types* (e.g. `turn.request`, `tool.execute`).
/// Routing to specific agents, users, or conversations is done via metadata
/// fields on the message and [`ClaimFilter`] on consumption.
#[async_trait]
pub trait MessageBus: Send + Sync {
    /// Publish a message to the bus.
    ///
    /// Returns the ID of the newly created message.
    async fn publish(&self, request: PublishRequest) -> Result<Uuid>;

    /// Atomically claim the next pending message on `topic`.
    ///
    /// Equivalent to `claim_filtered` with an empty [`ClaimFilter`].
    async fn claim(&self, topic: &str, worker_id: &str) -> Result<Option<BusMessage>> {
        self.claim_filtered(topic, worker_id, &ClaimFilter::default())
            .await
    }

    /// Atomically claim the next pending message matching the filter.
    ///
    /// Returns `None` if no messages match.  The message transitions
    /// to [`MessageStatus::Claimed`] and must be [`ack`](MessageBus::ack)ed
    /// or [`nack`](MessageBus::nack)ed by the caller.
    async fn claim_filtered(
        &self,
        topic: &str,
        worker_id: &str,
        filter: &ClaimFilter,
    ) -> Result<Option<BusMessage>>;

    /// Acknowledge successful processing — sets status to [`MessageStatus::Done`].
    async fn ack(&self, message_id: Uuid) -> Result<()>;

    /// Negative acknowledge — releases the message back to
    /// [`MessageStatus::Pending`] so another worker can retry it.
    async fn nack(&self, message_id: Uuid) -> Result<()>;

    /// Mark a message as permanently [`MessageStatus::Failed`].
    async fn fail(&self, message_id: Uuid) -> Result<()>;

    /// Query messages on a topic, optionally filtered by status.
    ///
    /// Results are ordered oldest-first (`created_at ASC`).
    async fn list(
        &self,
        topic: &str,
        status: Option<MessageStatus>,
        limit: u32,
    ) -> Result<Vec<BusMessage>>;

    /// Reclaim messages that were claimed longer than `timeout` ago but
    /// never acknowledged.  Returns the number of messages reset to pending.
    async fn reap_stale(&self, timeout: Duration) -> Result<u64>;

    /// Delete completed (`Done`) messages older than `older_than`.
    /// Returns the number of messages purged.
    async fn purge(&self, older_than: DateTime<Utc>) -> Result<u64>;
}
