//! NATS JetStream-backed [`MessageBus`] implementation.
//!
//! Replaces the SQLite-backed bus to eliminate write-lock contention when the
//! orchestrator, OTel exporters, conversation store, and bus all share a single
//! SQLite pool.
//!
//! # Design
//!
//! - One JetStream stream (`ASSISTANT_BUS`) captures all `bus.>` subjects.
//! - Metadata (user_id, agent_id, conversation_id, …) is stored in NATS
//!   headers so the payload remains a plain JSON blob — same as the SQLite bus.
//! - Durable pull consumers are created per `(worker_id, topic)` pair.
//! - [`ClaimFilter`] fields are checked in-process after pulling; NATS subject
//!   filtering handles the topic dimension.
//! - In-flight messages are tracked in a `HashMap` keyed by the [`Uuid`] we
//!   assign at publish time, so [`ack`]/[`nack`]/[`fail`] map cleanly to
//!   JetStream ack / nak / term.
//!
//! # Delivery semantics
//!
//! JetStream provides **at-least-once** delivery out of the box, matching the
//! trait contract.  `ack_wait` on consumers replaces the SQLite `reap_stale`
//! mechanism.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_nats::jetstream::consumer::pull;
use async_nats::jetstream::consumer::Consumer;
use async_nats::jetstream::stream::RetentionPolicy;
use async_nats::jetstream::{self, AckKind};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

use assistant_core::{BusMessage, ClaimFilter, MessageBus, MessageStatus, PublishRequest};

// -- Header keys ------------------------------------------------------------

const H_MESSAGE_ID: &str = "Bus-Message-Id";
const H_TOPIC: &str = "Bus-Topic";
const H_CREATED_AT: &str = "Bus-Created-At";
const H_USER_ID: &str = "Bus-User-Id";
const H_AGENT_ID: &str = "Bus-Agent-Id";
const H_CONVERSATION_ID: &str = "Bus-Conversation-Id";
const H_INTERFACE: &str = "Bus-Interface";
const H_REPLY_TO: &str = "Bus-Reply-To";
const H_CORRELATION_ID: &str = "Bus-Correlation-Id";
const H_CAUSATION_ID: &str = "Bus-Causation-Id";
const H_BATCH_ID: &str = "Bus-Batch-Id";

// -- Stream / consumer defaults ---------------------------------------------

const STREAM_NAME: &str = "ASSISTANT_BUS";
const STREAM_SUBJECTS: &str = "bus.>";
/// How long the server waits for an ack before redelivering.
const ACK_WAIT: Duration = Duration::from_secs(300);
/// Maximum number of messages fetched per `claim_filtered` call.
const FETCH_BATCH: usize = 10;
/// Delay before a nak'd message is redelivered.
const NAK_DELAY: Duration = Duration::from_millis(200);
/// Stream max age — auto-purge messages older than this.
const STREAM_MAX_AGE: Duration = Duration::from_secs(86_400); // 24 h

// -- NatsMessageBus ---------------------------------------------------------

/// NATS JetStream implementation of [`MessageBus`].
///
/// Create with [`NatsMessageBus::new`], passing a NATS server URL (e.g.
/// `nats://localhost:4222`).  The constructor connects, creates (or reuses)
/// the JetStream stream, and is ready for publishing / claiming immediately.
pub struct NatsMessageBus {
    js: jetstream::Context,
    stream: jetstream::stream::Stream,
    /// In-flight messages awaiting ack/nack/fail, keyed by our Bus-Message-Id.
    inflight: Arc<RwLock<HashMap<Uuid, async_nats::jetstream::Message>>>,
}

