//! SQLite-backed [`MessageBus`] implementation.
//!
//! Uses a single `bus_messages` table as a durable, topic-based work queue.
//! Claim semantics rely on SQLite's serialised writes — `BEGIN IMMEDIATE`
//! ensures exactly one worker wins the race for a given message.
//!
//! Routing is handled via metadata columns (`agent_id`, `user_id`,
//! `conversation_id`, `batch_id`) and dynamic WHERE-clause construction
//! in [`claim_filtered`](MessageBus::claim_filtered).

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use tracing::debug;
use uuid::Uuid;

use assistant_core::{BusMessage, ClaimFilter, MessageBus, MessageStatus, PublishRequest};

/// All columns selected in queries — keeps SELECT lists consistent.
const SELECT_COLS: &str = "\
    id, topic, payload, status, \
    user_id, agent_id, \
    conversation_id, interface, reply_to, \
    correlation_id, causation_id, batch_id, \
    created_at, claimed_at, claimed_by";

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
    async fn publish(&self, req: PublishRequest) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let payload_str = serde_json::to_string(&req.payload)?;
        let now = Utc::now();
        let conv_str = req.conversation_id.map(|c| c.to_string());
        let corr_str = req.correlation_id.map(|c| c.to_string());
        let cause_str = req.causation_id.map(|c| c.to_string());
        let batch_str = req.batch_id.map(|b| b.to_string());

        sqlx::query(
            "INSERT INTO bus_messages \
                (id, topic, payload, status, \
                 user_id, agent_id, \
                 conversation_id, interface, reply_to, \
                 correlation_id, causation_id, batch_id, \
                 created_at) \
             VALUES (?1, ?2, ?3, 'pending', \
                     ?4, ?5, \
                     ?6, ?7, ?8, \
                     ?9, ?10, ?11, \
                     ?12)",
        )
        .bind(&id_str)
        .bind(&req.topic)
        .bind(&payload_str)
        .bind(&req.user_id)
        .bind(&req.agent_id)
        .bind(&conv_str)
        .bind(&req.interface)
        .bind(&req.reply_to)
        .bind(&corr_str)
        .bind(&cause_str)
        .bind(&batch_str)
        .bind(now)
        .execute(&self.pool)
        .await?;

        debug!(id = %id, topic = %req.topic, "published bus message");
        Ok(id)
    }

    async fn claim_filtered(
        &self,
        topic: &str,
        worker_id: &str,
        filter: &ClaimFilter,
    ) -> Result<Option<BusMessage>> {
        let now = Utc::now();

        // Build the inner SELECT with optional filter predicates.
        let mut where_clauses = vec!["topic = ?3".to_string(), "status = 'pending'".to_string()];
        // Track bind index — first 3 are: claimed_at, claimed_by, topic
        let mut next_bind: u32 = 4;
        let mut extra_binds: Vec<String> = Vec::new();

        if let Some(ref agent) = filter.agent_id {
            where_clauses.push(format!("agent_id = ?{next_bind}"));
            extra_binds.push(agent.clone());
            next_bind += 1;
        }
        if let Some(ref user) = filter.user_id {
            where_clauses.push(format!("user_id = ?{next_bind}"));
            extra_binds.push(user.clone());
            next_bind += 1;
        }
        if let Some(ref conv) = filter.conversation_id {
            where_clauses.push(format!("conversation_id = ?{next_bind}"));
            extra_binds.push(conv.to_string());
            next_bind += 1;
        }
        if let Some(ref batch) = filter.batch_id {
            where_clauses.push(format!("batch_id = ?{next_bind}"));
            extra_binds.push(batch.to_string());
            // next_bind not needed after last, but keep for consistency
            let _ = next_bind;
        }

        let where_sql = where_clauses.join(" AND ");
        let sql = format!(
            "UPDATE bus_messages \
             SET status = 'claimed', claimed_at = ?1, claimed_by = ?2 \
             WHERE id = ( \
                 SELECT id FROM bus_messages \
                 WHERE {where_sql} \
                 ORDER BY created_at ASC \
                 LIMIT 1 \
             ) \
             RETURNING {SELECT_COLS}"
        );

        let mut tx = self.pool.begin().await?;
        sqlx::query("PRAGMA busy_timeout = 5000;")
            .execute(&mut *tx)
            .await?;

        let mut query = sqlx::query(&sql).bind(now).bind(worker_id).bind(topic);

        for val in &extra_binds {
            query = query.bind(val);
        }

        let row = query.fetch_optional(&mut *tx).await?;
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
                let sql = format!(
                    "SELECT {SELECT_COLS} FROM bus_messages \
                     WHERE topic = ?1 AND status = ?2 \
                     ORDER BY created_at ASC LIMIT ?3"
                );
                sqlx::query(&sql)
                    .bind(topic)
                    .bind(s.to_string())
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                let sql = format!(
                    "SELECT {SELECT_COLS} FROM bus_messages \
                     WHERE topic = ?1 \
                     ORDER BY created_at ASC LIMIT ?2"
                );
                sqlx::query(&sql)
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
    let raw_corr: Option<String> = r.get("correlation_id");
    let raw_cause: Option<String> = r.get("causation_id");
    let raw_batch: Option<String> = r.get("batch_id");

    Ok(BusMessage {
        id: Uuid::parse_str(&raw_id)?,
        topic: r.get("topic"),
        payload: serde_json::from_str(&raw_payload)?,
        status: MessageStatus::parse(&raw_status)?,
        user_id: r.get("user_id"),
        agent_id: r.get("agent_id"),
        conversation_id: raw_conv.map(|s| Uuid::parse_str(&s)).transpose()?,
        interface: r.get("interface"),
        reply_to: r.get("reply_to"),
        correlation_id: raw_corr.map(|s| Uuid::parse_str(&s)).transpose()?,
        causation_id: raw_cause.map(|s| Uuid::parse_str(&s)).transpose()?,
        batch_id: raw_batch.map(|s| Uuid::parse_str(&s)).transpose()?,
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

    // -- publish & list -----------------------------------------------------

    #[tokio::test]
    async fn test_publish_and_list() {
        let (_s, bus) = bus().await;
        let payload = serde_json::json!({"action": "greet"});

        let id = bus
            .publish(PublishRequest::new("test.topic", payload.clone()))
            .await
            .unwrap();

        let msgs = bus.list("test.topic", None, 10).await.unwrap();
        assert_eq!(msgs.len(), 1, "should have one message");
        assert_eq!(msgs[0].id, id);
        assert_eq!(msgs[0].topic, "test.topic");
        assert_eq!(msgs[0].status, MessageStatus::Pending);
        assert_eq!(msgs[0].payload, payload);
        assert!(msgs[0].conversation_id.is_none());
        assert!(msgs[0].agent_id.is_none());
    }

    #[tokio::test]
    async fn test_publish_with_full_metadata() {
        let (_s, bus) = bus().await;
        let conv_id = Uuid::new_v4();
        let corr_id = Uuid::new_v4();
        let cause_id = Uuid::new_v4();
        let batch_id = Uuid::new_v4();

        let id = bus
            .publish(
                PublishRequest::new("turn.request", serde_json::json!({"prompt": "hi"}))
                    .with_user_id("U123")
                    .with_agent_id("main")
                    .with_conversation_id(conv_id)
                    .with_interface("slack")
                    .with_reply_to("turn.result")
                    .with_correlation_id(corr_id)
                    .with_causation_id(cause_id)
                    .with_batch_id(batch_id),
            )
            .await
            .unwrap();

        let msgs = bus.list("turn.request", None, 10).await.unwrap();
        assert_eq!(msgs.len(), 1);
        let m = &msgs[0];
        assert_eq!(m.id, id);
        assert_eq!(m.user_id.as_deref(), Some("U123"));
        assert_eq!(m.agent_id.as_deref(), Some("main"));
        assert_eq!(m.conversation_id, Some(conv_id));
        assert_eq!(m.interface.as_deref(), Some("slack"));
        assert_eq!(m.reply_to.as_deref(), Some("turn.result"));
        assert_eq!(m.correlation_id, Some(corr_id));
        assert_eq!(m.causation_id, Some(cause_id));
        assert_eq!(m.batch_id, Some(batch_id));
    }

    // -- claim (unfiltered) -------------------------------------------------

    #[tokio::test]
    async fn test_claim_returns_oldest_first() {
        let (_s, bus) = bus().await;

        bus.publish(PublishRequest::new("q", serde_json::json!({"seq": 1})))
            .await
            .unwrap();
        bus.publish(PublishRequest::new("q", serde_json::json!({"seq": 2})))
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

        bus.publish(PublishRequest::new("q", serde_json::json!({"only": true})))
            .await
            .unwrap();

        bus.claim("q", "w1").await.unwrap().unwrap();

        let second = bus.claim("q", "w2").await.unwrap();
        assert!(
            second.is_none(),
            "already-claimed message should not be re-claimed"
        );
    }

    // -- claim_filtered -----------------------------------------------------

    #[tokio::test]
    async fn test_claim_filtered_by_agent() {
        let (_s, bus) = bus().await;

        bus.publish(
            PublishRequest::new("turn.request", serde_json::json!({"for": "alpha"}))
                .with_agent_id("alpha"),
        )
        .await
        .unwrap();
        bus.publish(
            PublishRequest::new("turn.request", serde_json::json!({"for": "beta"}))
                .with_agent_id("beta"),
        )
        .await
        .unwrap();

        let beta_msg = bus
            .claim_filtered(
                "turn.request",
                "w1",
                &ClaimFilter::new().with_agent_id("beta"),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(beta_msg.payload["for"], "beta");
        assert_eq!(beta_msg.agent_id.as_deref(), Some("beta"));

        // Alpha still available
        let alpha_msg = bus
            .claim_filtered(
                "turn.request",
                "w1",
                &ClaimFilter::new().with_agent_id("alpha"),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(alpha_msg.payload["for"], "alpha");
    }

    #[tokio::test]
    async fn test_claim_filtered_by_batch() {
        let (_s, bus) = bus().await;
        let batch_a = Uuid::new_v4();
        let batch_b = Uuid::new_v4();

        bus.publish(
            PublishRequest::new("tool.result", serde_json::json!({"tool": "bash"}))
                .with_batch_id(batch_a),
        )
        .await
        .unwrap();
        bus.publish(
            PublishRequest::new("tool.result", serde_json::json!({"tool": "web"}))
                .with_batch_id(batch_b),
        )
        .await
        .unwrap();

        let msg = bus
            .claim_filtered(
                "tool.result",
                "w1",
                &ClaimFilter::new().with_batch_id(batch_b),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg.payload["tool"], "web");
        assert_eq!(msg.batch_id, Some(batch_b));
    }

    #[tokio::test]
    async fn test_claim_filtered_by_conversation() {
        let (_s, bus) = bus().await;
        let conv_a = Uuid::new_v4();
        let conv_b = Uuid::new_v4();

        bus.publish(
            PublishRequest::new("turn.request", serde_json::json!({"conv": "a"}))
                .with_conversation_id(conv_a),
        )
        .await
        .unwrap();
        bus.publish(
            PublishRequest::new("turn.request", serde_json::json!({"conv": "b"}))
                .with_conversation_id(conv_b),
        )
        .await
        .unwrap();

        let msg = bus
            .claim_filtered(
                "turn.request",
                "w1",
                &ClaimFilter::new().with_conversation_id(conv_b),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg.payload["conv"], "b");
    }

    #[tokio::test]
    async fn test_claim_filtered_by_user() {
        let (_s, bus) = bus().await;

        bus.publish(
            PublishRequest::new("turn.request", serde_json::json!({"u": 1})).with_user_id("alice"),
        )
        .await
        .unwrap();
        bus.publish(
            PublishRequest::new("turn.request", serde_json::json!({"u": 2})).with_user_id("bob"),
        )
        .await
        .unwrap();

        let msg = bus
            .claim_filtered(
                "turn.request",
                "w1",
                &ClaimFilter::new().with_user_id("bob"),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg.payload["u"], 2);
        assert_eq!(msg.user_id.as_deref(), Some("bob"));
    }

    #[tokio::test]
    async fn test_claim_filtered_combined() {
        let (_s, bus) = bus().await;
        let conv = Uuid::new_v4();

        // Message for agent alpha, conv X
        bus.publish(
            PublishRequest::new("turn.request", serde_json::json!({"match": false}))
                .with_agent_id("alpha")
                .with_conversation_id(Uuid::new_v4()),
        )
        .await
        .unwrap();

        // Message for agent beta, conv X — the one we want
        bus.publish(
            PublishRequest::new("turn.request", serde_json::json!({"match": true}))
                .with_agent_id("beta")
                .with_conversation_id(conv),
        )
        .await
        .unwrap();

        let msg = bus
            .claim_filtered(
                "turn.request",
                "w1",
                &ClaimFilter::new()
                    .with_agent_id("beta")
                    .with_conversation_id(conv),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg.payload["match"], true);
    }

    #[tokio::test]
    async fn test_claim_filtered_returns_none_when_no_match() {
        let (_s, bus) = bus().await;

        bus.publish(PublishRequest::new("q", serde_json::json!({})).with_agent_id("alpha"))
            .await
            .unwrap();

        let result = bus
            .claim_filtered("q", "w1", &ClaimFilter::new().with_agent_id("beta"))
            .await
            .unwrap();
        assert!(result.is_none(), "should not match different agent");
    }

    // -- ack / nack / fail --------------------------------------------------

    #[tokio::test]
    async fn test_ack_sets_done() {
        let (_s, bus) = bus().await;

        let id = bus
            .publish(PublishRequest::new("q", serde_json::json!({})))
            .await
            .unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.ack(id).await.unwrap();

        let msgs = bus.list("q", Some(MessageStatus::Done), 10).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].id, id);
    }

    #[tokio::test]
    async fn test_nack_releases_for_retry() {
        let (_s, bus) = bus().await;

        let id = bus
            .publish(PublishRequest::new("q", serde_json::json!({})))
            .await
            .unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.nack(id).await.unwrap();

        let reclaimed = bus.claim("q", "w2").await.unwrap().unwrap();
        assert_eq!(reclaimed.id, id);
        assert_eq!(reclaimed.claimed_by.as_deref(), Some("w2"));
    }

    #[tokio::test]
    async fn test_fail_marks_permanent() {
        let (_s, bus) = bus().await;

        let id = bus
            .publish(PublishRequest::new("q", serde_json::json!({})))
            .await
            .unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.fail(id).await.unwrap();

        let msgs = bus
            .list("q", Some(MessageStatus::Failed), 10)
            .await
            .unwrap();
        assert_eq!(msgs.len(), 1);

        let next = bus.claim("q", "w2").await.unwrap();
        assert!(next.is_none(), "failed messages must not be claimable");
    }

    // -- list ---------------------------------------------------------------

    #[tokio::test]
    async fn test_list_filters_by_status() {
        let (_s, bus) = bus().await;

        bus.publish(PublishRequest::new("q", serde_json::json!({"a": 1})))
            .await
            .unwrap();
        let id2 = bus
            .publish(PublishRequest::new("q", serde_json::json!({"a": 2})))
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

        bus.publish(PublishRequest::new("topic.a", serde_json::json!({})))
            .await
            .unwrap();
        bus.publish(PublishRequest::new("topic.b", serde_json::json!({})))
            .await
            .unwrap();

        let a = bus.list("topic.a", None, 10).await.unwrap();
        assert_eq!(a.len(), 1);

        let b = bus.list("topic.b", None, 10).await.unwrap();
        assert_eq!(b.len(), 1);
    }

    // -- reap & purge -------------------------------------------------------

    #[tokio::test]
    async fn test_reap_stale_reclaims_old_messages() {
        let (_s, bus) = bus().await;

        let id = bus
            .publish(PublishRequest::new("q", serde_json::json!({})))
            .await
            .unwrap();
        bus.claim("q", "w1").await.unwrap();

        // Backdate claimed_at
        sqlx::query(
            "UPDATE bus_messages SET claimed_at = datetime('now', '-1 hour') WHERE id = ?1",
        )
        .bind(id.to_string())
        .execute(&bus.pool)
        .await
        .unwrap();

        let reaped = bus.reap_stale(Duration::from_secs(60)).await.unwrap();
        assert_eq!(reaped, 1, "should reap one stale message");

        let reclaimed = bus.claim("q", "w2").await.unwrap();
        assert!(reclaimed.is_some(), "reaped message should be claimable");
    }

    #[tokio::test]
    async fn test_reap_stale_ignores_fresh_claims() {
        let (_s, bus) = bus().await;

        bus.publish(PublishRequest::new("q", serde_json::json!({})))
            .await
            .unwrap();
        bus.claim("q", "w1").await.unwrap();

        let reaped = bus.reap_stale(Duration::from_secs(3600)).await.unwrap();
        assert_eq!(reaped, 0, "fresh claims should not be reaped");
    }

    #[tokio::test]
    async fn test_purge_removes_old_done_messages() {
        let (_s, bus) = bus().await;

        let id = bus
            .publish(PublishRequest::new("q", serde_json::json!({})))
            .await
            .unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.ack(id).await.unwrap();

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

        let id = bus
            .publish(PublishRequest::new("q", serde_json::json!({})))
            .await
            .unwrap();
        bus.claim("q", "w1").await.unwrap();
        bus.ack(id).await.unwrap();

        let cutoff = Utc::now() - chrono::Duration::days(1);
        let purged = bus.purge(cutoff).await.unwrap();
        assert_eq!(purged, 0, "recent done messages should not be purged");
    }
}
