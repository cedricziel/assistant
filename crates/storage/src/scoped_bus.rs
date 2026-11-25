//! A [`MessageBus`] decorator that namespaces every topic.
//!
//! Space databases carry their own SQLite bus, so turns never cross spaces
//! there. A broker shared by every space (NATS) has one subject tree, so each
//! space's runtime reaches it through a `ScopedMessageBus`: publishers and
//! consumers of one scope cannot see the messages of another.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use assistant_core::{BusMessage, ClaimFilter, MessageBus, MessageStatus, PublishRequest};

/// Prefixes every topic with `{scope}.` on the way in and strips it on the way out.
pub struct ScopedMessageBus {
    inner: Arc<dyn MessageBus>,
    prefix: String,
}

impl ScopedMessageBus {
    /// `scope` must be a valid subject token sequence, e.g. `"{org}.{space}"`.
    pub fn new(inner: Arc<dyn MessageBus>, scope: &str) -> Self {
        Self {
            inner,
            prefix: format!("{scope}."),
        }
    }

    fn scoped(&self, topic: &str) -> String {
        format!("{}{topic}", self.prefix)
    }

    fn unscoped(&self, mut message: BusMessage) -> BusMessage {
        if let Some(rest) = message.topic.strip_prefix(&self.prefix) {
            message.topic = rest.to_owned();
        }
        message
    }
}

#[async_trait]
impl MessageBus for ScopedMessageBus {
    async fn publish(&self, mut request: PublishRequest) -> Result<Uuid> {
        request.topic = self.scoped(&request.topic);
        self.inner.publish(request).await
    }

    async fn claim_filtered(
        &self,
        topic: &str,
        worker_id: &str,
        filter: &ClaimFilter,
    ) -> Result<Option<BusMessage>> {
        let claimed = self
            .inner
            .claim_filtered(&self.scoped(topic), worker_id, filter)
            .await?;
        Ok(claimed.map(|m| self.unscoped(m)))
    }

    async fn ack(&self, message_id: Uuid) -> Result<()> {
        self.inner.ack(message_id).await
    }

    async fn nack(&self, message_id: Uuid) -> Result<()> {
        self.inner.nack(message_id).await
    }

    async fn nack_delayed(&self, message_id: Uuid, delay: Duration) -> Result<()> {
        self.inner.nack_delayed(message_id, delay).await
    }

    async fn fail(&self, message_id: Uuid) -> Result<()> {
        self.inner.fail(message_id).await
    }

    async fn list(
        &self,
        topic: &str,
        status: Option<MessageStatus>,
        limit: u32,
    ) -> Result<Vec<BusMessage>> {
        let messages = self.inner.list(&self.scoped(topic), status, limit).await?;
        Ok(messages.into_iter().map(|m| self.unscoped(m)).collect())
    }

    async fn reap_stale(&self, timeout: Duration) -> Result<u64> {
        self.inner.reap_stale(timeout).await
    }

    async fn purge(&self, older_than: DateTime<Utc>) -> Result<u64> {
        self.inner.purge(older_than).await
    }

    async fn has_active_message(&self, conversation_id: &str, topic: &str) -> Result<bool> {
        self.inner
            .has_active_message(conversation_id, &self.scoped(topic))
            .await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::InMemoryMessageBus;

    use super::*;

    fn scoped_pair() -> (Arc<dyn MessageBus>, ScopedMessageBus, ScopedMessageBus) {
        let shared: Arc<dyn MessageBus> = Arc::new(InMemoryMessageBus::new());
        let a = ScopedMessageBus::new(shared.clone(), "acme.alpha");
        let b = ScopedMessageBus::new(shared.clone(), "acme.beta");
        (shared, a, b)
    }

    #[tokio::test]
    async fn a_scope_never_claims_another_scopes_messages() {
        let (_shared, a, b) = scoped_pair();
        b.publish(PublishRequest::new("turn.request", json!({"from": "b"})))
            .await
            .unwrap();

        let claimed = a.claim("turn.request", "worker-a").await.unwrap();
        assert!(claimed.is_none(), "scope alpha must not see beta's turn");

        let claimed = b.claim("turn.request", "worker-b").await.unwrap();
        assert_eq!(claimed.unwrap().payload, json!({"from": "b"}));
    }

    #[tokio::test]
    async fn messages_travel_under_a_prefixed_topic_and_come_back_unprefixed() {
        let (shared, a, _b) = scoped_pair();
        a.publish(PublishRequest::new("turn.result", json!({})))
            .await
            .unwrap();

        let raw = shared
            .list("acme.alpha.turn.result", None, 10)
            .await
            .unwrap();
        assert_eq!(raw.len(), 1, "the shared bus stores the prefixed topic");

        let listed = a.list("turn.result", None, 10).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].topic, "turn.result");
    }

    #[tokio::test]
    async fn active_message_lookup_is_scoped() {
        let (_shared, a, b) = scoped_pair();
        let conv = Uuid::new_v4();
        a.publish(PublishRequest::new("turn.request", json!({})).with_conversation_id(conv))
            .await
            .unwrap();

        assert!(
            a.has_active_message(&conv.to_string(), "turn.request")
                .await
                .unwrap()
        );
        assert!(
            !b.has_active_message(&conv.to_string(), "turn.request")
                .await
                .unwrap()
        );
    }
}
