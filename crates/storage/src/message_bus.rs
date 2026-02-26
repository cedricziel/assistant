//! SQLite-backed [`MessageBus`] implementation.
//!
//! Uses a single `bus_messages` table as a durable, topic-based work queue.
//! Claim semantics rely on SQLite's serialised writes — `BEGIN IMMEDIATE`
//! ensures exactly one worker wins the race for a given message.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use tracing::debug;
use uuid::Uuid;

use assistant_core::{BusMessage, MessageBus, MessageStatus};

/// SQLite-backed message bus.
pub struct SqliteMessageBus {
    pool: SqlitePool,
}

impl SqliteMessageBus {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MessageBus for SqliteMessageBus {
    async fn publish(&self, topic: &str, payload: &Value) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let payload_str = serde_json::to_string(payload)?;
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO bus_messages (id, topic, payload, status, created_at) \
             VALUES (?1, ?2, ?3, 'pending', ?4)",
        )
        .bind(&id_str)
        .bind(topic)
        .bind(&payload_str)
        .bind(now)
        .execute(&self.pool)
        .await?;

        debug!(id = %id, topic = %topic, "published bus message");
        Ok(id)
    }

    async fn publish_for_conversation(
        &self,
        topic: &str,
        payload: &Value,
        conversation_id: Uuid,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let payload_str = serde_json::to_string(payload)?;
        let conv_str = conversation_id.to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO bus_messages (id, topic, payload, status, conversation_id, created_at) \
             VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
        )
        .bind(&id_str)
        .bind(topic)
        .bind(&payload_str)
        .bind(&conv_str)
        .bind(now)
        .execute(&self.pool)
        .await?;

        debug!(id = %id, topic = %topic, conversation_id = %conversation_id, "published bus message");
        Ok(id)
    }

    async fn claim(&self, topic: &str, worker_id: &str) -> Result<Option<BusMessage>> {
        let now = Utc::now();

        // Use a transaction with BEGIN IMMEDIATE to serialise claim attempts.
        let mut tx = self.pool.begin().await?;
        sqlx::query("PRAGMA busy_timeout = 5000;")
            .execute(&mut *tx)
            .await?;

        let row = sqlx::query(
            "UPDATE bus_messages \
             SET status = 'claimed', claimed_at = ?1, claimed_by = ?2 \
             WHERE id = ( \
                 SELECT id FROM bus_messages \
                 WHERE topic = ?3 AND status = 'pending' \
                 ORDER BY created_at ASC \
                 LIMIT 1 \
             ) \
             RETURNING id, topic, payload, status, conversation_id, \
                       created_at, claimed_at, claimed_by",
        )
        .bind(now)
        .bind(worker_id)
        .bind(topic)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        match row {
            Some(r) => {
                let msg = parse_row(r)?;
                debug!(id = %msg.id, topic = %topic, worker = %worker_id, "claimed bus message");
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }

    async fn ack(&self, message_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE bus_messages SET status = 'done' WHERE id = ?1")
            .bind(message_id.to_string())
            .execute(&self.pool)
            .await?;
        debug!(id = %message_id, "acked bus message");
        Ok(())
    }

    async fn nack(&self, message_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE bus_messages SET status = 'pending', claimed_at = NULL, claimed_by = NULL \
             WHERE id = ?1",
        )
        .bind(message_id.to_string())
        .execute(&self.pool)
        .await?;
        debug!(id = %message_id, "nacked bus message");
        Ok(())
    }

    async fn fail(&self, message_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE bus_messages SET status = 'failed' WHERE id = ?1")
            .bind(message_id.to_string())
            .execute(&self.pool)
            .await?;
        debug!(id = %message_id, "failed bus message");
        Ok(())
    }

    async fn list(
        &self,
        topic: &str,
        status: Option<MessageStatus>,
        limit: u32,
    ) -> Result<Vec<BusMessage>> {
        let rows = match status {
            Some(s) => {
                sqlx::query(
                    "SELECT id, topic, payload, status, conversation_id, \
                            created_at, claimed_at, claimed_by \
                     FROM bus_messages \
                     WHERE topic = ?1 AND status = ?2 \
                     ORDER BY created_at ASC \
                     LIMIT ?3",
                )
                .bind(topic)
                .bind(s.to_string())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT id, topic, payload, status, conversation_id, \
                            created_at, claimed_at, claimed_by \
                     FROM bus_messages \
                     WHERE topic = ?1 \
                     ORDER BY created_at ASC \
                     LIMIT ?2",
                )
                .bind(topic)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };

        rows.into_iter().map(parse_row).collect()
    }

    async fn reap_stale(&self, timeout: Duration) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::from_std(timeout)?;

        let result = sqlx::query(
            "UPDATE bus_messages \
             SET status = 'pending', claimed_at = NULL, claimed_by = NULL \
             WHERE status = 'claimed' AND claimed_at < ?1",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        let count = result.rows_affected();
        if count > 0 {
            debug!(count = count, "reaped stale bus messages");
        }
        Ok(count)
    }

    async fn purge(&self, older_than: DateTime<Utc>) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM bus_messages WHERE status = 'done' AND created_at < ?1")
                .bind(older_than)
                .execute(&self.pool)
                .await?;

        let count = result.rows_affected();
        if count > 0 {
            debug!(count = count, "purged completed bus messages");
        }
        Ok(count)
    }
}

