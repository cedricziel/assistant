//! Broadcasts conversation mutation events to subscribers.
//!
//! The [`ConversationBroadcast`] trait abstracts the broadcast mechanism so
//! an in-memory implementation can be swapped for a distributed one (e.g.
//! NATS) when multi-process support is needed.

use tokio::sync::broadcast;
use uuid::Uuid;

use crate::ConversationRecord;

/// Events emitted when conversations are mutated.
#[derive(Debug, Clone)]
pub enum ConversationEvent {
    /// A conversation was created or updated.
    Upserted(ConversationRecord),
    /// A conversation was deleted.
    Deleted {
        conversation_id: Uuid,
        agent_id: String,
    },
}

/// Broadcasts conversation mutation events to subscribers.
pub trait ConversationBroadcast: Send + Sync {
    /// Emit a conversation event to all current subscribers.
    fn emit(&self, event: ConversationEvent);

    /// Subscribe to conversation events. Returns a receiver that will
    /// yield all events emitted after this call.
    fn subscribe(&self) -> broadcast::Receiver<ConversationEvent>;
}

/// In-memory broadcaster backed by a `tokio::sync::broadcast` channel.
pub struct InMemoryConversationBroadcaster {
    sender: broadcast::Sender<ConversationEvent>,
}

impl InMemoryConversationBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self { sender }
    }
}

impl Default for InMemoryConversationBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationBroadcast for InMemoryConversationBroadcaster {
    fn emit(&self, event: ConversationEvent) {
        // Ignore send errors — they just mean no active subscribers.
        let _ = self.sender.send(event);
    }

    fn subscribe(&self) -> broadcast::Receiver<ConversationEvent> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscribe_receives_upserted_event() {
        let broadcaster = InMemoryConversationBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        let record = ConversationRecord {
            id: Uuid::new_v4(),
            agent_id: "default".to_string(),
            title: Some("Test".to_string()),
            user_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        broadcaster.emit(ConversationEvent::Upserted(record.clone()));

        let event = rx.recv().await.expect("should receive event");
        match event {
            ConversationEvent::Upserted(conv) => {
                assert_eq!(conv.id, record.id, "conversation id should match");
                assert_eq!(conv.title.as_deref(), Some("Test"), "title should match");
            }
            _ => panic!("expected Upserted event"),
        }
    }

    #[tokio::test]
    async fn test_subscribe_receives_deleted_event() {
        let broadcaster = InMemoryConversationBroadcaster::new();
        let mut rx = broadcaster.subscribe();

        let id = Uuid::new_v4();
        broadcaster.emit(ConversationEvent::Deleted {
            conversation_id: id,
            agent_id: "default".to_string(),
        });

        let event = rx.recv().await.expect("should receive event");
        match event {
            ConversationEvent::Deleted {
                conversation_id,
                agent_id,
            } => {
                assert_eq!(conversation_id, id);
                assert_eq!(agent_id, "default");
            }
            _ => panic!("expected Deleted event"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers_receive_event() {
        let broadcaster = InMemoryConversationBroadcaster::new();
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();

        let record = ConversationRecord {
            id: Uuid::new_v4(),
            agent_id: "default".to_string(),
            title: None,
            user_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        broadcaster.emit(ConversationEvent::Upserted(record.clone()));

        let e1 = rx1.recv().await.expect("subscriber 1 should receive event");
        let e2 = rx2.recv().await.expect("subscriber 2 should receive event");

        match (e1, e2) {
            (ConversationEvent::Upserted(c1), ConversationEvent::Upserted(c2)) => {
                assert_eq!(c1.id, record.id);
                assert_eq!(c2.id, record.id);
            }
            _ => panic!("both subscribers should receive Upserted"),
        }
    }

    #[tokio::test]
    async fn test_emit_without_subscribers_does_not_panic() {
        let broadcaster = InMemoryConversationBroadcaster::new();
        // No subscribers — should not panic.
        broadcaster.emit(ConversationEvent::Deleted {
            conversation_id: Uuid::new_v4(),
            agent_id: "test".to_string(),
        });
    }
}