impl NatsMessageBus {
    /// Connect to NATS and ensure the JetStream stream exists.
    pub async fn new(url: &str) -> Result<Self> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| anyhow::anyhow!("failed to connect to NATS at {url}: {e}"))?;

        let js = jetstream::new(client);

        let stream = js
            .get_or_create_stream(jetstream::stream::Config {
                name: STREAM_NAME.to_string(),
                subjects: vec![STREAM_SUBJECTS.to_string()],
                retention: RetentionPolicy::Limits,
                max_age: STREAM_MAX_AGE,
                ..Default::default()
            })
            .await
            .map_err(|e| anyhow::anyhow!("failed to create/get JetStream stream: {e}"))?;

        debug!(stream = STREAM_NAME, "NATS message bus ready");

        Ok(Self {
            js,
            stream,
            inflight: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Build or retrieve a durable pull consumer for `(worker_id, topic)`.
    async fn consumer_for(&self, worker_id: &str, topic: &str) -> Result<Consumer<pull::Config>> {
        let name = format!("{}-{}", worker_id, topic.replace('.', "-"));

        let consumer = self
            .stream
            .get_or_create_consumer(
                &name,
                pull::Config {
                    durable_name: Some(name.clone()),
                    filter_subject: format!("bus.{topic}"),
                    ack_wait: ACK_WAIT,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!("failed to create NATS consumer {name}: {e}"))?;

        Ok(consumer)
    }
}

// -- MessageBus impl --------------------------------------------------------

#[async_trait]
impl MessageBus for NatsMessageBus {
    async fn publish(&self, req: PublishRequest) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let subject = format!("bus.{}", req.topic);

        // -- build headers --
        let mut headers = async_nats::HeaderMap::new();

        let id_s = id.to_string();
        headers.insert(H_MESSAGE_ID, id_s.as_str());
        headers.insert(H_TOPIC, req.topic.as_str());

        let now_s = now.to_rfc3339();
        headers.insert(H_CREATED_AT, now_s.as_str());

        if let Some(ref v) = req.user_id {
            headers.insert(H_USER_ID, v.as_str());
        }
        if let Some(ref v) = req.agent_id {
            headers.insert(H_AGENT_ID, v.as_str());
        }
        if let Some(ref v) = req.conversation_id {
            let s = v.to_string();
            headers.insert(H_CONVERSATION_ID, s.as_str());
        }
        if let Some(ref v) = req.interface {
            headers.insert(H_INTERFACE, v.as_str());
        }
        if let Some(ref v) = req.reply_to {
            headers.insert(H_REPLY_TO, v.as_str());
        }
        if let Some(ref v) = req.correlation_id {
            let s = v.to_string();
            headers.insert(H_CORRELATION_ID, s.as_str());
        }
        if let Some(ref v) = req.causation_id {
            let s = v.to_string();
            headers.insert(H_CAUSATION_ID, s.as_str());
        }
        if let Some(ref v) = req.batch_id {
            let s = v.to_string();
            headers.insert(H_BATCH_ID, s.as_str());
        }

        let payload = serde_json::to_vec(&req.payload)?;

        self.js
            .publish_with_headers(subject, headers, payload.into())
            .await
            .map_err(|e| anyhow::anyhow!("NATS publish send failed: {e}"))?
            .await
            .map_err(|e| anyhow::anyhow!("NATS publish ack failed: {e}"))?;

        debug!(id = %id, topic = %req.topic, "published bus message via NATS");
        Ok(id)
    }

    async fn claim_filtered(
        &self,
        topic: &str,
        worker_id: &str,
        filter: &ClaimFilter,
    ) -> Result<Option<BusMessage>> {
        let consumer: Consumer<pull::Config> = self.consumer_for(worker_id, topic).await?;

        let mut batch = consumer
            .fetch()
            .max_messages(FETCH_BATCH)
            .messages()
            .await
            .map_err(|e| anyhow::anyhow!("NATS fetch failed: {e}"))?;

        while let Some(msg_result) = batch.next().await {
            let msg: async_nats::jetstream::Message =
                msg_result.map_err(|e| anyhow::anyhow!("NATS message receive error: {e}"))?;

            let headers = msg.headers.as_ref();
            if !matches_filter(headers, filter) {
                // Not for us — return to the stream after a short delay so
                // the intended consumer can pick it up.
                if let Err(e) = msg.ack_with(AckKind::Nak(Some(NAK_DELAY))).await {
                    warn!(error = %e, "NATS nak failed");
                }
                continue;
            }

            let bus_msg = parse_nats_message(&msg, worker_id)?;
            debug!(
                id = %bus_msg.id,
                topic = %topic,
                worker = %worker_id,
                "claimed bus message via NATS"
            );

            // Stash the raw NATS message so ack/nack/fail can operate on it.
            self.inflight.write().await.insert(bus_msg.id, msg);

            return Ok(Some(bus_msg));
        }

        Ok(None)
    }

    async fn ack(&self, message_id: Uuid) -> Result<()> {
        let msg = self
            .inflight
            .write()
            .await
            .remove(&message_id)
            .ok_or_else(|| anyhow::anyhow!("no inflight NATS message for id {message_id}"))?;

        msg.ack()
            .await
            .map_err(|e| anyhow::anyhow!("NATS ack failed: {e}"))?;

        debug!(id = %message_id, "acked bus message via NATS");
        Ok(())
    }

    async fn nack(&self, message_id: Uuid) -> Result<()> {
        let msg = self
            .inflight
            .write()
            .await
            .remove(&message_id)
            .ok_or_else(|| anyhow::anyhow!("no inflight NATS message for id {message_id}"))?;

        msg.ack_with(AckKind::Nak(None))
            .await
            .map_err(|e| anyhow::anyhow!("NATS nak failed: {e}"))?;

        debug!(id = %message_id, "nacked bus message via NATS");
        Ok(())
    }

    async fn fail(&self, message_id: Uuid) -> Result<()> {
        let msg = self
            .inflight
            .write()
            .await
            .remove(&message_id)
            .ok_or_else(|| anyhow::anyhow!("no inflight NATS message for id {message_id}"))?;

        msg.ack_with(AckKind::Term)
            .await
            .map_err(|e| anyhow::anyhow!("NATS term failed: {e}"))?;

        debug!(id = %message_id, "failed bus message via NATS (terminated)");
        Ok(())
    }

    /// **NATS behavioural note:** Only returns in-flight (claimed) messages
    /// from this process's memory.  Unlike the SQLite bus, this does *not*
    /// query all persisted messages.  Use the NATS dashboard / CLI for full
    /// stream observability.
    async fn list(
        &self,
        topic: &str,
        status: Option<MessageStatus>,
        _limit: u32,
    ) -> Result<Vec<BusMessage>> {
        match status {
            Some(MessageStatus::Claimed) | None => {
                let inflight = self.inflight.read().await;
                let result: Vec<BusMessage> = inflight
                    .values()
                    .filter_map(|msg| parse_nats_message(msg, "list").ok())
                    .filter(|m| m.topic == topic)
                    .collect();
                debug!(
                    topic = %topic,
                    count = result.len(),
                    "NATS list: returning in-flight messages only (no persisted query)"
                );
                Ok(result)
            }
            _ => {
                debug!(
                    topic = %topic,
                    status = ?status,
                    "NATS list: status filter not supported, returning empty"
                );
                Ok(vec![])
            }
        }
    }

    /// **NATS behavioural note:** No-op — JetStream automatically redelivers
    /// messages whose `ack_wait` expires, replacing manual stale reaping.
    async fn reap_stale(&self, _timeout: Duration) -> Result<u64> {
        debug!("NATS reap_stale: no-op (JetStream ack_wait handles redelivery)");
        Ok(0)
    }

    /// **NATS behavioural note:** No-op — the JetStream stream's `MaxAge`
    /// setting handles time-based purging automatically.
    async fn purge(&self, _older_than: DateTime<Utc>) -> Result<u64> {
        debug!("NATS purge: no-op (JetStream MaxAge handles expiration)");
        Ok(0)
    }
}

// -- Helpers ----------------------------------------------------------------

/// Check whether a NATS message's headers satisfy the [`ClaimFilter`].
fn matches_filter(headers: Option<&async_nats::HeaderMap>, filter: &ClaimFilter) -> bool {
    if filter.is_empty() {
        return true;
    }

    let Some(h) = headers else {
        // No headers at all — can only match an empty filter (handled above).
        return false;
    };

    if let Some(ref want) = filter.agent_id {
        if header_str(h, H_AGENT_ID) != Some(want.as_str()) {
            return false;
        }
    }
    if let Some(ref want) = filter.user_id {
        if header_str(h, H_USER_ID) != Some(want.as_str()) {
            return false;
        }
    }
    if let Some(ref want) = filter.conversation_id {
        let want_s = want.to_string();
        if header_str(h, H_CONVERSATION_ID) != Some(want_s.as_str()) {
            return false;
        }
    }
    if let Some(ref want) = filter.batch_id {
        let want_s = want.to_string();
        if header_str(h, H_BATCH_ID) != Some(want_s.as_str()) {
            return false;
        }
    }
    if let Some(ref want) = filter.interface {
        if header_str(h, H_INTERFACE) != Some(want.as_str()) {
            return false;
        }
    }

    true
}

/// Extract a single header value as `&str`.
fn header_str<'a>(h: &'a async_nats::HeaderMap, key: &str) -> Option<&'a str> {
    h.get(key).map(|v| v.as_str())
}