// -- Row parsing ------------------------------------------------------------

fn parse_row(r: sqlx::sqlite::SqliteRow) -> Result<BusMessage> {
    let raw_id: String = r.get("id");
    let raw_payload: String = r.get("payload");
    let raw_status: String = r.get("status");
    let raw_conv: Option<String> = r.get("conversation_id");

    Ok(BusMessage {
        id: Uuid::parse_str(&raw_id)?,
        topic: r.get("topic"),
        payload: serde_json::from_str(&raw_payload)?,
        status: MessageStatus::parse(&raw_status)?,
        conversation_id: raw_conv.map(|s| Uuid::parse_str(&s)).transpose()?,
        created_at: r.get("created_at"),
        claimed_at: r.get("claimed_at"),
        claimed_by: r.get("claimed_by"),
    })
}

// -- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    async fn bus() -> (StorageLayer, SqliteMessageBus) {
        let s = StorageLayer::new_in_memory().await.unwrap();
        let b = s.message_bus();
        (s, b)
    }

    #[tokio::test]
    async fn test_publish_and_list() {
        let (_s, bus) = bus().await;
        let payload = serde_json::json!({"action": "greet", "name": "world"});

        let id = bus.publish("test.topic", &payload).await.unwrap();

        let msgs = bus.list("test.topic", None, 10).await.unwrap();
        assert_eq!(msgs.len(), 1, "should have one message");
        assert_eq!(msgs[0].id, id);
        assert_eq!(msgs[0].topic, "test.topic");
        assert_eq!(msgs[0].status, MessageStatus::Pending);
        assert_eq!(msgs[0].payload, payload);
        assert!(msgs[0].conversation_id.is_none());
    }

    #[tokio::test]
    async fn test_publish_for_conversation() {
        let (_s, bus) = bus().await;
        let conv_id = Uuid::new_v4();
        let payload = serde_json::json!({"turn": 1});

        bus.publish_for_conversation("turn.request", &payload, conv_id)
            .await
            .unwrap();

        let msgs = bus.list("turn.request", None, 10).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].conversation_id, Some(conv_id));
    }

    #[tokio::test]
    async fn test_claim_returns_oldest_first() {
        let (_s, bus) = bus().await;

        bus.publish("q", &serde_json::json!({"seq": 1}))
            .await
            .unwrap();
        bus.publish("q", &serde_json::json!({"seq": 2}))
            .await
            .unwrap();

        let first = bus.claim("q", "w1").await.unwrap().unwrap();
        assert_eq!(first.payload["seq"], 1, "should claim oldest first");
        assert_eq!(first.status, MessageStatus::Claimed);
        assert_eq!(first.claimed_by.as_deref(), Some("w1"));

        let second = bus.claim("q", "w2").await.unwrap().unwrap();
        assert_eq!(second.payload["seq"], 2);
    }

    #[tokio::test]
    async fn test_claim_returns_none_when_empty() {
        let (_s, bus) = bus().await;
        let result = bus.claim("empty.topic", "w1").await.unwrap();
        assert!(result.is_none(), "should return None on empty topic");
    }

    #[tokio::test]
    async fn test_claim_skips_already_claimed() {
        let (_s, bus) = bus().await;

        bus.publish("q", &serde_json::json!({"only": true}))
            .await
            .unwrap();

        bus.claim("q", "w1").await.unwrap().unwrap();

        let second = bus.claim("q", "w2").await.unwrap();
        assert!(
            second.is_none(),
            "already-claimed message should not be re-claimed"
        );
    }

    #[tokio::test]
    async fn test_ack_sets_done() {
        let (_s, bus) = bus().await;

        let id = bus.publish("q", &serde_json::json!({})).await.unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.ack(id).await.unwrap();

        let msgs = bus.list("q", Some(MessageStatus::Done), 10).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].id, id);
    }

    #[tokio::test]
    async fn test_nack_releases_for_retry() {
        let (_s, bus) = bus().await;

        let id = bus.publish("q", &serde_json::json!({})).await.unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.nack(id).await.unwrap();

        // Should be claimable again
        let reclaimed = bus.claim("q", "w2").await.unwrap().unwrap();
        assert_eq!(reclaimed.id, id);
        assert_eq!(reclaimed.claimed_by.as_deref(), Some("w2"));
    }

    #[tokio::test]
    async fn test_fail_marks_permanent() {
        let (_s, bus) = bus().await;

        let id = bus.publish("q", &serde_json::json!({})).await.unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.fail(id).await.unwrap();

        let msgs = bus
            .list("q", Some(MessageStatus::Failed), 10)
            .await
            .unwrap();
        assert_eq!(msgs.len(), 1);

        // Should not be claimable
        let next = bus.claim("q", "w2").await.unwrap();
        assert!(next.is_none(), "failed messages must not be claimable");
    }

    #[tokio::test]
    async fn test_list_filters_by_status() {
        let (_s, bus) = bus().await;

        bus.publish("q", &serde_json::json!({"a": 1}))
            .await
            .unwrap();
        let id2 = bus
            .publish("q", &serde_json::json!({"a": 2}))
            .await
            .unwrap();
        bus.claim("q", "w1").await.unwrap(); // claims first
        bus.claim("q", "w1").await.unwrap(); // claims second
        bus.ack(id2).await.unwrap();

        let pending = bus
            .list("q", Some(MessageStatus::Pending), 10)
            .await
            .unwrap();
        assert_eq!(pending.len(), 0);

        let claimed = bus
            .list("q", Some(MessageStatus::Claimed), 10)
            .await
            .unwrap();
        assert_eq!(claimed.len(), 1);

        let done = bus.list("q", Some(MessageStatus::Done), 10).await.unwrap();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].id, id2);
    }

    #[tokio::test]
    async fn test_list_different_topics_are_isolated() {
        let (_s, bus) = bus().await;

        bus.publish("topic.a", &serde_json::json!({}))
            .await
            .unwrap();
        bus.publish("topic.b", &serde_json::json!({}))
            .await
            .unwrap();

        let a = bus.list("topic.a", None, 10).await.unwrap();
        assert_eq!(a.len(), 1);

        let b = bus.list("topic.b", None, 10).await.unwrap();
        assert_eq!(b.len(), 1);
    }

    #[tokio::test]
    async fn test_reap_stale_reclaims_old_messages() {
        let (_s, bus) = bus().await;

        let id = bus.publish("q", &serde_json::json!({})).await.unwrap();
        bus.claim("q", "w1").await.unwrap();

        // Manually backdate the claimed_at to simulate staleness
        sqlx::query(
            "UPDATE bus_messages SET claimed_at = datetime('now', '-1 hour') WHERE id = ?1",
        )
        .bind(id.to_string())
        .execute(&bus.pool)
        .await
        .unwrap();

        let reaped = bus.reap_stale(Duration::from_secs(60)).await.unwrap();
        assert_eq!(reaped, 1, "should reap one stale message");

        // Should be claimable again
        let reclaimed = bus.claim("q", "w2").await.unwrap();
        assert!(reclaimed.is_some(), "reaped message should be claimable");
    }

    #[tokio::test]
    async fn test_reap_stale_ignores_fresh_claims() {
        let (_s, bus) = bus().await;

        bus.publish("q", &serde_json::json!({})).await.unwrap();
        bus.claim("q", "w1").await.unwrap();

        let reaped = bus.reap_stale(Duration::from_secs(3600)).await.unwrap();
        assert_eq!(reaped, 0, "fresh claims should not be reaped");
    }

    #[tokio::test]
    async fn test_purge_removes_old_done_messages() {
        let (_s, bus) = bus().await;

        let id = bus.publish("q", &serde_json::json!({})).await.unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.ack(id).await.unwrap();

        // Backdate the created_at
        sqlx::query(
            "UPDATE bus_messages SET created_at = datetime('now', '-2 days') WHERE id = ?1",
        )
        .bind(id.to_string())
        .execute(&bus.pool)
        .await
        .unwrap();

        let cutoff = Utc::now() - chrono::Duration::days(1);
        let purged = bus.purge(cutoff).await.unwrap();
        assert_eq!(purged, 1, "should purge one old done message");

        let msgs = bus.list("q", None, 10).await.unwrap();
        assert!(msgs.is_empty(), "purged message should be gone");
    }

    #[tokio::test]
    async fn test_purge_keeps_recent_done_messages() {
        let (_s, bus) = bus().await;

        let id = bus.publish("q", &serde_json::json!({})).await.unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.ack(id).await.unwrap();

        let cutoff = Utc::now() - chrono::Duration::days(1);
        let purged = bus.purge(cutoff).await.unwrap();
        assert_eq!(purged, 0, "recent done messages should not be purged");
    }
}
