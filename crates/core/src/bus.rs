//! Core message bus abstraction.
//!
//! Defines the [`MessageBus`] trait and supporting types for a durable,
//! topic-based message queue.  The trait is backend-agnostic — the default
//! implementation uses SQLite (in `assistant-storage`), but can be swapped
//! for NATS, Redis Streams, or any other broker by providing an alternative
//! implementation.

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

impl MessageStatus {
    /// Parse a status string (case-insensitive).
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "claimed" => Ok(Self::Claimed),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            other => anyhow::bail!("unknown message status: {other}"),
        }
    }
}

// -- Bus Message ------------------------------------------------------------

/// A single message on the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusMessage {
    pub id: Uuid,
    pub topic: String,
    pub payload: Value,
    pub status: MessageStatus,
    /// Optional correlation with a conversation.
    pub conversation_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub claimed_by: Option<String>,
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
#[async_trait]
pub trait MessageBus: Send + Sync {
    /// Publish a message to a topic.
    ///
    /// Returns the ID of the newly created message.
    async fn publish(&self, topic: &str, payload: &Value) -> Result<Uuid>;

    /// Publish a message correlated with a specific conversation.
    async fn publish_for_conversation(
        &self,
        topic: &str,
        payload: &Value,
        conversation_id: Uuid,
    ) -> Result<Uuid>;

    /// Atomically claim the next pending message on `topic`.
    ///
    /// Returns `None` if no messages are pending.  The message transitions
    /// to [`MessageStatus::Claimed`] and must be [`ack`](MessageBus::ack)ed
    /// or [`nack`](MessageBus::nack)ed by the caller.
    async fn claim(&self, topic: &str, worker_id: &str) -> Result<Option<BusMessage>>;

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