/// Reconstruct a [`BusMessage`] from a NATS JetStream message.
fn parse_nats_message(msg: &async_nats::jetstream::Message, worker_id: &str) -> Result<BusMessage> {
    let headers = msg
        .headers
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("NATS message missing headers"))?;

    let id_str = header_str(headers, H_MESSAGE_ID)
        .ok_or_else(|| anyhow::anyhow!("missing Bus-Message-Id header"))?;
    let id = Uuid::parse_str(id_str)?;

    let topic = header_str(headers, H_TOPIC)
        .ok_or_else(|| anyhow::anyhow!("missing Bus-Topic header"))?
        .to_string();

    let payload: Value = serde_json::from_slice(&msg.payload)?;

    let created_at = header_str(headers, H_CREATED_AT)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    let conversation_id =
        header_str(headers, H_CONVERSATION_ID).and_then(|s| Uuid::parse_str(s).ok());

    let correlation_id =
        header_str(headers, H_CORRELATION_ID).and_then(|s| Uuid::parse_str(s).ok());

    let causation_id = header_str(headers, H_CAUSATION_ID).and_then(|s| Uuid::parse_str(s).ok());

    let batch_id = header_str(headers, H_BATCH_ID).and_then(|s| Uuid::parse_str(s).ok());

    Ok(BusMessage {
        id,
        topic,
        payload,
        status: MessageStatus::Claimed,
        user_id: header_str(headers, H_USER_ID).map(String::from),
        agent_id: header_str(headers, H_AGENT_ID).map(String::from),
        conversation_id,
        interface: header_str(headers, H_INTERFACE).map(String::from),
        reply_to: header_str(headers, H_REPLY_TO).map(String::from),
        correlation_id,
        causation_id,
        batch_id,
        created_at,
        claimed_at: Some(Utc::now()),
        claimed_by: Some(worker_id.to_string()),
    })
}

// -- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Unit tests (no Docker) ---------------------------------------------

    #[test]
    fn test_matches_filter_empty() {
        assert!(matches_filter(None, &ClaimFilter::default()));
    }

    #[test]
    fn test_matches_filter_with_agent_id() {
        let mut h = async_nats::HeaderMap::new();
        h.insert(H_AGENT_ID, "alpha");

        let filter = ClaimFilter::new().with_agent_id("alpha");
        assert!(matches_filter(Some(&h), &filter));

        let filter = ClaimFilter::new().with_agent_id("beta");
        assert!(!matches_filter(Some(&h), &filter));
    }

    #[test]
    fn test_matches_filter_with_batch_id() {
        let batch = Uuid::new_v4();
        let mut h = async_nats::HeaderMap::new();
        h.insert(H_BATCH_ID, batch.to_string().as_str());

        let filter = ClaimFilter::new().with_batch_id(batch);
        assert!(matches_filter(Some(&h), &filter));

        let filter = ClaimFilter::new().with_batch_id(Uuid::new_v4());
        assert!(!matches_filter(Some(&h), &filter));
    }

    #[test]
    fn test_matches_filter_combined() {
        let conv = Uuid::new_v4();
        let mut h = async_nats::HeaderMap::new();
        h.insert(H_AGENT_ID, "main");
        h.insert(H_INTERFACE, "Slack");
        h.insert(H_CONVERSATION_ID, conv.to_string().as_str());

        let filter = ClaimFilter::new()
            .with_agent_id("main")
            .with_interface("Slack")
            .with_conversation_id(conv);
        assert!(matches_filter(Some(&h), &filter));

        // Mismatch on one field → reject
        let filter = ClaimFilter::new()
            .with_agent_id("main")
            .with_interface("Web");
        assert!(!matches_filter(Some(&h), &filter));
    }

    #[test]
    fn test_matches_filter_no_headers_non_empty_filter() {
        let filter = ClaimFilter::new().with_agent_id("x");
        assert!(!matches_filter(None, &filter));
    }

    // -- Integration tests (require Docker) ---------------------------------

    use testcontainers::core::IntoContainerPort;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::{ContainerAsync, ImageExt};
    use testcontainers_modules::nats::{Nats, NatsServerCmd};

    const NATS_PORT: u16 = 4222;

    /// Start a NATS container with JetStream enabled, return container handle
    /// and the connection URL.
    async fn start_nats() -> (ContainerAsync<Nats>, String) {
        let cmd = NatsServerCmd::default().with_jetstream();
        let container = Nats::default()
            .with_cmd(&cmd)
            .start()
            .await
            .expect("failed to start NATS container");

        let host = container.get_host().await.expect("no container host");
        let port = container
            .get_host_port_ipv4(NATS_PORT.tcp())
            .await
            .expect("no mapped port");
        let url = format!("nats://{host}:{port}");

        (container, url)
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_publish_and_claim() {
        let (_container, url) = start_nats().await;
        let bus = NatsMessageBus::new(&url).await.unwrap();

        let payload = serde_json::json!({"action": "greet"});
        let _id = bus
            .publish(PublishRequest::new("test.topic", payload.clone()))
            .await
            .unwrap();

        let msg = bus
            .claim("test.topic", "w1")
            .await
            .unwrap()
            .expect("should claim message");

        assert_eq!(msg.payload, payload);
        assert_eq!(msg.topic, "test.topic");
        assert_eq!(msg.status, MessageStatus::Claimed);
        assert_eq!(msg.claimed_by.as_deref(), Some("w1"));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_publish_with_metadata() {
        let (_container, url) = start_nats().await;
        let bus = NatsMessageBus::new(&url).await.unwrap();

        let conv_id = Uuid::new_v4();
        let batch_id = Uuid::new_v4();

        bus.publish(
            PublishRequest::new("turn.request", serde_json::json!({"prompt": "hi"}))
                .with_user_id("U123")
                .with_agent_id("main")
                .with_conversation_id(conv_id)
                .with_interface("slack")
                .with_batch_id(batch_id),
        )
        .await
        .unwrap();

        let msg = bus
            .claim("turn.request", "w1")
            .await
            .unwrap()
            .expect("should claim message");

        assert_eq!(msg.user_id.as_deref(), Some("U123"));
        assert_eq!(msg.agent_id.as_deref(), Some("main"));
        assert_eq!(msg.conversation_id, Some(conv_id));
        assert_eq!(msg.interface.as_deref(), Some("slack"));
        assert_eq!(msg.batch_id, Some(batch_id));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_claim_filtered_by_batch() {
        let (_container, url) = start_nats().await;
        let bus = NatsMessageBus::new(&url).await.unwrap();

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
            .expect("should claim batch_b message");

        assert_eq!(msg.payload["tool"], "web");
        assert_eq!(msg.batch_id, Some(batch_b));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_ack_nack_fail() {
        let (_container, url) = start_nats().await;
        let bus = NatsMessageBus::new(&url).await.unwrap();

        // Publish three messages
        bus.publish(PublishRequest::new("q", serde_json::json!({"n": 1})))
            .await
            .unwrap();
        bus.publish(PublishRequest::new("q", serde_json::json!({"n": 2})))
            .await
            .unwrap();
        bus.publish(PublishRequest::new("q", serde_json::json!({"n": 3})))
            .await
            .unwrap();

        // Claim and ack the first
        let m1 = bus.claim("q", "w1").await.unwrap().unwrap();
        bus.ack(m1.id).await.unwrap();

        // Claim and fail the second
        let m2 = bus.claim("q", "w1").await.unwrap().unwrap();
        bus.fail(m2.id).await.unwrap();

        // Claim and nack the third (returns to stream)
        let m3 = bus.claim("q", "w1").await.unwrap().unwrap();
        bus.nack(m3.id).await.unwrap();

        // After a short delay the nack'd message should be reclaimable
        tokio::time::sleep(Duration::from_millis(500)).await;
        let reclaimed = bus.claim("q", "w1").await.unwrap();
        assert!(reclaimed.is_some(), "nacked message should be reclaimable");
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn test_claim_returns_none_when_empty() {
        let (_container, url) = start_nats().await;
        let bus = NatsMessageBus::new(&url).await.unwrap();

        let result = bus.claim("empty.topic", "w1").await.unwrap();
        assert!(result.is_none(), "should return None on empty topic");
    }
}
